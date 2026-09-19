import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider, useQuery } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ExecutionBoardRefresh } from "./native-subtasks";
import { agentTaskSnapshotOptions } from "@multica/core/agents/queries";
import { api } from "@multica/core/api";
vi.mock("@multica/core/api", () => ({ api: { getAgentTaskSnapshot: vi.fn() } }));
const status = vi.hoisted(() => ({ snapshot: { stale: false, diagnostic: null, updatedAt: null } as { stale: boolean; diagnostic: string | null; updatedAt: number | null }, listeners: new Set<() => void>() }));
vi.mock("./multica-execution-board", () => ({ getExecutionBoardStatus: () => status.snapshot, subscribeExecutionBoardStatus: (fn: () => void) => { status.listeners.add(fn); return () => status.listeners.delete(fn); } }));
let client: QueryClient;
const readIssues = vi.fn(), readWorking = vi.fn(), readOther = vi.fn();
function Queries() {
  const issues = useQuery({ queryKey: ["issues", "ws", "my"], queryFn: readIssues });
  const working = useQuery({ queryKey: ["workspaces", "ws", "working-agents"], queryFn: readWorking });
  useQuery({ queryKey: ["agents", "ws"], queryFn: readOther });
  const snapshot = useQuery(agentTaskSnapshotOptions("ws"));
  return <><ExecutionBoardRefresh workspaceId="ws" /><p>{issues.data}</p><p>{working.data}</p><p>{snapshot.data?.[0]?.status}</p></>;
}
const mount = (container?: HTMLElement) => render(<QueryClientProvider client={client}><Queries /></QueryClientProvider>, { container });
beforeEach(() => {
  vi.useFakeTimers();
  client = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity, gcTime: Infinity } } });
  readIssues.mockReset().mockResolvedValue("Old card"); readWorking.mockReset().mockResolvedValue("1 working"); readOther.mockReset().mockResolvedValue([]);
  status.snapshot = { stale: false, diagnostic: null, updatedAt: null };
  vi.mocked(api.getAgentTaskSnapshot).mockReset().mockResolvedValue([]);
});
afterEach(() => { cleanup(); client.clear(); vi.useRealTimers(); vi.restoreAllMocks(); });
const tick = async (ms = 20) => act(async () => { await vi.advanceTimersByTimeAsync(ms); });
describe("unified Issue query refresh", () => {
  it("refreshes Issues and working count together at 5s; stops after leaving", async () => {
    const view = mount(); await tick();
    expect(readIssues).toHaveBeenCalledTimes(1);
    await tick(4990); expect(readIssues).toHaveBeenCalledTimes(2); expect(readWorking).toHaveBeenCalledTimes(2);
    expect(readOther).toHaveBeenCalledTimes(1);
    expect(api.getAgentTaskSnapshot).toHaveBeenCalledTimes(2);
    view.unmount(); await tick(15000); expect(readIssues).toHaveBeenCalledTimes(2);
  });
  it("pauses both document and shadow host hidden polling and refreshes on return", async () => {
    const host = document.createElement("div"); document.body.append(host);
    const container = document.createElement("div"); host.attachShadow({ mode: "open" }).append(container);
    const view = mount(container);
    try {
      await tick(); await act(async () => { host.style.display = "none"; }); await tick(15000);
      expect(readIssues).toHaveBeenCalledTimes(1); expect(readWorking).toHaveBeenCalledTimes(1);
      expect(api.getAgentTaskSnapshot).toHaveBeenCalledTimes(1);
      await act(async () => { host.style.display = ""; }); await tick(); expect(readIssues).toHaveBeenCalledTimes(2);
      const visibility = vi.spyOn(document, "visibilityState", "get").mockReturnValue("hidden");
      fireEvent(document, new Event("visibilitychange")); await tick(15000); expect(readIssues).toHaveBeenCalledTimes(2);
      visibility.mockReturnValue("visible"); fireEvent(document, new Event("visibilitychange")); await tick(); expect(readIssues).toHaveBeenCalledTimes(3);
    } finally { view.unmount(); host.remove(); }
  });
  it("retains cached cards on query failure with visible stale and retry recovery", async () => {
    mount(); await tick(); readIssues.mockRejectedValue(new Error("private error")); await tick(5000);
    expect(screen.getByText("Old card")).toBeTruthy(); expect(screen.getByRole("alert").textContent).toContain("已过期");
    expect(screen.queryByText(/private error/)).toBeNull();
    readIssues.mockResolvedValue("New card"); fireEvent.click(screen.getByRole("button", { name: "重试" })); await tick();
    expect(screen.getByText("New card")).toBeTruthy(); expect(screen.queryByRole("alert")).toBeNull();
  });
  it("surfaces adapter partial/cached stale even when queries succeeded", async () => {
    mount(); await tick();
    act(() => { status.snapshot = { stale: true, diagnostic: "private database path", updatedAt: null }; status.listeners.forEach(fn => fn()); });
    expect(screen.getByRole("alert").textContent).toContain("已过期"); expect(screen.getByText("Old card")).toBeTruthy();
    expect(screen.queryByText(/private database/)).toBeNull();
    act(() => { status.snapshot = { stale: false, diagnostic: null, updatedAt: 100 }; status.listeners.forEach(fn => fn()); });
    expect(screen.queryByRole("alert")).toBeNull();
  });
});
