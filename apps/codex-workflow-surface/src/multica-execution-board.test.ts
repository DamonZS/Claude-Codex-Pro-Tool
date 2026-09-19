import { describe, expect, it, vi } from "vitest";
import { MulticaApiAdapter } from "./multica-api-adapter";
import type { JsonRecord } from "./multica-adapter-dto";
import { getExecutionBoardStatus, subscribeExecutionBoardStatus } from "./multica-execution-board";
import { IssueSchema, AgentTaskSchema } from "../vendor/multica/packages/core/api/schemas";

const now = Date.parse("2026-09-19T00:00:00Z");
const issue = (id: string, extra: JsonRecord = {}) => ({ id, revision: 1, title: id, workspace_id: "workspace", status: "todo", creator_type: "member", creator_id: "user", ...extra });
const native = (id: string, status = "running", extra: JsonRecord = {}) => ({ id, parent_thread_id: "parent", agent_nickname: "Descartes", status, updated_at_ms: now, source: "codex_native", read_only: true, title: "PRIVATE FULL PROMPT", ...extra });
const execution = (bindingId: string, extra: JsonRecord = {}) => ({ bindingId, workspaceId: "workspace", agentId: "agent", state: "running", revision: 1, createdAtMs: now, updatedAtMs: now, attemptNo: 1, ...extra });
function fixture() {
  const data: Record<string, JsonRecord[]> = { issues: [], agents: [{ id: "agent", revision: 1, name: "Worker", owner_id: "user" }], squads: [], codex_native_agents: [], executions: [] };
  const errors = new Set<string>();
  const stalePages = new Set<string>();
  const postJson = vi.fn(async (path: string, payload: JsonRecord) => {
    if (path === "/multica/workspace/bootstrap") return { workspace: { id: "workspace" }, user: { id: "user" } };
    const source = path === "/multica/executions/list" ? "executions" : String(payload.resource);
    if (errors.has(source)) throw new Error("read_failed");
    const all = data[source];
    if (!all) throw new Error(`unexpected_call:${path}`);
    return { status: "ok", items: all.slice(Number(payload.offset), Number(payload.offset) + Number(payload.limit)), total: all.length, stale: stalePages.has(`${source}:${payload.offset}`) };
  });
  const adapter = new MulticaApiAdapter({ postJson });
  const get = async (path = "/api/issues") => {
    const reply = await adapter.transport(path);
    expect(reply.status).toBe(200);
    return reply.json();
  };
  const table = async (kind: string, body: JsonRecord = {}) => {
    const reply = await adapter.transport(`/api/issues/table/${kind}`, { method: "POST", body: JSON.stringify({ query: { scope: { kind: "my", relation: "any" }, filters: {}, sort: { field: "title", direction: "asc" } }, group: { kind: "status" }, ...body }) });
    expect(reply.status).toBe(200);
    return reply.json();
  };
  return { data, errors, stalePages, postJson, adapter, get, table };
}

describe("unified execution board through adapter requests", () => {
  it("merges native and execution-only work into the original board before grouping/filtering/paging", async () => {
    const f = fixture();
    f.data.issues = [issue("ordinary", { status: "custom_review" }), issue("linked")];
    f.data.executions = [execution("old", { issueId: "linked", state: "completed", createdAtMs: now - 10 }), execution("new", { issueId: "linked", state: "queued", codexThreadId: "linked-thread" }), execution("solo", { codexThreadId: "solo-thread" })];
    f.data.codex_native_agents = [native("linked-thread"), native("solo-thread"), native("child")];
    const result = await f.get();
    expect(result.total).toBe(4);
    expect(result.issues.map((i: JsonRecord) => i.id).sort()).toEqual(["ccp-execution:solo", "codex-native:child", "linked", "ordinary"]);
    expect(result.issues.find((i: JsonRecord) => i.id === "linked")).toMatchObject({ status: "todo", metadata: { ccp_execution_state: "queued" } });
    expect(result.issues.find((i: JsonRecord) => i.id === "ordinary")).toMatchObject({ status: "custom_review" });
    expect((await f.get("/api/issues?status=in_progress&limit=1&offset=1")).total).toBe(2);
    expect((await f.table("groups")).groups).toEqual(expect.arrayContaining([{ key: "status:in_progress", value: { kind: "status", status: "in_progress" }, count: 2 }]));
    const card = await f.get("/api/issues/codex-native%3Achild");
    expect(IssueSchema.safeParse(card).success).toBe(true);
    expect(card).toMatchObject({ title: "Descartes", metadata: { ccp_read_only: true, ccp_source: "codex-native", ccp_thread_id: "child", ccp_parent_thread_id: "parent", ccp_agent_name: "Descartes" } });
    expect(JSON.stringify(card)).not.toContain("PRIVATE FULL PROMPT");
    expect(await f.get("/api/agents")).toHaveLength(1);
    const working = await f.get("/api/working-agents?type=issue&scope=mine&relation=any");
    expect(working.flatMap((a: JsonRecord) => a.issue_ids).sort()).toEqual(["ccp-execution:solo", "codex-native:child"]);
    const snapshot = await f.get("/api/agent-task-snapshot");
    expect(snapshot.filter((t: JsonRecord) => t.status === "running").map((t: JsonRecord) => t.issue_id).sort()).toEqual(["ccp-execution:solo", "codex-native:child"]);
    for (const task of snapshot) expect(AgentTaskSchema.safeParse(task).success).toBe(true);
    expect(f.postJson.mock.calls.every(([path]) => ["/multica/workspace/bootstrap", "/multica/workspace/query", "/multica/executions/list"].includes(path))).toBe(true);
  });

  it.each(["codex-native:child", "ccp-execution:solo"])("blocks all virtual mutations before any write: %s", async (id) => {
    const f = fixture();
    const requests: [string, string, JsonRecord][] = [
      [`/api/issues/${id}`, "PATCH", { title: "edited" }], [`/api/issues/${id}`, "DELETE", {}],
      [`/api/issues/${id}/move`, "POST", { status: "done", before_id: null, after_id: null }],
      [`/api/issues/${id}/comments`, "POST", { content: "comment" }], [`/api/issues/${id}/rerun`, "POST", {}],
      [`/api/issues/${id}/subscribe`, "POST", {}], [`/api/issues/${id}/properties/p`, "PUT", { value: true }],
      ["/api/issues/batch-delete", "POST", { issue_ids: ["ordinary", id] }],
      ["/api/issues/batch-update", "POST", { issue_ids: [id], updates: { status: "done" } }],
      ["/api/issues", "POST", { title: "nested", parent_issue_id: id }],
    ];
    for (const [path, method, body] of requests) {
      const reply = await f.adapter.transport(path, { method, body: JSON.stringify(body) });
      expect(reply.status).toBe(403);
      expect(await reply.json()).toMatchObject({ code: "read_only_projection" });
    }
    expect(f.postJson).not.toHaveBeenCalled();
  });

  it("preserves a full board and working summary on failures, and retains ordinary Issues on a cold degraded read", async () => {
    const f = fixture(), changed = vi.fn(), unsubscribe = subscribeExecutionBoardStatus(changed);
    f.data.issues = [issue("ordinary")];
    f.data.executions = [execution("solo")];
    f.data.codex_native_agents = [native("child", "unknown")];
    f.stalePages.add("codex_native_agents:0");
    expect((await f.get()).total).toBe(3);
    expect(getExecutionBoardStatus()).toMatchObject({ stale: true, updatedAt: null });
    expect((await f.get("/api/issues/codex-native:child")).metadata.ccp_execution_state).toBe("unknown");
    f.stalePages.clear();
    f.data.codex_native_agents = [native("child", "inProgress")];
    const complete = await f.get();
    const working = await f.get("/api/working-agents");
    expect(getExecutionBoardStatus().stale).toBe(false);
    const snapshot = getExecutionBoardStatus();
    expect(getExecutionBoardStatus()).toBe(snapshot);
    f.data.issues = [];
    f.errors.add("executions");
    expect(await f.get()).toEqual(complete);
    expect(await f.get("/api/working-agents")).toEqual(working);
    expect(getExecutionBoardStatus()).toMatchObject({ stale: true });
    f.errors.clear();
    f.data.codex_native_agents = [native("child", "completed")];
    expect((await f.get("/api/issues?status=done")).issues.map((i: JsonRecord) => i.id)).toEqual(["codex-native:child"]);
    expect(getExecutionBoardStatus().stale).toBe(false);
    expect(changed).toHaveBeenCalled();
    unsubscribe();
    const cold = fixture();
    cold.data.issues = [issue("ordinary", { status: "custom" })];
    cold.errors.add("codex_native_agents");
    expect((await cold.get()).issues).toMatchObject([{ id: "ordinary", status: "custom" }]);
    expect(getExecutionBoardStatus().stale).toBe(true);
  });

  it("reads native pages up to 500 and retains the whole previous board when the second page is stale", async () => {
    const f = fixture();
    f.data.codex_native_agents = Array.from({ length: 520 }, (_, i) => native(`child-${i}`, "inProgress"));
    expect((await f.get()).total).toBe(500);
    expect(f.postJson.mock.calls.filter(([, p]) => p.resource === "codex_native_agents").map(([, p]) => p.offset)).toEqual([0, 100, 200, 300, 400]);
    f.stalePages.add("codex_native_agents:100");
    f.data.codex_native_agents[0].status = "completed";
    expect((await f.get("/api/issues?status=in_progress")).total).toBe(500);
    expect(getExecutionBoardStatus().stale).toBe(true);
  });

  it("uses numeric creation/attempt ordering with stable binding ties and suppresses old linked threads", async () => {
    const f = fixture();
    f.data.issues = [issue("linked")];
    f.data.executions = [
      execution("old", { issueId: "linked", state: "completed", createdAtMs: now - 1, updatedAtMs: now + 100, codexThreadId: "old-thread" }),
      execution("z", { issueId: "linked", state: "running", attemptNo: 9 }),
      execution("a", { issueId: "linked", state: "completed", attemptNo: 10 }),
      execution("b", { issueId: "linked", state: "binding_pending", attemptNo: 10, codexThreadId: "new-thread" }),
    ];
    f.data.codex_native_agents = [native("old-thread"), native("new-thread")];
    expect((await f.get()).issues).toMatchObject([{ id: "linked", status: "todo", metadata: { ccp_source: "multica-execution", ccp_binding_id: "b", ccp_execution_state: "binding_pending" } }]);
    expect(await f.get("/api/working-agents")).toEqual([]);
  });

  it.each([
    ["binding_pending", "todo"], ["dispatched", "todo"], ["waiting_local_directory", "todo"], ["running", "in_progress"],
    ["cancel_pending", "in_progress"], ["completed", "done"], ["failed", "blocked"], ["interrupted", "blocked"], ["cancelled", "cancelled"],
    ["reconciling", "blocked"], ["stale", "blocked"], ["orphaned", "blocked"], ["future_state", "blocked"],
  ])("projects %s to %s while retaining the real state and actor", async (state, category) => {
    const f = fixture();
    f.data.executions = [execution("solo", { state, parentThreadId: "parent", updatedAtMs: now + 50 })];
    const card = await f.get("/api/issues/ccp-execution:solo");
    expect(card).toMatchObject({ assignee_id: "agent", status: category, updated_at: new Date(now + 50).toISOString(), metadata: { ccp_execution_state: state, ccp_parent_thread_id: "parent" } });
    expect((await f.get("/api/working-agents")).length).toBe(category === "in_progress" ? 1 : 0);
  });

  it("invalidates table cursors when execution status changes even if the stored Issue revision is unchanged", async () => {
    const f = fixture();
    f.data.issues = [issue("a"), issue("b")];
    f.data.executions = [execution("linked", { issueId: "a" })];
    const query = { scope: { kind: "workspace" }, filters: {}, sort: { field: "title", direction: "asc" } };
    const first = await f.table("rows", { query, group: { kind: "none" }, page: { limit: 1 } });
    f.data.executions[0].state = "completed";
    const reply = await f.adapter.transport("/api/issues/table/rows", { method: "POST", body: JSON.stringify({ query, group: { kind: "none" }, page: { limit: 1, cursor: first.next_cursor } }) });
    expect(reply.status).toBe(409);
    expect(await reply.json()).toMatchObject({ code: "query_cursor_conflict" });
  });

  it("gives an existing Issue priority over a newer unlinked binding on the same thread", async () => {
    const f = fixture();
    f.data.issues = [issue("linked")];
    f.data.executions = [execution("linked-run", { issueId: "linked", codexThreadId: "shared", state: "running" }), execution("orphan", { codexThreadId: "shared", state: "completed", createdAtMs: now + 1 })];
    f.data.codex_native_agents = [native("shared", "completed")];
    expect((await f.get()).issues).toMatchObject([{ id: "linked", status: "in_progress", metadata: { ccp_binding_id: "linked-run" } }]);
  });

  it("keeps more than 500 execution-only records, ignores idle definitions, and shares working filters/facets", async () => {
    const f = fixture();
    expect((await f.get()).total).toBe(0);
    f.data.executions = Array.from({ length: 501 }, (_, i) => execution(`run-${i}`, { state: i === 500 ? "running" : "completed" }));
    expect((await f.get("/api/issues?limit=600")).total).toBe(501);
    const query = { scope: { kind: "my", relation: "involved" }, filters: { working_only: true }, sort: {} };
    const rows = await f.table("rows", { query, group: { kind: "none" } });
    expect(rows.rows.map((r: { issue: JsonRecord }) => r.issue.id)).toEqual(["ccp-execution:run-500"]);
    const facets = await f.table("facets", { query, facets: [{ kind: "working_agents" }] });
    expect(facets.facets[0].values).toEqual([{ key: "agent", count: 1 }]);
  });

  it("does not mark reconciled or unknown tasks as active ahead of the latest completed task", async () => {
    const f = fixture();
    f.data.executions = [execution("uncertain", { state: "reconciling" }), execution("finished", { state: "completed", createdAtMs: now + 1 })];
    const snapshot = await f.get("/api/agent-task-snapshot");
    expect(snapshot.map((t: JsonRecord) => t.id)).toEqual(["finished"]);
    expect(await f.get("/api/working-agents")).toEqual([]);
  });

  it("does not classify the binding run id as an autopilot working agent", async () => {
    const f = fixture();
    f.data.executions = [execution("thread-run", { multicaRunId: "ordinary-run", executionKind: "thread", state: "running" })];
    expect(await f.get("/api/working-agents?type=autopilot")).toEqual([]);
  });

  it("retains the first degraded usable board through later source outages before any complete snapshot", async () => {
    const f = fixture();
    f.data.issues = [issue("ordinary")];
    f.data.codex_native_agents = [native("child", "unknown")];
    f.stalePages.add("codex_native_agents:0");
    const first = await f.get();
    f.errors.add("codex_native_agents");
    expect(await f.get()).toEqual(first);
    f.errors.add("issues");
    expect(await f.get()).toEqual(first);
    expect(getExecutionBoardStatus()).toMatchObject({ stale: true, updatedAt: null });
  });

  it("deduplicates Issue metadata associations before creating virtual bindings or native cards", async () => {
    const f = fixture();
    f.data.issues = [issue("linked", { metadata: { ccp_binding_id: "saved-binding", ccp_thread_id: "saved-thread", custom: true } })];
    f.data.executions = [execution("saved-binding", { codexThreadId: "saved-thread", state: "running" })];
    f.data.codex_native_agents = [native("saved-thread", "completed")];
    expect((await f.get()).issues).toMatchObject([{ id: "linked", status: "in_progress", metadata: { custom: true, ccp_binding_id: "saved-binding" } }]);
  });

  it("projects a native status through an Issue thread metadata association even without a binding", async () => {
    const f = fixture();
    f.data.issues = [issue("linked", { metadata: { ccp_thread_id: "native-thread" } })];
    f.data.codex_native_agents = [native("native-thread", "inProgress")];
    const result = await f.get();
    expect(result.issues).toMatchObject([{ id: "linked", status: "in_progress", metadata: { ccp_source: "codex-native", ccp_thread_id: "native-thread", ccp_execution_state: "inProgress" } }]);
    expect(result.issues).toHaveLength(1);
  });

  it("marks unknown native statuses pending confirmation without using edge status or inventing active tasks", async () => {
    const f = fixture();
    f.data.codex_native_agents = [native("unknown", "future_state", { edge_status: "closed" }), native("finished", "completed", { edge_status: "open" })];
    const items = (await f.get()).issues;
    expect(items.find((i: JsonRecord) => i.id === "codex-native:unknown")).toMatchObject({ status: "blocked", metadata: { ccp_execution_state: "future_state" } });
    expect(items.find((i: JsonRecord) => i.id === "codex-native:finished")).toMatchObject({ status: "done" });
    expect(await f.get("/api/working-agents")).toEqual([]);
    expect(getExecutionBoardStatus().stale).toBe(false);
  });

  it("rejects synthetic display agents as execution or assignment targets without calling the bridge", async () => {
    const f = fixture();
    for (const id of ["codex-native-agent:child", "ccp-execution-agent:binding"]) {
      const reply = await f.adapter.transport("/api/issues/quick-create", { method: "POST", body: JSON.stringify({ agent_id: id, prompt: "run" }) });
      expect(reply.status).toBe(403);
    }
    expect(f.postJson).not.toHaveBeenCalled();
  });

  it("orders virtual cards by recent updates under the default position sort without changing real Issue positions", async () => {
    const f = fixture();
    f.data.issues = [issue("ordinary", { position: 42 })];
    f.data.codex_native_agents = [native("a-old", "inProgress", { updated_at_ms: now - 2 }), native("z-new", "inProgress", { updated_at_ms: now })];
    f.data.executions = [execution("middle", { updatedAtMs: now - 1 })];
    const result = await f.get();
    expect(result.issues.map((row: JsonRecord) => row.id)).toEqual(["codex-native:z-new", "ccp-execution:middle", "codex-native:a-old", "ordinary"]);
    expect(result.issues.find((row: JsonRecord) => row.id === "ordinary").position).toBe(42);
  });

  it("keeps an adopted native continuation on its original card while queue and actual turn states change", async () => {
    const f = fixture();
    f.data.codex_native_agents = [native("child", "completed")];
    f.data.executions = [execution("native-binding", { agentId: null, codexThreadId: "child", state: "binding_pending", nativeResume: true, parentThreadId: "parent" })];
    let cards = (await f.get()).issues;
    expect(cards).toHaveLength(1);
    expect(cards[0]).toMatchObject({ id: "codex-native:child", status: "todo", metadata: { ccp_source: "codex-native", ccp_execution_state: "binding_pending", ccp_binding_id: "native-binding", ccp_agent_name: "Descartes" } });
    f.data.executions[0].state = "running";
    expect((await f.get("/api/issues/codex-native:child")).status).toBe("in_progress");
    f.data.executions[0].state = "failed";
    cards = (await f.get()).issues;
    expect(cards).toHaveLength(1);
    expect(cards[0]).toMatchObject({ id: "codex-native:child", status: "blocked", metadata: { ccp_execution_state: "failed" } });
  });
});
