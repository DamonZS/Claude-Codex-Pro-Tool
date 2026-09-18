import { afterEach, describe, expect, it, vi } from "vitest";
import { blockedSocket, connectWorkflowHost, disconnectWorkflowHost, workflowFetch, workflowPortal } from "./upstream-host";
import { MulticaApiAdapter } from "./multica-api-adapter";

afterEach(disconnectWorkflowHost);
describe("local workflow boundary", () => {
  it("has no network fallback before connection or after disposal", async () => {
    const fetch = vi.spyOn(globalThis, "fetch");
    expect((await workflowFetch("/api/me")).status).toBe(503);
    expect(fetch).not.toHaveBeenCalled();
    fetch.mockRestore();
  });
  it("forwards local API data but strips ambient headers and cookies", async () => {
    const transport = vi.fn().mockResolvedValue(new Response("{}"));
    const container = document.createElement("div");
    connectWorkflowHost(transport, container);
    await workflowFetch("/api/issues", { method: "POST", body: "{}", credentials: "include", headers: { Authorization: "fixture" } });
    expect(transport).toHaveBeenCalledWith("/api/issues", { method: "POST", body: "{}", signal: undefined, headers: new Headers() });
    expect(workflowPortal()).toBe(container);
    for (const path of ["https://remote.invalid/api/me", "//remote.invalid/api/me", "/auth/login"]) {
      expect((await workflowFetch(path)).status).toBe(503);
    }
    expect(transport).toHaveBeenCalledTimes(1);
    expect(blockedSocket).toThrow("capability_unavailable");
    disconnectWorkflowHost();
    expect(() => workflowPortal()).toThrow();
  });
  it("preserves allowlisted headers and request identity for adapter retries", async () => {
    const receipts = new Map<string, { signature: unknown; result: unknown }>();
    const postJson = vi.fn(async (path: string, payload: Record<string, unknown>) => {
      if (path.endsWith("/bootstrap")) return { status: "ok", workspace: { id: "workspace", slug: "local", name: "Local" }, user: { id: "user" } };
      if (path === "/multica/workspace/command") {
        const receipt = receipts.get(String(payload.commandId));
        if (receipt && receipt.signature !== payload.commandSignature) return { status: "failed", code: "multica_workspace_idempotency_conflict" };
        return { status: "ok", found: !!receipt, result: receipt ? structuredClone(receipt.result) : null };
      }
      if (path.endsWith("/upsert")) {
        const result = { status: "ok", entity: { ...(payload.entity as object), revision: 1 } };
        receipts.set(String(payload.commandId), { signature: payload.commandSignature, result: structuredClone(result) });
        return result;
      }
      return { status: "ok", items: [], total: 0 };
    });
    const adapter = new MulticaApiAdapter({ postJson });
    const transport = vi.fn((path: string, init: RequestInit) => adapter.transport(path, init));
    connectWorkflowHost(transport, document.createElement("div"));
    const init: RequestInit = { method: "POST", body: JSON.stringify({ title: "Retry fixture" }), headers: { "X-Workspace-Slug": "local", "Content-Type": "application/json", "X-Request-Id": "request-fixture", Authorization: "fixture", Cookie: "fixture" } };
    const results = await Promise.all(Array.from({ length: 3 }, () => workflowFetch("/api/issues", init)));
    expect(results.every((result) => result.ok)).toBe(true);
    expect(postJson.mock.calls.filter(([path]) => path.endsWith("/upsert"))).toHaveLength(1);
    expect(transport.mock.calls[1][1]).toBe(transport.mock.calls[0][1]);
    const headers = new Headers(transport.mock.calls[0][1].headers);
    expect(Object.fromEntries(headers)).toEqual({ "content-type": "application/json", "x-request-id": "request-fixture", "x-workspace-slug": "local" });
    const otherWorkspace = await workflowFetch("/api/issues", { ...init, headers: { "X-Workspace-Slug": "other" } });
    expect(otherWorkspace.status).toBe(403);
    const keyed = { ...init, headers: { "Idempotency-Key": "explicit-fixture" } };
    await workflowFetch("/api/issues", keyed);
    expect(new Headers(transport.mock.calls.at(-1)![1].headers).get("idempotency-key")).toBe("explicit-fixture");
  });
});
