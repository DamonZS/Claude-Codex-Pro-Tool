import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "@multica/core/api";
import type { Issue } from "@multica/core/types";
import { ExecutionDetail } from "./native-subtasks";
import { submitNativeExecutionIntent } from "./native-execution-actions";
vi.mock("@multica/core/api", () => ({ api: { getIssue: vi.fn() } }));

const issue: Issue = { id: "codex-native:child", title: "Descartes", updated_at: "2026-09-19T00:00:00Z",
  workspace_id: "workspace", number: 1, identifier: "NATIVE-1", description: null, status: "in_progress", priority: "none",
  assignee_type: "agent", assignee_id: "native-agent", creator_type: "member", creator_id: "user", parent_issue_id: null,
  project_id: null, position: 0, stage: null, start_date: null, due_date: null, properties: {}, created_at: "2026-09-19T00:00:00Z",
  metadata: { ccp_read_only: true, ccp_source: "codex-native", ccp_execution_state: "inProgress",
    ccp_thread_id: "child", ccp_parent_thread_id: "parent", ccp_agent_name: "Descartes" },
};
let client: QueryClient;
const openThread = vi.fn();
beforeEach(() => {
  client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity } } });
  vi.spyOn(api, "getIssue").mockResolvedValue(issue);
  openThread.mockReset().mockResolvedValue(true);
  window.__CODEX_WORKFLOW_BRIDGE__ = { postJson: vi.fn(), openThread };
});
afterEach(() => { cleanup(); client.clear(); delete window.__CODEX_WORKFLOW_BRIDGE__; vi.restoreAllMocks(); });
const mount = (id = issue.id, onOpenThread?: (id: string) => Promise<unknown>) => render(
  <QueryClientProvider client={client}><ExecutionDetail id={id} workspaceId="workspace" onOpenThread={onOpenThread} leadingAction={<button>返回我的任务</button>} /></QueryClientProvider>,
);

describe("unified execution readonly detail", () => {
  it("shows pending and failed drag intent in detail and retries without opening either session", async () => {
    const failed = { ...issue, id: "codex-native:detail-retry", metadata: { ...issue.metadata, ccp_thread_id: "detail-retry", ccp_execution_state: "failed" } };
    vi.mocked(api.getIssue).mockResolvedValue(failed);
    let finish!: (value: unknown) => void;
    const postJson = vi.fn().mockImplementation(() => new Promise(resolve => { finish = resolve; }));
    window.__CODEX_WORKFLOW_BRIDGE__ = { postJson, openThread };
    mount(failed.id);
    await screen.findByRole("heading", { name: "Descartes" });
    let pending!: Promise<void>;
    act(() => { pending = submitNativeExecutionIntent(failed, "in_progress"); });
    expect(screen.getByRole("status").textContent).toContain("正在提交执行指令");
    await act(async () => { finish({ status: "failed" }); await pending; });
    expect(screen.getByRole("alert").textContent).toContain("执行指令提交失败");
    vi.mocked(api.getIssue).mockResolvedValue({ ...failed, updated_at: "2026-09-19T01:00:00Z", metadata: { ...failed.metadata, ccp_execution_state: "reconciling" } });
    await act(async () => { await client.invalidateQueries(); });
    await screen.findByText(/状态待确认/);
    expect(screen.getByRole("alert").textContent).toContain("执行指令提交失败");
    fireEvent.click(screen.getByRole("button", { name: "重试执行指令" }));
    expect(postJson).toHaveBeenCalledTimes(2);
    expect(postJson.mock.calls[0]).toEqual(postJson.mock.calls[1]);
    await act(async () => { finish({ status: "failed", code: "native_dispatch_outcome_pending" }); });
    expect(screen.getByRole("alert").textContent).toContain("执行指令提交失败");
    vi.mocked(api.getIssue).mockResolvedValue({ ...failed, updated_at: "2026-09-19T02:00:00Z", metadata: { ...failed.metadata, ccp_execution_state: "reconciling" } });
    await act(async () => { await client.invalidateQueries(); });
    fireEvent.click(screen.getByRole("button", { name: "重试执行指令" }));
    await waitFor(() => expect(postJson).toHaveBeenCalledTimes(3));
    expect(postJson.mock.calls[2]).toEqual(postJson.mock.calls[0]);
    await act(async () => { finish({ status: "ok" }); });
    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.getByRole("button", { name: "返回我的任务" })).toBeTruthy();
    expect(openThread).not.toHaveBeenCalled();
  });
  it("opens child and parent from the same Issue projection without an editor", async () => {
    mount();
    expect(await screen.findByRole("heading", { name: "Descartes" })).toBeTruthy();
    expect(screen.getByText(/Codex 原生/)).toBeTruthy();
    expect(screen.getByText(/执行中/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "打开子会话" }));
    expect(openThread).toHaveBeenLastCalledWith("child");
    fireEvent.click(screen.getByRole("button", { name: "打开父会话" }));
    expect(openThread).toHaveBeenLastCalledWith("parent");
    expect(screen.queryByRole("textbox")).toBeNull();
    expect(api.getIssue).toHaveBeenCalledWith("codex-native:child");
  });
  it("prefers the mount host callback", async () => {
    const custom = vi.fn().mockResolvedValue(true);
    mount(issue.id, custom);
    fireEvent.click(await screen.findByRole("button", { name: "打开子会话" }));
    expect(custom).toHaveBeenCalledWith("child");
    expect(openThread).not.toHaveBeenCalled();
  });
  it.each([false, { status: "failed" }, { status: "error" }, { ok: false }])("keeps detail and retry after open failure %j", async (result) => {
    openThread.mockResolvedValue(result);
    mount();
    fireEvent.click(await screen.findByRole("button", { name: "打开子会话" }));
    expect(await screen.findByRole("alert")).toHaveProperty("textContent", "会话打开失败，请重试。");
    openThread.mockResolvedValue(true);
    fireEvent.click(screen.getByRole("button", { name: "打开子会话" }));
    await act(async () => {});
    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.getByRole("button", { name: "返回我的任务" })).toBeTruthy();
  });
  it("retains return during loading and retries query failure", async () => {
    vi.mocked(api.getIssue).mockRejectedValueOnce(new Error("private error"));
    mount();
    expect(screen.getByRole("button", { name: "返回我的任务" })).toBeTruthy();
    expect(await screen.findByRole("alert")).toHaveProperty("textContent", "执行详情读取失败，请重试。");
    fireEvent.click(screen.getByRole("button", { name: "重试" }));
    expect(await screen.findByRole("heading", { name: "Descartes" })).toBeTruthy();
  });
  it("shows execution-only pending work with no fabricated thread actions", async () => {
    vi.mocked(api.getIssue).mockResolvedValue({ ...issue, id: "ccp-execution:binding", metadata: {
      ccp_read_only: true, ccp_source: "multica-execution", ccp_execution_state: "binding_pending", ccp_agent_name: "Worker",
    } });
    mount("ccp-execution:binding");
    expect(await screen.findByText(/等待执行/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "打开子会话" })).toBeNull();
    expect(screen.queryByRole("button", { name: "打开父会话" })).toBeNull();
  });
  it("renders unknown state as unconfirmed instead of failure", async () => {
    vi.mocked(api.getIssue).mockResolvedValue({ ...issue, metadata: { ...issue.metadata, ccp_execution_state: "future-state" } });
    mount();
    expect(await screen.findByText(/状态待确认/)).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
  });
});
