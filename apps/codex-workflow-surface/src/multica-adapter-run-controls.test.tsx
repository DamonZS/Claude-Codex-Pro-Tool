import { act, fireEvent, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MulticaApiAdapter } from "./multica-api-adapter";
import { type JsonRecord } from "./multica-adapter-dto";
import { ApiClient } from "../vendor/multica/packages/core/api/client";
import { useModalStore } from "@multica/core/modals";
import * as workflowHost from "./upstream-host";
import { runtime } from "./main";

const workspace = { id: "run-control-workspace", slug: "run-control", name: "Run controls" };
const date = "2026-09-18T00:00:00.000Z";
const init = (body: JsonRecord, key = "run-control"): RequestInit => ({ method: "PUT", body: JSON.stringify(body), headers: { "Idempotency-Key": key } });

function fixture(nativeSupported = true) {
  const collections: Record<string, JsonRecord[]> = {
    issues: ["issue-1", "issue-2"].map((id) => ({ id, workspace_id: workspace.id, title: id, status: "todo", revision: 1, creator_type: "member", creator_id: "user-1", assignee_type: "member", assignee_id: "user-1", created_at: date, updated_at: date })),
    agents: [{ id: "agent-1", name: "Native agent", runtime_id: "native-runtime", revision: 1, owner_id: "user-1", permission_mode: "private" }],
    runtimes: [{ id: "native-runtime", kind: "codex_page_host", provider: "codex", status: "available", native_task_host_supported: nativeSupported }],
  };
  const receipts = new Map<string, { signature: unknown; result: JsonRecord }>();
  const postJson = vi.fn(async (path: string, payload: JsonRecord): Promise<unknown> => {
    if (path === "/multica/workspace/bootstrap") return { status: "ok", workspace, user: { id: "user-1", kind: "local_control_plane" }, runtime: { available: true, runtimeId: "native-runtime", nativeTaskHostSupported: nativeSupported } };
    if (path === "/multica/workspace/query") {
      const items = collections[String(payload.resource)] ?? [];
      return { status: "ok", items, total: items.length };
    }
    if (path === "/multica/workspace/command") {
      const receipt = receipts.get(String(payload.commandId));
      if (receipt && receipt.signature !== payload.commandSignature) return { status: "failed", code: "idempotency_conflict" };
      return { status: "ok", found: !!receipt, result: receipt?.result };
    }
    if (path === "/multica/workspace/upsert") {
      const entity = payload.entity as JsonRecord;
      const previous = collections.issues.find((item) => item.id === entity.id)!;
      if (previous.revision !== payload.expectedRevision) return { status: "failed", code: "revision_conflict" };
      const saved = { ...entity, revision: Number(previous.revision) + 1 };
      const savedId = String(entity.id);
      collections.issues = collections.issues.map((item) => item.id === savedId ? saved : item);
      const result = { status: "ok", entity: saved };
      receipts.set(String(payload.commandId), { signature: payload.commandSignature, result });
      return result;
    }
    return { status: "failed", code: "capability_unavailable" };
  });
  const bridge = { postJson, openThread: async () => undefined };
  return { collections, bridge, postJson, adapter: new MulticaApiAdapter(bridge), writes: () => postJson.mock.calls.filter(([path]) => path === "/multica/workspace/upsert").map(([, payload]) => payload) };
}

let container: HTMLDivElement | undefined;
let host: HTMLDivElement | undefined;
afterEach(async () => {
  if (container) await act(async () => runtime.unmount(container!));
  host?.remove();
  container = undefined;
  host = undefined;
  delete window.__CODEX_WORKFLOW_BRIDGE__;
  vi.restoreAllMocks();
});

describe("original issue write-time run controls", () => {
  it.each([
    { suppress_run: true }, { suppress_run: false }, { handoff_note: "Review only the changed files" },
    { handoff_note: "" }, { suppress_run: true, handoff_note: "Do not dispatch this note" },
  ])("forwards original update controls outside the saved Issue: %j", async (controls) => {
    const f = fixture();
    vi.spyOn(workflowHost, "workflowFetch").mockImplementation((path, request) => f.adapter.transport(String(path), request));
    const issue = await new ApiClient("").updateIssue("issue-1", { assignee_type: "agent", assignee_id: "agent-1", ...controls });
    expect(issue.assignee_id).toBe("agent-1");
    expect(f.writes()).toHaveLength(1);
    const write = f.writes()[0];
    if ("suppress_run" in controls) expect(write.suppressRun).toBe(controls.suppress_run); else expect(write).not.toHaveProperty("suppressRun");
    if ("handoff_note" in controls) expect(write.handoffNote).toBe(controls.handoff_note); else expect(write).not.toHaveProperty("handoffNote");
    for (const field of ["suppress_run", "handoff_note", "suppressRun", "handoffNote"]) {
      expect(write.entity).not.toHaveProperty(field);
      expect(issue).not.toHaveProperty(field);
    }
    expect(f.postJson.mock.calls.some(([path]) => path.startsWith("/multica/executions/"))).toBe(false);
  });
  it("forwards promotion and all batch items using original client methods", async () => {
    const f = fixture();
    vi.spyOn(workflowHost, "workflowFetch").mockImplementation((path, request) => f.adapter.transport(String(path), request));
    const client = new ApiClient("");
    await client.updateIssue("issue-1", { status: "in_progress", handoff_note: "Start the review" });
    expect(f.writes()[0]).toMatchObject({ handoffNote: "Start the review", entity: { status: "in_progress" } });
    expect(await client.batchUpdateIssues(["issue-1", "issue-2"], { assignee_type: "agent", assignee_id: "agent-1", suppress_run: true })).toEqual({ updated: 2 });
    const batch = f.writes().slice(1);
    expect(batch).toHaveLength(2);
    expect(batch.every((write) => write.suppressRun === true)).toBe(true);
    expect(new Set(batch.map((write) => write.commandId)).size).toBe(2);
    expect(new Set(batch.map((write) => write.commandSignature)).size).toBe(2);
  });
  it("replays controls after adapter recreation and conflicts on a changed run decision", async () => {
    const f = fixture();
    const request = init({ assignee_type: "agent", assignee_id: "agent-1", suppress_run: true });
    const first = await f.adapter.transport("/api/issues/issue-1", request);
    const replay = await new MulticaApiAdapter(f.bridge).transport("/api/issues/issue-1", request);
    expect(replay.status).toBe(200);
    expect(await replay.json()).toEqual(await first.json());
    expect(f.writes()).toHaveLength(1);
    expect((await new MulticaApiAdapter(f.bridge).transport("/api/issues/issue-1", init({ assignee_type: "agent", assignee_id: "agent-1", suppress_run: false }))).status).toBe(409);
    expect(f.writes()).toHaveLength(1);
  });
  it("reports real partial batch failure without claiming all run decisions were saved", async () => {
    const f = fixture(), original = f.postJson.getMockImplementation()!;
    f.postJson.mockImplementation(async (path, payload) => path === "/multica/workspace/upsert" && (payload.entity as JsonRecord).id === "issue-2" ? { status: "failed", code: "revision_conflict" } : original(path, payload));
    const result = await f.adapter.transport("/api/issues/batch-update", { ...init({ issue_ids: ["issue-1", "issue-2"], updates: { suppress_run: false, handoff_note: "Exact batch note" } }), method: "POST" });
    expect(result.status).toBe(409);
    expect(await result.json()).toMatchObject({ completed_ids: ["issue-1"], failed_id: "issue-2" });
    expect(f.writes().every((write) => write.handoffNote === "Exact batch note" && write.suppressRun === false)).toBe(true);
    expect(f.collections.issues[1].revision).toBe(1);
  });
  it.each([{ suppress_run: "true" }, { suppress_run: null }, { handoff_note: 42 }, { handoff_note: "x".repeat(4097) }])("rejects invalid controls before writing: %j", async (controls) => {
    const f = fixture();
    expect((await f.adapter.transport("/api/issues/issue-1", init(controls))).status).toBe(400);
    expect(f.writes()).toHaveLength(0);
  });
  it("retains upstream move/create field boundaries instead of silently dropping controls", async () => {
    const f = fixture();
    let boundaryKey = 0;
    for (const controls of [{ suppress_run: true }, { handoff_note: "Note" }]) {
      expect((await f.adapter.transport("/api/issues/issue-1/move", { ...init({ before_id: null, after_id: null, ...controls }, `boundary-${boundaryKey++}`), method: "POST" })).status).toBe(400);
      expect((await f.adapter.transport("/api/issues", { ...init({ title: "New issue", ...controls }, `boundary-${boundaryKey++}`), method: "POST" })).status).toBe(400);
    }
    expect(f.writes()).toHaveLength(0);
    expect(f.postJson.mock.calls.some(([path]) => path === "/multica/workspace/move-issue")).toBe(false);
  });
  it.each(["confirm", "suppress", "legacy"])("uses the original run-confirm dialog: %s", async (choice) => {
    localStorage.clear();
    const f = fixture(choice !== "legacy");
    window.__CODEX_WORKFLOW_BRIDGE__ = f.bridge;
    host = document.createElement("div");
    document.body.append(host);
    container = document.createElement("div");
    host.attachShadow({ mode: "open" }).append(container);
    await act(async () => runtime.mount(container!, { route: "agents", workspaceId: workspace.id, workspaceSlug: workspace.slug }));
    await waitFor(() => expect(container!.querySelector(".ccp-workflow-page h1")).not.toBeNull());
    act(() => useModalStore.getState().open("issue-run-confirm", { issueIds: ["issue-1"], assigneeType: "agent", assigneeId: "agent-1", assigneeName: "Native agent" }));
    const dialog = await within(container).findByRole("dialog");
    const note = within(dialog).getByRole("textbox") as HTMLTextAreaElement;
    await waitFor(() => expect(f.postJson.mock.calls.some(([, payload]) => payload.resource === "runtimes")).toBe(true));
    await waitFor(() => expect(note.disabled).toBe(choice === "legacy"));
    if (choice === "legacy") return;
    fireEvent.change(note, { target: { value: "  Review only the changed files  " } });
    fireEvent.click(within(dialog).getByRole("button", { name: choice === "suppress" ? "暂不开始" : "确认指派" }));
    await waitFor(() => expect(f.writes()).toHaveLength(1));
    expect(f.writes()[0]).toMatchObject({ entity: { assignee_type: "agent", assignee_id: "agent-1" } });
    if (choice === "suppress") {
      expect(f.writes()[0].suppressRun).toBe(true);
      expect(f.writes()[0]).not.toHaveProperty("handoffNote");
    } else {
      expect(f.writes()[0].handoffNote).toBe("Review only the changed files");
      expect(f.writes()[0]).not.toHaveProperty("suppressRun");
    }
  });
});
