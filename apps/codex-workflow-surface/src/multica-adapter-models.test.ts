// @vitest-environment node
import { afterEach, describe, expect, it, vi } from "vitest";
import { ApiClient, setApiInstance } from "../vendor/multica/packages/core/api";
import { resolveRuntimeModels } from "../vendor/multica/packages/core/runtimes/models";
import * as host from "./upstream-host";

afterEach(() => vi.restoreAllMocks());

describe("shared model discovery with native selector authority", () => {
  it("returns runtime-managed configuration without issuing a daemon request", async () => {
    setApiInstance(new ApiClient(""));
    const transport = vi.spyOn(host, "workflowFetch").mockResolvedValue(Response.json([
      { id: "native", metadata: { nativeModelSelectionAuthoritative: true } },
    ]));
    expect(await resolveRuntimeModels("native")).toEqual({ models: [], unavailableModels: [], supported: false, cached: false });
    expect(transport).toHaveBeenCalledTimes(1);
    expect(transport.mock.calls[0][0]).toBe("/api/runtimes?");
  });
  it.each([undefined, false, "true"])("preserves other runtime discovery for authority=%j", async (authority) => {
    setApiInstance(new ApiClient(""));
    const transport = vi.spyOn(host, "workflowFetch").mockImplementation(async (path) => {
      if (String(path) === "/api/runtimes?") return Response.json([{ id: "daemon", metadata: { nativeModelSelectionAuthoritative: authority } }]);
      return Response.json({ id: "discovery", runtime_id: "daemon", status: "completed", models: [{ id: "actual-model", label: "Actual model" }], supported: true, created_at: "", updated_at: "" });
    });
    expect(await resolveRuntimeModels("daemon")).toMatchObject({ supported: true, models: [{ id: "actual-model" }] });
    expect(transport.mock.calls.map(([path]) => path)).toEqual(["/api/runtimes?", "/api/runtimes/daemon/models"]);
  });
});
