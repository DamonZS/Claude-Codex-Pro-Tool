// @vitest-environment node
import { afterEach, describe, expect, it, vi } from "vitest";
import { MulticaApiAdapter, type MulticaAdapterBridge } from "./multica-api-adapter";
import { type JsonRecord, taskDto } from "./multica-adapter-dto";
import { buildCreateAgentRequest, EMPTY_AGENT_DRAFT } from "../vendor/multica/packages/core/agents/draft";
import { ApiClient } from "../vendor/multica/packages/core/api/client";
import { configStore } from "../vendor/multica/packages/core/config";
import { quickCreateVersionBlocked } from "../vendor/multica/packages/core/runtimes/cli-version";
import * as workflowHost from "./upstream-host";
import {
  AgentTaskSchema, AutopilotRunSchema, IssueSchema, IssueViewListSchema, IssueViewPreferenceSchema, IssuePropertySchema, IssuePropertiesResponseSchema, ListPropertiesResponseSchema, SubscribersListSchema, QuickActionSchema,
  ListAutopilotsResponseSchema, ListIssueStatusesResponseSchema, SearchIssuesResponseSchema, IssueTableRowsResponseSchema, IssueTableGroupsResponseSchema,
} from "../vendor/multica/packages/core/api/schemas";

const date = "2026-09-18T00:00:00.000Z";
// Shape emitted by MulticaWorkspaceBootstrap and runtime_collection, not upstream server DTOs.
const coreBootstrap = {
  status: "ok", fetchedAtMs: Date.parse(date), workspace: { id: "workspace-1", slug: "local", name: "Local" },
  user: { id: "user-1", kind: "local_control_plane" },
  runtime: { available: true, runtimeId: "native-runtime", provider: "codex", status: "available", capabilities: [], skillsSupported: false, nativeTaskHostSupported: true, skillsInventorySupported: false, skillProtocol: null, multiAgentSupported: false },
  modules: ["issues", "autopilots", "agents"], collections: {},
};
const coreRuntime = {
  id: "native-runtime", workspace_id: "workspace-1", kind: "codex_page_host", provider: "codex", status: "available",
  capabilities: [], skills_supported: false, native_task_host_supported: true, skills_inventory_supported: false,
  skill_protocol: null, multi_agent_supported: false, registered: false,
};
const issue = (id: string, overrides: JsonRecord = {}): JsonRecord => ({
  id, workspace_id: "workspace-1", title: `Task ${id}`, description: "Description", revision: 3,
  status: "todo", priority: "none", creator_type: "member", creator_id: "user-1",
  assignee_type: "member", assignee_id: "user-1", position: 0, created_at: date, updated_at: date, ...overrides,
});
const autopilot = (overrides: JsonRecord = {}): JsonRecord => ({ id: "auto-1", workspace_id: "workspace-1", title: "Automation", assignee_id: "agent-1", revision: 2, created_by_id: "user-1", triggers: [], created_at: date, updated_at: date, ...overrides });
const binding = (overrides: JsonRecord = {}): JsonRecord => ({ bindingId: "binding-1", workspaceId: "workspace-1", issueId: "issue-1", agentId: "agent-1", codexRuntimeId: "native-runtime", codexThreadId: "native-thread", state: "running", revision: 4, attemptNo: 1, createdAtMs: Date.parse(date), updatedAtMs: Date.parse(date), ...overrides });
const run = (overrides: JsonRecord = {}): JsonRecord => ({ id: "run-1", autopilotId: "auto-1", triggerId: null, source: "manual", status: "running", revision: 1, issueId: "issue-1", taskId: "binding-1", triggeredAtMs: Date.parse(date), createdAtMs: Date.parse(date), ...overrides });
const init = (method: string, body: JsonRecord = {}, key?: string): RequestInit => ({ method, body: JSON.stringify(body), ...(key ? { headers: { "Idempotency-Key": key } } : {}) });

function fixture(initial: Record<string, JsonRecord[]> = {}) {
  const collections = structuredClone(initial);
  const executions = [binding()];
  const receipts = new Map<string, { signature: unknown; result: unknown }>();
  const complete = (payload: JsonRecord, result: unknown) => {
    if (typeof payload.commandId === "string") receipts.set(payload.commandId, { signature: payload.commandSignature, result: structuredClone(result) });
    return result;
  };
  const postJson = vi.fn(async (path: string, payload: JsonRecord): Promise<unknown> => {
    if (path === "/multica/workspace/bootstrap") return structuredClone(coreBootstrap);
    if (path === "/multica/workspace/command") {
      const receipt = receipts.get(String(payload.commandId));
      if (receipt && receipt.signature !== payload.commandSignature) return { status: "failed", code: "multica_workspace_idempotency_conflict" };
      return { status: "ok", found: !!receipt, result: receipt ? structuredClone(receipt.result) : null };
    }
    if (path === "/multica/workspace/query") {
      const items = collections[String(payload.resource)] ?? [];
      return { status: "ok", items: items.slice(Number(payload.offset), Number(payload.offset) + Number(payload.limit)), total: items.length };
    }
    if (path === "/multica/workspace/upsert" || path === "/multica/agents/create") {
      const resource = path.endsWith("create") ? "agents" : String(payload.resource);
      const input = payload.entity as JsonRecord;
      const items = collections[resource] ?? [];
      const previous = items.find((item) => item.id === input.id);
      if (previous && payload.expectedRevision !== previous.revision) return { status: "failed", code: "multica_workspace_revision_conflict", current_revision: previous.revision };
      const saved = { ...input, workspace_id: "workspace-1", revision: Number(previous?.revision ?? 0) + 1, created_at_ms: previous?.created_at_ms ?? Date.parse(date), updated_at_ms: Date.parse(date) };
      collections[resource] = [...items.filter((item) => item.id !== input.id), saved];
      return complete(payload, { status: "ok", entity: saved });
    }
    if (path === "/multica/workspace/delete") {
      const resource = String(payload.resource), items = collections[resource] ?? [];
      const previous = items.find((item) => item.id === payload.entityId);
      if (previous && previous.revision !== payload.expectedRevision) return { status: "failed", code: "revision_conflict" };
      collections[resource] = items.filter((item) => item.id !== payload.entityId);
      return complete(payload, { status: "ok", deleted: !!previous });
    }
    if (path === "/multica/workspace/move-issue") return complete(payload, { status: "ok", entity: issue(String(payload.issueId), { status: payload.status, assignee_type: payload.assigneeType, assignee_id: payload.assigneeId, revision: Number(payload.expectedRevision) + 1 }) });
    if (path === "/multica/executions/list") {
      const items = executions.filter((item) => !payload.issueId || item.issueId === payload.issueId);
      return { status: "ok", items: items.slice(Number(payload.offset), Number(payload.offset) + Number(payload.limit)), total: items.length };
    }
    if (["/multica/executions/open", "/multica/executions/status", "/multica/executions/create", "/multica/executions/continue"].includes(path)) return { status: "ok", binding: binding({ revision: 5 }) };
    if (path === "/multica/executions/cancel") return { status: "ok", binding: binding({ state: "cancelled", revision: 5, completedAtMs: Date.parse(date) }) };
    if (path === "/multica/autopilots/runs") return { status: "ok", runs: [run(), run({ id: "run-2" })], total: 2 };
    if (path === "/multica/autopilots/run" || path === "/multica/autopilots/trigger") return { status: "ok", run: run() };
    return { status: "failed", code: "capability_unavailable" };
  });
  const bridge: MulticaAdapterBridge = { postJson, openThread: vi.fn(async () => undefined) };
  return { adapter: new MulticaApiAdapter(bridge), bridge, postJson, collections, executions, receipts };
}
afterEach(() => vi.restoreAllMocks());

describe("original page DTO compatibility", () => {
  it("declares supported starter persistence and accepts original Builder draft create/update requests", async () => {
    const f = fixture(), client = new ApiClient("");
    vi.spyOn(workflowHost, "workflowFetch").mockImplementation((path, request) => f.adapter.transport(String(path), request));
    const previous = configStore.getState().agentConversationStartersSupported;
    const starters = [{ label: "Review", prompt: "Review the current changes" }];
    const request = buildCreateAgentRequest({ draft: { ...EMPTY_AGENT_DRAFT, name: "Builder draft", conversationStarters: starters }, runtimeId: "native-runtime" });
    expect(request.conversation_starters).toEqual(starters);
    try {
      configStore.getState().setAgentConversationStartersSupported(false);
      await expect(client.createAgent(request)).rejects.toThrow(/conversation starters/);
      expect(f.postJson).not.toHaveBeenCalled();
      const config = await client.getConfig();
      expect(config).toMatchObject({ agent_conversation_starters_supported: true, allow_signup: false, workspace_creation_disabled: true, local_worktree_supported: false, vcs_integration_available: false, feature_flags: { local_property_catalog_management: false } });
      expect(config.server_version).toBeUndefined();
      configStore.getState().setAgentConversationStartersSupported(config.agent_conversation_starters_supported);
      const agent = await client.createAgent(request);
      expect(agent.id).not.toBe("");
      expect(agent.conversation_starters).toEqual(starters);
      expect(f.collections.agents[0].conversation_starters).toEqual(starters);
      expect((await client.getAgent(agent.id)).conversation_starters).toEqual(starters);
      const updated = [{ label: "Summarize", prompt: "Summarize the current task" }];
      expect((await client.updateAgent(agent.id, { conversation_starters: updated })).conversation_starters).toEqual(updated);
      expect(f.collections.agents[0].conversation_starters).toEqual(updated);
      expect((await client.updateAgent(agent.id, { conversation_starters: [] })).conversation_starters).toEqual([]);
      expect(f.collections.agents[0].conversation_starters).toEqual([]);
    } finally {
      configStore.getState().setAgentConversationStartersSupported(previous);
    }
  });
  it("does not declare capabilities when authoritative bootstrap fails", async () => {
    const f = fixture();
    f.postJson.mockResolvedValue({ status: "failed", code: "capability_unavailable" });
    const response = await f.adapter.transport("/api/config");
    expect(response.status).toBe(503);
    expect(await response.json()).not.toHaveProperty("agent_conversation_starters_supported");
  });
  it("returns actual saved view arrays and agent/autopilot shapes", async () => {
    const f = fixture({ issues: [issue("issue-1")], issue_views: [{ id: "view-1", revision: 2, scope_type: "my", scope_id: null, name: "My view" }], agents: [{ id: "agent-1", revision: 2, name: "Agent", permission_mode: "private", owner_id: "user-1" }], autopilots: [autopilot()] });
    const get = async (path: string) => (await f.adapter.transport(path)).json();
    expect(IssueSchema.safeParse(await get("/api/issues/issue-1")).success).toBe(true);
    const views = await get("/api/issue-views?scope_type=my");
    expect(IssueViewListSchema.safeParse(views).success).toBe(true);
    expect(views).toHaveLength(1);
    expect(await get("/api/agents?include_archived=true")).toMatchObject([{ id: "agent-1", permission_mode: "private" }]);
    expect(ListAutopilotsResponseSchema.safeParse(await get("/api/autopilots")).success).toBe(true);
    expect(await get("/api/autopilots/auto-1")).toMatchObject({ autopilot: { id: "auto-1" }, triggers: [] });
  });
  it("preserves bootstrap identity without minting an owner or token", async () => {
    const f = fixture();
    expect(await (await f.adapter.transport("/api/me")).json()).toEqual({ id: "user-1", kind: "local_control_plane" });
    expect(await (await f.adapter.transport("/api/workspaces/workspace-1/members")).json()).toEqual([{ id: "user-1", user_id: "user-1", workspace_id: "workspace-1", role: "member", name: "", email: "", avatar_url: null, created_at: "" }]);
    expect((await f.adapter.transport("/api/tokens")).status).toBe(503);
  });
  it("preserves the authoritative bootstrap id through the original getMe schema", async () => {
    const f = fixture();
    vi.spyOn(workflowHost, "workflowFetch").mockImplementation((path, request) => f.adapter.transport(String(path), request));
    const user = await new ApiClient("").getMe();
    expect(user.id).toBe(coreBootstrap.user.id);
    expect(user).toMatchObject({ name: "", email: "", avatar_url: null, created_at: "" });
    expect(user).not.toHaveProperty("role");
  });
  it("enables original quick create from verified native capability without inventing CLI metadata", async () => {
    const f = fixture({ runtimes: [coreRuntime] });
    vi.spyOn(workflowHost, "workflowFetch").mockImplementation((path, request) => f.adapter.transport(String(path), request));
    const [runtime] = await new ApiClient("").listRuntimes();
    expect(runtime.metadata).toEqual({ nativeTaskHostSupported: true, nativeModelSelectionAuthoritative: true, nativeHandoffSupported: true });
    expect(runtime.metadata).not.toHaveProperty("cli_version");
    expect(runtime).not.toHaveProperty("daemon_version");
    expect(quickCreateVersionBlocked(runtime.metadata, false)).toBe(false);
    expect(quickCreateVersionBlocked(runtime.metadata, true)).toBe(false);
  });
  it.each([
    { native_task_host_supported: false }, { native_task_host_supported: "true" },
    { id: "other-host" }, { status: "offline" },
  ])("keeps native capability closed for an unverified runtime %j", async (override) => {
    const f = fixture({ runtimes: [{ ...coreRuntime, ...override, metadata: { nativeTaskHostSupported: true } }] });
    const [runtime] = await (await f.adapter.transport("/api/runtimes")).json();
    expect(runtime.metadata.nativeTaskHostSupported).toBe(false);
    expect(runtime.metadata.nativeHandoffSupported).toBe(false);
    expect(quickCreateVersionBlocked(runtime.metadata, false)).toBe(true);
  });
  it("does not inherit native capability from an unavailable bootstrap Host", async () => {
    const f = fixture({ runtimes: [coreRuntime] });
    f.postJson.mockResolvedValueOnce({ ...coreBootstrap, runtime: { ...coreBootstrap.runtime, available: false } });
    const [runtime] = await (await f.adapter.transport("/api/runtimes")).json();
    expect(runtime.metadata.nativeTaskHostSupported).toBe(false);
  });
  it("normalizes the actual timestamp-less Core status catalog without erasing it", async () => {
    const f = fixture({ issue_statuses: [{ id: "issue-status-todo", workspace_id: "workspace-1", revision: 1, key: "todo", name: "Todo", category: "todo", description: "", color: "#2563EB", is_system: true, position: 1, archived_at: null }] });
    const result = await (await f.adapter.transport("/api/issue-statuses")).json();
    expect(ListIssueStatusesResponseSchema.safeParse(result).success).toBe(true);
    expect(result).toMatchObject({ total: 1, statuses: [{ key: "todo", created_at: "", updated_at: "" }] });
  });
  it.each(["/api/properties", "/api/issue-view-preferences?scope_type=my", "/api/autopilots/usage"])("never turns missing %s into empty success", async (path) => {
    const f = fixture();
    const original = f.postJson.getMockImplementation()!;
    f.postJson.mockImplementation(async (route, payload) => route === "/multica/workspace/query" ? { status: "failed", code: "capability_unavailable" } : original(route, payload));
    const result = await f.adapter.transport(path);
    expect(result.status).toBe(503);
    expect(await result.json()).toMatchObject({ code: "capability_unavailable" });
  });
  it("filters control-plane records out of the runtime selector", async () => {
    const f = fixture({ runtimes: [{ id: "control", kind: "control_plane" }, { id: "host-1", kind: "codex_page_host", status: "available", provider: "codex" }] });
    expect(await (await f.adapter.transport("/api/runtimes")).json()).toMatchObject([{ id: "host-1", status: "online" }]);
  });
  it("maps owner=me only for the verified current local Host runtime", async () => {
    const f = fixture({ runtimes: [{ id: "control", kind: "control_plane" }, { id: "host-1", kind: "codex_page_host", status: "available", provider: "codex" }, { id: "other-host", kind: "codex_page_host", status: "available" }] });
    f.postJson.mockResolvedValueOnce({ status: "ok", fetchedAtMs: Date.parse(date), workspace: { id: "workspace-1", slug: "local", name: "Local" }, user: { id: "user-1", kind: "local_control_plane" }, runtime: { available: true, runtimeId: "host-1" }, modules: [], collections: {} });
    expect(await (await f.adapter.transport("/api/runtimes?owner=me")).json()).toEqual([expect.objectContaining({ id: "host-1", owner_id: "user-1", status: "online" })]);
  });
});

describe("persisted property and preference contracts", () => {
  it.each([true, false, undefined, "true"])("projects only the explicit local catalog capability %s without elevating membership", async (capability) => {
    const f = fixture();
    f.postJson.mockResolvedValueOnce({ ...coreBootstrap, permissions: { managePropertyCatalog: capability } });
    const config = await (await f.adapter.transport("/api/config")).json();
    expect(config.feature_flags.local_property_catalog_management).toBe(capability === true);
    expect(await (await f.adapter.transport("/api/workspaces/workspace-1/members")).json()).toMatchObject([{ user_id: "user-1", role: "member" }]);
  });
  it("assigns distinct stable IDs to original editor options and replays after adapter reconstruction", async () => {
    const f = fixture();
    const request = init("POST", { name: "Size", type: "select", config: { options: [{ id: "", name: "Small", color: "#112233" }, { id: "", name: "Large", color: "#445566" }] } }, "original-options");
    const first = await f.adapter.transport("/api/properties", request);
    expect(first.status).toBe(201);
    const saved = await first.json();
    expect(IssuePropertySchema.safeParse(saved).success).toBe(true);
    const ids = saved.config.options.map((option: JsonRecord) => option.id);
    expect(ids.every((id: string) => id.length > 0)).toBe(true);
    expect(new Set(ids).size).toBe(2);
    expect(await (await new MulticaApiAdapter(f.bridge).transport("/api/properties", request)).json()).toEqual(saved);
    expect(f.postJson.mock.calls.filter(([path]) => path === "/multica/workspace/upsert")).toHaveLength(1);
  });
  it("round trips metadata through the actual vendored client without parser fallback", async () => {
    const f = fixture({ issues: [issue("issue-1")] });
    const transport = vi.spyOn(workflowHost, "workflowFetch").mockImplementation((path, request) => f.adapter.transport(String(path), request));
    try {
      const client = new ApiClient("");
      const property = await client.createProperty({ name: "Estimate", type: "number" });
      expect(property.id).not.toBe("");
      expect((await client.listProperties()).properties[0].name).toBe("Estimate");
      expect((await client.setIssueProperty("issue-1", property.id, 13)).properties[property.id]).toBe(13);
      expect((await client.unsetIssueProperty("issue-1", property.id)).properties).toEqual({});
      const prefs = await client.putIssueViewPreference({ scope_type: "my", prefs: { hidden: ["view-hidden"], order: ["view-visible"] } });
      expect(prefs.prefs.hidden).toEqual(["view-hidden"]);
      expect((await client.getIssueViewPreference({ scope_type: "my" })).prefs.order).toEqual(["view-visible"]);
      await client.subscribeToIssue("issue-1");
      expect((await client.listIssueSubscribers("issue-1"))[0].user_id).toBe("user-1");
      await client.unsubscribeFromIssue("issue-1");
      expect(await client.listIssueSubscribers("issue-1")).toEqual([]);
    } finally {
      transport.mockRestore();
    }
  });
  it("round trips the original property DTO and immutable type through durable receipts", async () => {
    const f = fixture();
    const request = init("POST", { name: "Size", type: "select", config: { options: [{ id: "small", name: "Small", color: "#112233" }] } }, "property-created");
    const created = await f.adapter.transport("/api/properties", request);
    expect(created.status).toBe(201);
    const property = await created.json();
    expect(IssuePropertySchema.safeParse(property).success).toBe(true);
    expect(property).toMatchObject({ id: "property-created", revision: 1, type: "select", created_at: date });
    const replay = await new MulticaApiAdapter(f.bridge).transport("/api/properties", request);
    expect(await replay.json()).toEqual(property);
    expect(f.postJson.mock.calls.filter(([path]) => path === "/multica/workspace/upsert")).toHaveLength(1);
    const updated = await f.adapter.transport("/api/properties/property-created", init("PATCH", { name: "Estimate", archived: true }));
    expect(await updated.json()).toMatchObject({ revision: 2, name: "Estimate", archived: true, type: "select" });
    const listed = await (await f.adapter.transport("/api/properties?include_archived=true")).json();
    expect(ListPropertiesResponseSchema.safeParse(listed).success).toBe(true);
    expect(listed.total).toBe(1);
    expect(await (await f.adapter.transport("/api/properties")).json()).toEqual({ properties: [], total: 0 });
    expect((await f.adapter.transport("/api/properties/property-created", init("PATCH", { type: "text" }))).status).toBe(400);
    expect((await f.adapter.transport("/api/properties/property-created", init("PATCH", { name: "Stale", expected_revision: 1 }))).status).toBe(409);
  });
  it("changes one property without losing other issue values and replays the original revision", async () => {
    const f = fixture({ issues: [issue("issue-1", { properties: { untouched: "keep" } })], properties: [{ id: "estimate", revision: 1, name: "Estimate", type: "number" }] });
    const request = init("PUT", { value: 8 }, "set-estimate");
    const saved = await (await f.adapter.transport("/api/issues/issue-1/properties/estimate", request)).json();
    expect(IssuePropertiesResponseSchema.safeParse(saved).success).toBe(true);
    expect(saved).toEqual({ properties: { untouched: "keep", estimate: 8 }, issue_revision: 4 });
    expect(await (await new MulticaApiAdapter(f.bridge).transport("/api/issues/issue-1/properties/estimate", request)).json()).toEqual(saved);
    expect(await (await f.adapter.transport("/api/issues/issue-1/properties/estimate", init("DELETE"))).json()).toEqual({ properties: { untouched: "keep" }, issue_revision: 5 });
    expect(f.postJson.mock.calls.filter(([path]) => path === "/multica/workspace/upsert")).toHaveLength(2);
    expect((await f.adapter.transport("/api/issues/issue-1/properties/estimate", init("PUT", { value: { unsupported: true } }))).status).toBe(400);
  });
  it("preserves Core property-validation errors without claiming a mutation", async () => {
    const f = fixture({ issues: [issue("issue-1")], properties: [{ id: "estimate", revision: 1, name: "Estimate", type: "number" }] });
    const original = f.postJson.getMockImplementation()!;
    f.postJson.mockImplementation(async (path, payload) => path === "/multica/workspace/upsert" ? { status: "failed", code: "multica_workspace_property_value_invalid" } : original(path, payload));
    const result = await f.adapter.transport("/api/issues/issue-1/properties/estimate", init("PUT", { value: "not a number" }));
    expect(result.status).toBe(400);
    expect(await result.json()).toMatchObject({ code: "multica_workspace_property_value_invalid" });
    expect(f.collections.issues[0].revision).toBe(3);
  });
  it("stores preferences in a user-scoped resource, not saved views, and survives adapter reconstruction", async () => {
    const f = fixture({ issue_view_preferences: [{ id: "other", revision: 1, user_id: "other-user", scope_type: "my", scope_id: null, prefs: { hidden: ["other-view"], order: [] } }] });
    const empty = await (await f.adapter.transport("/api/issue-view-preferences?scope_type=my")).json();
    expect(IssueViewPreferenceSchema.safeParse(empty).success).toBe(true);
    expect(empty).toEqual({ scope_type: "my", scope_id: null, prefs: { hidden: [], order: [] }, updated_at: "" });
    const request = init("PUT", { scope_type: "my", prefs: { hidden: ["view-1"], order: ["view-2", "view-1"] } }, "preference-put");
    const written = await (await f.adapter.transport("/api/issue-view-preferences", request)).json();
    expect(IssueViewPreferenceSchema.safeParse(written).success).toBe(true);
    expect(written).toMatchObject({ revision: 1, updated_at: date, prefs: { hidden: ["view-1"], order: ["view-2", "view-1"] } });
    const restored = new MulticaApiAdapter(f.bridge);
    expect(await (await restored.transport("/api/issue-view-preferences?scope_type=my")).json()).toEqual(written);
    expect(await (await restored.transport("/api/issue-view-preferences", request)).json()).toEqual(written);
    expect(f.collections.issue_views).toBeUndefined();
    expect(f.postJson.mock.calls.filter(([path]) => path === "/multica/workspace/upsert")).toHaveLength(1);
    expect((await f.adapter.transport("/api/issue-view-preferences", init("PUT", { scope_type: "project", prefs: { hidden: [], order: [] } }))).status).toBe(400);
    expect((await f.adapter.transport("/api/issue-view-preferences", init("PUT", { scope_type: "my", prefs: { hidden: [], order: ["duplicate", "duplicate"] } }))).status).toBe(400);
  });
});

describe("collaboration and quick action metadata", () => {
  it("persists subscriber identity and removes only the selected issue subscription", async () => {
    const f = fixture({ issues: [issue("issue-1"), issue("issue-2")] });
    expect((await f.adapter.transport("/api/issues/issue-1/subscribe", init("POST", {}, "subscribe-1"))).status).toBe(204);
    expect((await f.adapter.transport("/api/issues/issue-2/subscribe", init("POST"))).status).toBe(204);
    const listed = await (await f.adapter.transport("/api/issues/issue-1/subscribers")).json();
    expect(SubscribersListSchema.safeParse(listed).success).toBe(true);
    expect(listed).toMatchObject([{ issue_id: "issue-1", user_id: "user-1", user_type: "member", reason: "manual", created_at: date }]);
    expect((await new MulticaApiAdapter(f.bridge).transport("/api/issues/issue-1/subscribe", init("POST", {}, "subscribe-1"))).status).toBe(204);
    expect(f.collections.subscribers).toHaveLength(2);
    const remove = init("POST", {}, "unsubscribe-1");
    expect((await f.adapter.transport("/api/issues/issue-1/unsubscribe", remove)).status).toBe(204);
    expect((await new MulticaApiAdapter(f.bridge).transport("/api/issues/issue-1/unsubscribe", remove)).status).toBe(204);
    expect(f.collections.subscribers).toHaveLength(1);
    expect(f.collections.subscribers[0].issue_id).toBe("issue-2");
  });
  it("binds reactions to the real issue or comment and current actor with stable replay", async () => {
    const f = fixture({ issues: [issue("issue-1")], comments: [{ id: "comment-1", issue_id: "issue-1", revision: 1 }] });
    for (const path of ["/api/issues/issue-1/reactions", "/api/comments/comment-1/reactions"]) {
      const request = init("POST", { emoji: "+1" }, path.includes("comments") ? "comment-like" : "issue-like");
      const saved = await (await f.adapter.transport(path, request)).json();
      expect(saved).toMatchObject({ actor_id: "user-1", actor_type: "member", emoji: "+1", created_at: date });
      expect(await (await new MulticaApiAdapter(f.bridge).transport(path, request)).json()).toEqual(saved);
      expect((await f.adapter.transport(path, init("DELETE", { emoji: "+1" }))).status).toBe(204);
    }
    expect(f.collections.reactions).toEqual([]);
    expect((await f.adapter.transport("/api/issues/issue-1/reactions", init("POST", { emoji: "x", actor_id: "other" }))).status).toBe(400);
  });
  it("persists collaborator DTOs but leaves creator enforcement to Core", async () => {
    const f = fixture({ autopilots: [autopilot({ collaborators: [] })] });
    const request = init("POST", { user_id: "user-1" }, "grant-access");
    const saved = await (await f.adapter.transport("/api/autopilots/auto-1/collaborators", request)).json();
    expect(saved).toMatchObject({ collaborators: [{ user_type: "member", user_id: "user-1", granted_by: "user-1", created_at: expect.any(String) }] });
    expect(await (await new MulticaApiAdapter(f.bridge).transport("/api/autopilots/auto-1/collaborators", request)).json()).toEqual(saved);
    expect(await (await f.adapter.transport("/api/autopilots/auto-1/collaborators/user-1", init("DELETE"))).json()).toEqual({ collaborators: [] });
    const original = f.postJson.getMockImplementation()!;
    f.postJson.mockImplementation(async (path, payload) => path === "/multica/workspace/upsert" ? { status: "failed", code: "multica_workspace_autopilot_access_denied" } : original(path, payload));
    expect((await f.adapter.transport("/api/autopilots/auto-1/collaborators", init("POST", { user_id: "user-1" }))).status).toBe(403);
    expect(f.collections.autopilots[0].collaborators).toEqual([]);
  });
  it("round trips quick actions without inventing usage or dispatching during catalog writes", async () => {
    const f = fixture();
    const created = await (await f.adapter.transport("/api/quick-actions", init("POST", { name: "Review", assignee_type: "agent", assignee_id: "agent-1", prompt: "Review this issue", visibility: "private" }, "review-action"))).json();
    expect(QuickActionSchema.safeParse(created).success).toBe(true);
    expect(created).toMatchObject({ created_by_id: "user-1", use_count: 0, last_used_at: null, revision: 1 });
    expect((await f.adapter.transport("/api/quick-actions/review-action", init("PATCH", { status: "archived" }))).status).toBe(200);
    expect(await (await f.adapter.transport("/api/quick-actions")).json()).toMatchObject({ quick_actions: [] });
    expect(await (await f.adapter.transport("/api/quick-actions?include_archived=true")).json()).toMatchObject({ quick_actions: [{ id: "review-action", status: "archived" }] });
    expect((await f.adapter.transport("/api/quick-actions/review-action", init("DELETE"))).status).toBe(204);
    expect(f.postJson.mock.calls.some(([path]) => path.startsWith("/multica/executions/"))).toBe(false);
    expect((await f.adapter.transport("/api/quick-actions", init("POST", { name: "Bad", assignee_type: "agent", assignee_id: "agent-1", prompt: "{{title}}" }))).status).toBe(400);
  });
});

describe("explicit missing backend capability errors", () => {
  it.each([
    ["GET", "/api/agents/agent-1/env"],
    ["GET", "/api/autopilots/usage"],
    ["GET", "/api/issues/limit-usage"],
    ["POST", "/api/issues/preview-trigger"],
    ["POST", "/api/runtimes/native-runtime/models"],
  ])("returns capability_unavailable for %s %s, never fabricated success", async (method, path) => {
    const f = fixture();
    const result = await f.adapter.transport(path, method === "GET" ? undefined : init(method));
    expect(result.status).toBe(503);
    expect(await result.json()).toEqual({ code: "capability_unavailable", error: "capability_unavailable" });
    expect(f.postJson.mock.calls[0][0]).toBe("/multica/workspace/bootstrap");
    expect(f.postJson.mock.calls.some(([bridgePath]) => bridgePath === "/multica/workspace/upsert" || bridgePath === "/multica/executions/create")).toBe(false);
  });
});

describe("primary flows with actual Core projection defaults", () => {
  it.each(["private", "workspace"] as const)("preserves the original manual Agent %s payload and assignment contract", async (permissionScope) => {
    const f = fixture({ runtimes: [coreRuntime] });
    const runtimes = await (await f.adapter.transport("/api/runtimes?owner=me")).json();
    const payload = buildCreateAgentRequest({ draft: { ...EMPTY_AGENT_DRAFT, name: "Manual agent", permissionScope }, runtimeId: runtimes[0].id });
    const wire = JSON.parse(JSON.stringify(payload));
    const access = { permission_mode: permissionScope === "private" ? "private" : "public_to", invocation_targets: permissionScope === "private" ? [] : [{ target_type: "workspace" }] };
    expect(wire).toEqual({ name: "Manual agent", description: "", runtime_id: "native-runtime", skill_ids: [], ...access });
    const result = await f.adapter.transport("/api/agents", init("POST", wire, "agent-created"));
    expect(result.status).toBe(200);
    expect(await result.json()).toMatchObject({ id: "agent-created", runtime_id: "native-runtime", ...access });
    expect(f.postJson.mock.calls.at(-1)).toEqual(["/multica/agents/create", { entity: expect.objectContaining({ id: "agent-created", owner_id: "user-1", ...access }), skills: [], commandId: "agent-created", commandSignature: expect.stringMatching(/^[a-f0-9]{64}$/) }]);
    const assigned = await f.adapter.transport("/api/issues", init("POST", { title: "Assigned task", assignee_type: "agent", assignee_id: "agent-created" }));
    expect(assigned.status).toBe(200);
    expect(f.postJson.mock.calls.at(-1)?.[1]).toMatchObject({ resource: "issues", expectedRevision: 0, entity: { assignee_type: "agent", assignee_id: "agent-created" } });
  });
  it("offers the current Host and a single ordinary local member without server identity fields", async () => {
    const f = fixture({ runtimes: [coreRuntime] });
    const runtimes = await (await f.adapter.transport("/api/runtimes?owner=me")).json();
    expect(runtimes).toEqual([{ ...coreRuntime, name: "Codex", runtime_mode: "local", status: "online", owner_id: "user-1", metadata: { nativeTaskHostSupported: true, nativeModelSelectionAuthoritative: true, nativeHandoffSupported: true } }]);
    const members = await (await f.adapter.transport("/api/workspaces/workspace-1/members")).json();
    expect(members).toEqual([{ id: "user-1", user_id: "user-1", workspace_id: "workspace-1", role: "member", name: "", email: "", avatar_url: null, created_at: "" }]);
    expect(coreBootstrap).not.toHaveProperty("members");
    expect(coreRuntime).not.toHaveProperty("owner_id");
  });
  it("propagates Core agent permission errors without converting access to execution rights", async () => {
    const f = fixture({ runtimes: [coreRuntime] });
    const original = f.postJson.getMockImplementation()!;
    f.postJson.mockImplementation(async (path, payload) => path === "/multica/agents/create" ? { status: "failed", code: "multica_workspace_agent_permission_invalid" } : original(path, payload));
    const runtimes = await (await f.adapter.transport("/api/runtimes?owner=me")).json();
    const result = await f.adapter.transport("/api/agents", init("POST", { name: "Manual", runtime_id: runtimes[0].id, permission_mode: "private", invocation_targets: [], skill_ids: [], template: "blank" }));
    expect(result.status).toBe(403);
    expect(await result.json()).toMatchObject({ code: "multica_workspace_agent_permission_invalid" });
    expect(f.postJson.mock.calls.at(-1)?.[1]).toMatchObject({ entity: { runtime_id: "native-runtime", permission_mode: "private" }, skills: [] });
  });
  it("creates a plain issue despite missing property catalog, assigns it, then opens the Core-linked task", async () => {
    const f = fixture();
    f.executions.length = 0;
    const original = f.postJson.getMockImplementation()!;
    f.postJson.mockImplementation(async (path, payload) => {
      if (path === "/multica/workspace/query" && payload.resource === "properties") return { status: "failed", code: "capability_unavailable" };
      const result = await original(path, payload);
      if (path === "/multica/workspace/upsert" && payload.resource === "issues" && (payload.entity as JsonRecord).assignee_type === "agent") {
        f.executions.push(binding({ issueId: (payload.entity as JsonRecord).id }));
        return { ...(result as JsonRecord), queue: { binding_id: "binding-1", status: "running", replay: false } };
      }
      return result;
    });
    expect((await f.adapter.transport("/api/properties")).status).toBe(503);
    const created = await (await f.adapter.transport("/api/issues", init("POST", { title: "Plain issue" }, "issue-1"))).json();
    expect(IssueSchema.safeParse(created).success).toBe(true);
    expect(created).toMatchObject({ status: "todo", priority: "none", assignee_type: null, assignee_id: null, metadata: {}, properties: {}, created_at: date, updated_at: date });
    const assigned = await f.adapter.transport("/api/issues/issue-1", init("PUT", { assignee_type: "agent", assignee_id: "agent-1" }));
    expect(assigned.status).toBe(200);
    expect(f.postJson.mock.calls.at(-1)?.[1]).toMatchObject({ expectedRevision: 1, entity: { assignee_type: "agent", assignee_id: "agent-1" } });
    expect(f.postJson.mock.calls.some(([path]) => path === "/multica/executions/create")).toBe(false);
    expect(await (await f.adapter.transport("/api/issues/issue-1/active-task")).json()).toMatchObject({ tasks: [{ id: "binding-1", thread_id: "native-thread" }] });
    expect((await f.adapter.transport("/api/tasks/binding-1/open", init("POST"))).status).toBe(200);
    expect(f.bridge.openThread).toHaveBeenCalledWith("native-thread");
  });
  it("maps minimal Core entities and prioritizes authoritative store timestamps", async () => {
    const stored = { revision: 1, workspace_id: "workspace-1", created_at_ms: Date.parse(date), updated_at_ms: Date.parse(date), updated_at: "2000-01-01T00:00:00Z" };
    const f = fixture({ issues: [{ ...stored, id: "issue-1", title: "Stored" }], agents: [{ ...stored, id: "agent-1", name: "Stored agent", permission_mode: "plan" }], autopilots: [{ ...stored, id: "auto-1", title: "Stored automation", assignee_id: "agent-1" }] });
    f.executions.length = 0;
    const issueResult = await (await f.adapter.transport("/api/issues/issue-1")).json();
    expect(IssueSchema.safeParse(issueResult).success).toBe(true);
    expect(issueResult).toMatchObject({ created_at: date, updated_at: date, status: "todo", priority: "none" });
    expect(await (await f.adapter.transport("/api/agents/agent-1")).json()).toMatchObject({ permission_mode: "plan", owner_id: null, updated_at: date });
    expect(await (await f.adapter.transport("/api/autopilots/auto-1")).json()).toMatchObject({ autopilot: { assignee_type: "agent", status: "active", execution_mode: "create_issue", updated_at: date }, triggers: [] });
  });
});

describe("original Agent quick-create contract", () => {
  const agent = { id: "agent-1", name: "Agent", revision: 1, runtime_id: "native-runtime", permission_mode: "private", owner_id: "user-1" };
  it("returns the actual assignment task id and replays three requests with one write", async () => {
    const f = fixture({ agents: [agent] });
    const request = { agent_id: "agent-1", prompt: "Review this change\nPreserve the full prompt", priority: "high", due_date: "2026-10-01", project_id: null, parent_issue_id: null };
    for (let n = 0; n < 3; n++) {
      const result = await f.adapter.transport("/api/issues/quick-create", init("POST", request, "issue-1"));
      expect(result.status).toBe(202);
      expect(await result.json()).toEqual({ task_id: "binding-1" });
    }
    expect(f.collections.issues[0]).toMatchObject({ id: "issue-1", title: "Review this change", description: request.prompt, priority: "high", due_date: "2026-10-01", assignee_type: "agent", assignee_id: "agent-1", project_id: null, parent_issue_id: null });
    expect(f.postJson.mock.calls.filter(([path]) => path === "/multica/workspace/upsert")).toHaveLength(1);
    expect(f.postJson.mock.calls.some(([path]) => path === "/multica/executions/create")).toBe(false);
    expect(await (await f.adapter.transport("/api/agents/agent-1/tasks")).json()).toMatchObject([{ id: "binding-1", issue_id: "issue-1" }]);
    const replay = await new MulticaApiAdapter(f.bridge).transport("/api/issues/quick-create", init("POST", request, "issue-1"));
    expect(replay.status).toBe(202);
    expect(await replay.json()).toEqual({ task_id: "binding-1" });
    expect(f.postJson.mock.calls.filter(([path]) => path === "/multica/workspace/upsert")).toHaveLength(1);
  });
  it("reports an already persisted issue when no assignment binding exists", async () => {
    const f = fixture({ agents: [agent] });
    f.executions.length = 0;
    const result = await f.adapter.transport("/api/issues/quick-create", init("POST", { agent_id: "agent-1", prompt: "Task" }, "issue-1"));
    expect(result.status).toBe(503);
    expect(await result.json()).toMatchObject({ code: "assignment_binding_unavailable", issue_id: "issue-1" });
    expect(f.collections.issues).toHaveLength(1);
  });
  it("does not present failed native execution as accepted", async () => {
    const f = fixture({ agents: [agent] });
    f.executions[0].state = "failed";
    const result = await f.adapter.transport("/api/issues/quick-create", init("POST", { agent_id: "agent-1", prompt: "Task" }, "issue-1"));
    expect(result.status).toBe(503);
    expect(await result.json()).toMatchObject({ issue_id: "issue-1", task_id: "binding-1", code: "assignment_failed" });
  });
  it.each([
    [{ prompt: "Task" }, 400],
    [{ prompt: "", agent_id: "agent-1" }, 400],
    [{ prompt: "Task", agent_id: "agent-1", squad_id: "squad-1" }, 400],
    [{ prompt: "Task", squad_id: "squad-1" }, 503],
    [{ prompt: "Task", agent_id: "agent-1", attachment_ids: ["upload-1"] }, 503],
    [{ prompt: "Task", agent_id: "agent-1", priority: "invalid" }, 400],
  ] as const)("rejects unsupported or invalid quick-create before persisting %j", async (request, status) => {
    const f = fixture({ agents: [agent] });
    expect((await f.adapter.transport("/api/issues/quick-create", init("POST", request))).status).toBe(status);
    expect(f.postJson.mock.calls.some(([path]) => path === "/multica/workspace/upsert")).toBe(false);
  });
});

describe("Core-backed issue status writes", () => {
  const builtin = { id: "status-todo", key: "todo", name: "Todo", category: "todo", color: "#2563eb", description: "", is_system: true, position: 0, archived_at: null, revision: 1 };
  it("returns a validation error for empty status creation, not a capability error", async () => {
    const f = fixture();
    const result = await f.adapter.transport("/api/issue-statuses", init("POST"));
    expect(result.status).toBe(400);
    expect(await result.json()).toMatchObject({ code: "invalid_mutation" });
    expect(f.postJson.mock.calls.some(([path]) => path === "/multica/workspace/upsert")).toBe(false);
  });
  it("creates, renames and archives a custom status using CAS, never physical deletion", async () => {
    const f = fixture({ issue_statuses: [builtin] });
    const created = await f.adapter.transport("/api/issue-statuses", init("POST", { name: "Ready Review", category: "todo", color: "#AABBCC" }, "custom-status"));
    expect(created.status).toBe(201);
    expect(await created.json()).toMatchObject({ key: "ready_review", color: "#aabbcc", is_system: false, position: 1, revision: 1 });
    expect(await (await f.adapter.transport("/api/issue-statuses/custom-status", init("PATCH", { name: "Ready", expected_revision: 1 }))).json()).toMatchObject({ key: "ready_review", name: "Ready", revision: 2 });
    expect(await (await f.adapter.transport("/api/issue-statuses/custom-status", init("DELETE", { expected_revision: 2 }))).json()).toMatchObject({ id: "custom-status", archived_at: expect.any(String), revision: 3 });
    expect(f.collections.issue_statuses).toHaveLength(2);
    expect(f.postJson.mock.calls.some(([path]) => path === "/multica/workspace/delete")).toBe(false);
    expect(await (await f.adapter.transport("/api/issue-statuses")).json()).toMatchObject({ total: 1 });
    expect(await (await f.adapter.transport("/api/issue-statuses?include_archived=true")).json()).toMatchObject({ total: 2 });
  });
  it("derives a category ordinal for non-Latin names without reusing archived keys", async () => {
    const f = fixture({ issue_statuses: [builtin, { ...builtin, id: "old", key: "todo_2", is_system: false, archived_at: date }] });
    const result = await f.adapter.transport("/api/issue-statuses", init("POST", { name: "\u5f85\u5ba2\u6237", category: "todo", color: "#aabbcc" }));
    expect(await result.json()).toMatchObject({ key: "todo_3" });
  });
  it("rejects system changes, immutable custom fields and unknown reorder IDs without writes", async () => {
    const f = fixture({ issue_statuses: [builtin] });
    expect((await f.adapter.transport("/api/issue-statuses/status-todo", init("DELETE"))).status).toBe(403);
    expect((await f.adapter.transport("/api/issue-statuses/status-todo", init("PATCH", { name: "Changed" }))).status).toBe(403);
    expect((await f.adapter.transport("/api/issue-statuses/custom", init("PATCH", { category: "done" }))).status).toBe(400);
    expect((await f.adapter.transport("/api/issue-statuses/reorder", init("PATCH", { category: "todo", ids: ["custom"] }))).status).toBe(409);
    expect(f.postJson.mock.calls.some(([path]) => path === "/multica/workspace/upsert")).toBe(false);
  });
  it("sends one atomic category reorder with every observed revision and replays its receipt", async () => {
    const custom = { ...builtin, id: "custom", key: "ready", is_system: false };
    const f = fixture({ issue_statuses: [builtin, custom] });
    const original = f.postJson.getMockImplementation()!;
    f.postJson.mockImplementation(async (path, payload) => {
      if (path !== "/multica/workspace/reorder-statuses") return original(path, payload);
      expect(payload).toMatchObject({ category: "todo", ids: ["status-todo", "custom"], expectedRevisions: { custom: 1, "status-todo": 1 }, commandSignature: expect.stringMatching(/^[a-f0-9]{64}$/) });
      const statuses = [builtin, custom].map((row, index) => ({ ...row, position: index, revision: 2 }));
      const result = { status: "ok", statuses };
      f.collections.issue_statuses = statuses;
      f.receipts.set(String(payload.commandId), { signature: payload.commandSignature, result });
      return result;
    });
    const request = init("PATCH", { category: "todo", ids: ["custom"] }, "reorder");
    const result = await f.adapter.transport("/api/issue-statuses/reorder", request);
    expect(result.status).toBe(200);
    expect(ListIssueStatusesResponseSchema.safeParse(await result.json()).success).toBe(true);
    expect((await new MulticaApiAdapter(f.bridge).transport("/api/issue-statuses/reorder", request)).status).toBe(200);
    expect(f.postJson.mock.calls.filter(([path]) => path === "/multica/workspace/reorder-statuses")).toHaveLength(1);
    expect(f.postJson.mock.calls.some(([path]) => path === "/multica/workspace/upsert")).toBe(false);
  });
  it("preserves stale revisions and Core permission failures", async () => {
    const custom = { ...builtin, id: "custom", key: "ready", is_system: false };
    const f = fixture({ issue_statuses: [builtin, custom] });
    await f.adapter.transport("/api/issue-statuses");
    f.collections.issue_statuses[1].revision = 2;
    expect((await f.adapter.transport("/api/issue-statuses/custom", init("PATCH", { name: "Changed" }))).status).toBe(409);
    f.postJson.mockResolvedValueOnce({ status: "failed", code: "permission_denied" });
    expect((await f.adapter.transport("/api/issue-statuses/custom", init("PATCH", { name: "Other", expected_revision: 2 }))).status).toBe(403);
  });
});

describe("bounded full-window queries", () => {
  it("filters before pagination, finds ids beyond row 100, and returns the filtered total", async () => {
    const f = fixture({ issues: Array.from({ length: 205 }, (_, i) => issue(`issue-${i}`, { position: i, priority: i >= 100 ? "high" : "low", creator_id: i % 2 ? "user-1" : "other" })) });
    const result = await f.adapter.transport("/api/issues?priority=high&creator_id=user-1&limit=2&offset=1&sort=position&direction=desc");
    expect(await result.json()).toMatchObject({ issues: [{ id: "issue-201" }, { id: "issue-199" }], total: 52 });
    expect(await (await f.adapter.transport("/api/issues/issue-204")).json()).toMatchObject({ id: "issue-204" });
    expect(f.postJson.mock.calls.filter(([path, p]) => path.endsWith("/query") && p.resource === "issues").map(([, p]) => p.offset)).toEqual([0, 100, 200, 0, 100, 200]);
  });
  it("handles the upstream POST query twin, including present-but-empty ids", async () => {
    const f = fixture({ issues: [issue("issue-1"), issue("issue-2")] });
    expect(await (await f.adapter.transport("/api/issues/query", init("POST", { ids: "", limit: "50" }))).json()).toEqual({ issues: [], total: 0 });
    expect(await (await f.adapter.transport("/api/issues/query", init("POST", { ids: "issue-2", limit: "50" }))).json()).toMatchObject({ issues: [{ id: "issue-2" }], total: 1 });
  });
  it("returns schema-valid search hits", async () => {
    const f = fixture({ issues: [issue("one")] });
    const result = await (await f.adapter.transport("/api/issues/search?q=Task")).json();
    expect(SearchIssuesResponseSchema.safeParse(result).success).toBe(true);
    expect(result).toMatchObject({ total: 1, issues: [{ match_source: "title" }] });
  });
  it("filters category, actor, labels, root, metadata and date fields", async () => {
    const f = fixture({ issues: [issue("issue-1", { status: "custom-review", status_category: "in_review", labels: [{ id: "label-1" }], metadata: { stage: "ready" } }), issue("issue-2", { parent_issue_id: "issue-1" })] });
    f.executions.length = 0;
    const query = new URLSearchParams({ status_category: "in_review", assignee_filters: "member:user-1", label_ids: "label-1", top_level_only: "true", metadata: '{"stage":"ready"}', date_start: "2026-09-17", date_end: "2026-09-19" });
    expect(await (await f.adapter.transport(`/api/issues?${query}`)).json()).toMatchObject({ total: 1, issues: [{ id: "issue-1" }] });
  });
  it.each(["limit=-1", "limit=NaN", "offset=1.5", "offset=Infinity", "limit=0", "limit=1&limit=2"])("rejects malformed pagination %s", async (query) => {
    expect((await fixture().adapter.transport(`/api/issues?${query}`)).status).toBe(400);
  });
  it("rejects truncated, oversized and changing collection snapshots", async () => {
    for (const payload of [{ items: [], total: 1 }, { items: [], total: 5001 }, { total: 0 }, { items: [], total: 0, stale: true }]) {
      const f = fixture();
      await f.adapter.transport("/api/me");
      f.postJson.mockResolvedValueOnce(payload);
      expect((await f.adapter.transport("/api/issues")).ok).toBe(false);
    }
  });
});

describe("original IssueSurface table contracts", () => {
  const query = { scope: { kind: "my", relation: "assigned" }, filters: {}, sort: { field: "position", direction: "asc" } };
  it("returns validated groups, hierarchy rows and snapshot-bound cursors", async () => {
    const f = fixture({ issues: [issue("a", { position: 1 }), issue("b", { position: 2 }), issue("c", { position: 3, parent_issue_id: "a" }), issue("d", { assignee_id: "other" })] });
    const groups = await (await f.adapter.transport("/api/issues/table/groups", init("POST", { query, group: { kind: "status_category" } }))).json();
    expect(IssueTableGroupsResponseSchema.safeParse(groups).success).toBe(true);
    expect(groups).toMatchObject({ total: 3, groups: [{ key: "status_category:todo", count: 3 }] });
    const body = { query, group: { kind: "status_category" }, group_key: "status_category:todo", parent_id: null, hierarchy: { enabled: true }, page: { limit: 1 } };
    const first = await (await f.adapter.transport("/api/issues/table/rows", init("POST", body))).json();
    expect(IssueTableRowsResponseSchema.safeParse(first).success).toBe(true);
    expect(first).toMatchObject({ total: 3, branch_total: 2, rows: [{ issue: { id: "a" }, direct_child_count: 1 }] });
    const next = { ...body, page: { limit: 1, cursor: first.next_cursor } };
    expect(await (await f.adapter.transport("/api/issues/table/rows", init("POST", next))).json()).toMatchObject({ rows: [{ issue: { id: "b" } }], next_cursor: null });
    f.collections.issues[0].revision = 4;
    expect((await f.adapter.transport("/api/issues/table/rows", init("POST", next))).status).toBe(409);
  });
  it("computes real facet counts and compounds", async () => {
    const f = fixture({ issues: [issue("a"), issue("b", { status: "done" })] });
    const facets = await (await f.adapter.transport("/api/issues/table/facets", init("POST", { query, facets: [{ kind: "status" }, { kind: "working_agents" }] }))).json();
    expect(facets).toMatchObject({ total: 2, facets: [{ kind: "status", values: [{ key: "todo", count: 1 }, { key: "done", count: 1 }] }, { kind: "working_agents", values: [] }] });
    const groups = await (await f.adapter.transport("/api/issues/table/groups", init("POST", { query, group: { kind: "compound", primary: "assignee", secondary: "status_category" } }))).json();
    expect(groups.groups[0].secondary_groups).toMatchObject([{ key: "compound:assignee:member:user-1:status_category:todo" }, { key: "compound:assignee:member:user-1:status_category:done" }]);
  });
  it("treats empty facets as unrestricted and drops only the facet's own dimension", async () => {
    const f = fixture({ issues: [issue("a"), issue("b", { status: "done" }), issue("child", { parent_issue_id: "a" })] });
    const request = { query: { ...query, filters: { statuses: ["todo"], priorities: [], include_sub_issues: false } }, facets: [{ kind: "status" }, { kind: "priority" }] };
    const result = await (await f.adapter.transport("/api/issues/table/facets", init("POST", request))).json();
    expect(result).toMatchObject({ total: 1, facets: [{ kind: "status", values: [{ key: "todo", count: 1 }, { key: "done", count: 1 }] }, { kind: "priority", values: [{ key: "none", count: 1 }] }] });
  });
});

describe("CRUD, CAS and idempotency", () => {
  it("uses separate durable commands for autopilot create and trigger create, replaying only the trigger result", async () => {
    const f = fixture();
    const original = f.postJson.getMockImplementation()!;
    f.postJson.mockImplementation(async (path, payload) => path === "/multica/webhooks/provision" ? { id: payload.triggerId, autopilot_id: payload.autopilotId, kind: "api", enabled: true, credential_revision: 1, webhook_token: null, created_at: date, updated_at: date } : original(path, payload));
    const create = init("POST", { title: "Automation", assignee_id: "agent-1", execution_mode: "run_only" }, "auto-key");
    const trigger = init("POST", { kind: "api" }, "trigger-key");
    expect((await f.adapter.transport("/api/autopilots", create)).status).toBe(200);
    expect(await (await f.adapter.transport("/api/autopilots/auto-key/triggers", trigger)).json()).toMatchObject({ id: "trigger-key", kind: "api" });
    expect(await (await new MulticaApiAdapter(f.bridge).transport("/api/autopilots/auto-key/triggers", trigger)).json()).toMatchObject({ id: "trigger-key", kind: "api" });
    expect(f.postJson.mock.calls.filter(([path]) => path === "/multica/workspace/upsert")).toHaveLength(2);
    expect(f.collections.autopilots[0].triggers).toHaveLength(1);
    expect((await new MulticaApiAdapter(f.bridge).transport("/api/autopilots/auto-key/triggers", init("POST", { kind: "api" }, "auto-key"))).status).toBe(409);
  });
  it("keeps attach/detach receipts separate and returns the original saved label result on replay", async () => {
    const f = fixture({ issues: [issue("issue-1")], labels: [{ id: "label-1", name: "Label", revision: 1, resource_type: "issue" }] });
    const attach = init("POST", { label_id: "label-1" }, "attach-key");
    const detach = init("DELETE", {}, "detach-key");
    const first = await (await f.adapter.transport("/api/issues/issue-1/labels", attach)).json();
    expect((await f.adapter.transport("/api/issues/issue-1/labels/label-1", detach)).status).toBe(200);
    expect(await (await new MulticaApiAdapter(f.bridge).transport("/api/issues/issue-1/labels", attach)).json()).toEqual(first);
    expect(f.collections.issues[0].label_ids).toEqual([]);
    expect(f.postJson.mock.calls.filter(([path]) => path === "/multica/workspace/upsert")).toHaveLength(2);
  });
  it("replays each batch step independently after recreation using stable distinct ledger keys", async () => {
    const f = fixture({ issues: [issue("a"), issue("b")] });
    const request = init("POST", { issue_ids: ["a", "b"], updates: { status: "done" } }, "batch-key");
    expect((await f.adapter.transport("/api/issues/batch-update", request)).status).toBe(200);
    const writes = f.postJson.mock.calls.filter(([path]) => path === "/multica/workspace/upsert").map(([, payload]) => payload);
    expect(writes).toHaveLength(2);
    expect(new Set(writes.map((payload) => payload.commandId)).size).toBe(2);
    expect(new Set(writes.map((payload) => payload.commandSignature)).size).toBe(2);
    expect(writes.every((payload) => payload.commandId !== "batch-key")).toBe(true);
    expect(await (await new MulticaApiAdapter(f.bridge).transport("/api/issues/batch-update", request)).json()).toEqual({ updated: 2 });
    expect(f.postJson.mock.calls.filter(([path]) => path === "/multica/workspace/upsert")).toHaveLength(2);
    expect(f.collections.issues.map((item) => item.revision)).toEqual([4, 4]);
    const deletes = init("POST", { issue_ids: ["a", "b"] }, "batch-delete-key");
    expect((await f.adapter.transport("/api/issues/batch-delete", deletes)).status).toBe(200);
    expect(await (await new MulticaApiAdapter(f.bridge).transport("/api/issues/batch-delete", deletes)).json()).toEqual({ deleted: 2 });
    expect(f.postJson.mock.calls.filter(([path]) => path === "/multica/workspace/delete")).toHaveLength(2);
  });
  it("recovers delete results after entity removal and rejects changed cross-refresh payloads", async () => {
    const f = fixture({ issues: [issue("a")] });
    expect((await f.adapter.transport("/api/issues/a", init("DELETE", {}, "delete-key"))).status).toBe(204);
    expect((await new MulticaApiAdapter(f.bridge).transport("/api/issues/a", init("DELETE", {}, "delete-key"))).status).toBe(204);
    const conflict = await new MulticaApiAdapter(f.bridge).transport("/api/issues", init("POST", { title: "Different" }, "delete-key"));
    expect(conflict.status).toBe(409);
    expect(f.postJson.mock.calls.filter(([path]) => path === "/multica/workspace/delete")).toHaveLength(1);
  });
  it("stops before writing when receipt lookup fails or is malformed", async () => {
    for (const lookup of [{ status: "failed", code: "runtime_unavailable" }, { status: "ok" }, { status: "ok", found: true, result: null }]) {
      const f = fixture();
      await f.adapter.transport("/api/me");
      f.postJson.mockResolvedValueOnce(lookup);
      expect((await f.adapter.transport("/api/issues", init("POST", { title: "Task" }))).ok).toBe(false);
      expect(f.postJson.mock.calls.some(([path]) => path === "/multica/workspace/upsert")).toBe(false);
    }
  });
  it("recovers a persisted create receipt after adapter recreation without another write", async () => {
    const f = fixture();
    const request = { title: "Persisted command" };
    expect((await f.adapter.transport("/api/issues", init("POST", request, "durable-key"))).status).toBe(200);
    const first = f.postJson.mock.calls.find(([path]) => path === "/multica/workspace/upsert")![1];
    const fresh = new MulticaApiAdapter(f.bridge);
    expect((await fresh.transport("/api/issues", init("POST", request, "durable-key"))).status).toBe(200);
    const second = f.postJson.mock.calls.at(-1)![1];
    expect(first).toMatchObject({ commandId: "durable-key", commandSignature: expect.stringMatching(/^[a-f0-9]{64}$/) });
    expect(second.commandSignature).toBe(first.commandSignature);
    expect(second.commandId).toBe(first.commandId);
    expect(f.postJson.mock.calls.filter(([path]) => path === "/multica/workspace/upsert")).toHaveLength(1);
  });
  it("replays a create three times without duplicate bridge writes", async () => {
    const f = fixture();
    const results = await Promise.all(Array.from({ length: 3 }, () => f.adapter.transport("/api/issues", init("POST", { title: "Created" }, "create-command"))));
    expect(await Promise.all(results.map((r) => r.json()))).toEqual(Array(3).fill(expect.objectContaining({ id: "create-command", revision: 1 })));
    expect(f.postJson.mock.calls.filter(([path]) => path.endsWith("/upsert"))).toHaveLength(1);
    expect((await f.adapter.transport("/api/issues", init("POST", { title: "Different" }, "create-command"))).status).toBe(409);
  });
  it("uses the observed revision and does not refresh away stale edits", async () => {
    const f = fixture({ issues: [issue("issue-1")] });
    await f.adapter.transport("/api/issues");
    f.collections.issues[0].revision = 4;
    const result = await f.adapter.transport("/api/issues/issue-1", init("PUT", { title: "Edit" }));
    expect(result.status).toBe(409);
    expect(await result.json()).toMatchObject({ code: "multica_workspace_revision_conflict", current_revision: 4 });
    expect(f.postJson.mock.calls.at(-1)?.[1].expectedRevision).toBe(3);
  });
  it("preserves explicit move nulls, revisions and delete semantics", async () => {
    const f = fixture({ issues: [issue("issue-1")] });
    const result = await f.adapter.transport("/api/issues/issue-1/move", init("POST", { status: "done", assignee_id: null, assignee_type: null, before_id: null, after_id: null, expected_revision: 3 }));
    expect(result.status).toBe(200);
    expect(f.postJson.mock.calls.at(-1)).toEqual(["/multica/workspace/move-issue", { issueId: "issue-1", status: "done", assigneeId: null, assigneeType: null, beforeId: null, afterId: null, expectedRevision: 3, commandId: expect.any(String), commandSignature: expect.stringMatching(/^[a-f0-9]{64}$/) }]);
    f.collections.issues[0].revision = 4;
    expect((await f.adapter.transport("/api/issues/issue-1", init("DELETE"))).status).toBe(204);
  });
  it.each(["agents", "autopilots", "issue-views", "labels"])("creates and updates %s through allowlisted resources", async (domain) => {
    const f = fixture();
    const data = domain === "autopilots" ? { title: "Automation", assignee_id: "agent-1", execution_mode: "run_only" } : { name: "Definition" };
    const created = await (await f.adapter.transport(`/api/${domain}`, init("POST", data, "new-item"))).json();
    expect(created).toMatchObject({ id: "new-item", revision: 1 });
    const patch = domain === "autopilots" ? { title: "Changed" } : { name: "Changed" };
    expect(await (await f.adapter.transport(`/api/${domain}/new-item`, init("PATCH", { ...patch, expected_revision: 1 }))).json()).toMatchObject({ revision: 2 });
  });
  it("accepts the original manual agent request including conversation starters", async () => {
    const f = fixture();
    const result = await f.adapter.transport("/api/agents", init("POST", { name: "Agent", description: "", runtime_id: "native-runtime", permission_mode: "private", invocation_targets: [], skill_ids: [], template: "blank", conversation_starters: [{ label: "Review", prompt: "Review this project" }] }));
    expect(result.status).toBe(200);
    expect(await result.json()).toMatchObject({ permission_mode: "private", conversation_starters: [{ label: "Review" }] });
    expect(f.postJson.mock.calls.at(-1)?.[0]).toBe("/multica/agents/create");
  });
  it("reports completed ids on a partial batch instead of claiming atomic success", async () => {
    const f = fixture({ issues: [issue("a"), issue("b")] });
    await f.adapter.transport("/api/issues");
    f.collections.issues[1].revision = 4;
    const result = await f.adapter.transport("/api/issues/batch-update", init("POST", { issue_ids: ["a", "b"], updates: { status: "done" } }));
    expect(result.status).toBe(409);
    expect(await result.json()).toMatchObject({ completed_ids: ["a"], failed_id: "b" });
    expect(f.collections.issues.find((item) => item.id === "a")).toMatchObject({ status: "done", revision: 4 });
    expect(f.collections.issues.find((item) => item.id === "b")).toMatchObject({ status: "todo", revision: 4 });
  });
  it("attaches and detaches issue labels through CAS", async () => {
    const f = fixture({ issues: [issue("issue-1")], labels: [{ id: "label-1", name: "Label", revision: 1, resource_type: "issue" }] });
    expect(await (await f.adapter.transport("/api/issues/issue-1/labels", init("POST", { label_id: "label-1" }))).json()).toMatchObject({ labels: [{ id: "label-1" }], issue_revision: 4 });
    expect(await (await f.adapter.transport("/api/issues/issue-1/labels/label-1", init("DELETE"))).json()).toEqual({ labels: [], issue_revision: 5 });
  });
});

describe("real native execution and automation", () => {
  it("exposes failed credential provisioning with the saved trigger ID and no fabricated token", async () => {
    const f = fixture();
    expect((await f.adapter.transport("/api/autopilots", init("POST", { title: "Webhook automation", assignee_id: "agent-1", execution_mode: "create_issue" }, "auto-1"))).status).toBe(200);
    const result = await f.adapter.transport("/api/autopilots/auto-1/triggers", init("POST", { kind: "webhook" }));
    expect(result.status).toBe(503);
    expect(await result.json()).toMatchObject({ code: "capability_unavailable", autopilot_id: "auto-1", trigger_saved: true, trigger_id: expect.any(String) });
    expect(f.collections.autopilots).toHaveLength(1);
    expect(f.collections.autopilots[0].triggers).toMatchObject([{ kind: "webhook", webhook_token: null }]);
    expect(f.postJson.mock.calls.filter(([path]) => path === "/multica/workspace/upsert")).toHaveLength(2);
  });
  it("uses Core's UTC schedule default and returns the persisted trigger", async () => {
    const f = fixture({ autopilots: [autopilot()] });
    const original = f.postJson.getMockImplementation()!;
    f.postJson.mockImplementation(async (path, payload) => {
      if (path === "/multica/autopilots/cron-preview") return { status: "ok", next_runs: ["2026-09-19T09:00:00Z"] };
      const result = await original(path, payload);
      if (path === "/multica/workspace/upsert") {
        const saved = (result as JsonRecord).entity as JsonRecord;
        (saved.triggers as JsonRecord[])[0].next_run_at = "2026-09-19T09:00:00Z";
      }
      return result;
    });
    const result = await f.adapter.transport("/api/autopilots/auto-1/triggers", init("POST", { kind: "schedule", cron_expression: "0 9 * * *" }));
    expect(result.status).toBe(200);
    expect(await result.json()).toMatchObject({ timezone: "UTC", enabled: true, next_run_at: "2026-09-19T09:00:00Z" });
    expect(f.postJson).toHaveBeenCalledWith("/multica/autopilots/cron-preview", { expr: "0 9 * * *", tz: "UTC" });
  });
  it.each([{}, { next_runs: null }, { next_runs: ["not-a-date"] }])("rejects unreadable Core schedule previews before saving: %j", async (preview) => {
    const f = fixture({ autopilots: [autopilot()] });
    const original = f.postJson.getMockImplementation()!;
    f.postJson.mockImplementation(async (path, payload) => path === "/multica/autopilots/cron-preview" ? { status: "ok", ...preview } : original(path, payload));
    expect((await f.adapter.transport("/api/autopilots/auto-1/triggers", init("POST", { kind: "schedule", cron_expression: "0 9 * * *" }))).status).toBe(502);
    expect(f.postJson.mock.calls.some(([path]) => path.endsWith("/upsert"))).toBe(false);
  });
  it("surfaces the missing cron-preview route and rejects malformed webhook filters", async () => {
    const f = fixture({ autopilots: [autopilot()] });
    expect((await f.adapter.transport("/api/autopilots/auto-1/triggers", init("POST", { kind: "schedule", cron_expression: "0 9 * * *" }))).status).toBe(503);
    expect((await f.adapter.transport("/api/autopilots/auto-1/triggers", init("POST", { kind: "api", event_filters: [{ event: "" }] }))).status).toBe(400);
    expect((await f.adapter.transport("/api/autopilots/auto-1/triggers", init("POST", { kind: "api", enabled: "false" }))).status).toBe(400);
    expect(f.postJson.mock.calls.some(([path]) => path.endsWith("/upsert"))).toBe(false);
  });
  it("validates schedule edits using Core cron preview, persists fields and deletes with CAS", async () => {
    const f = fixture({ autopilots: [autopilot()] });
    const original = f.postJson.getMockImplementation()!;
    f.postJson.mockImplementation(async (path, payload) => path === "/multica/autopilots/cron-preview" ? { status: "ok", next_runs: ["2026-09-19T09:00:00Z"] } : original(path, payload));
    const preview = await f.adapter.transport("/api/autopilots/cron-preview?expr=0+9+*+*+*&tz=Asia%2FShanghai");
    expect(await preview.json()).toEqual({ next_runs: ["2026-09-19T09:00:00Z"] });
    const created = await f.adapter.transport("/api/autopilots/auto-1/triggers", init("POST", { kind: "schedule", cron_expression: "0 9 * * *", timezone: "Asia/Shanghai", label: "Daily" }, "schedule-1"));
    expect(created.status).toBe(200);
    expect(await created.json()).toMatchObject({ id: "schedule-1", kind: "schedule", cron_expression: "0 9 * * *", timezone: "Asia/Shanghai", enabled: true });
    expect(f.collections.autopilots[0].revision).toBe(3);
    expect((await f.adapter.transport("/api/autopilots/auto-1/triggers/schedule-1", init("PATCH", { enabled: false, expected_revision: 3 }))).status).toBe(200);
    expect((await f.adapter.transport("/api/autopilots/auto-1/triggers/schedule-1", init("DELETE", { expected_revision: 4 }))).status).toBe(204);
    expect(f.collections.autopilots[0].triggers).toEqual([]);
  });
  it("does not save schedules when Core rejects cron or timezone", async () => {
    const f = fixture({ autopilots: [autopilot()] });
    const original = f.postJson.getMockImplementation()!;
    f.postJson.mockImplementation(async (path, payload) => path === "/multica/autopilots/cron-preview" ? { status: "failed", code: "autopilot_cron_invalid" } : original(path, payload));
    expect((await f.adapter.transport("/api/autopilots/auto-1/triggers", init("POST", { kind: "schedule", cron_expression: "bad", timezone: "bad" }))).status).toBe(400);
    expect(f.postJson.mock.calls.some(([path]) => path.endsWith("/upsert"))).toBe(false);
  });
  it("maps actual native bindings to the upstream AgentTask schema", async () => {
    const f = fixture();
    const tasks = await (await f.adapter.transport("/api/agents/agent-1/tasks")).json();
    expect(AgentTaskSchema.safeParse(tasks[0]).success).toBe(true);
    expect(tasks[0]).toMatchObject({ id: "binding-1", issue_id: "issue-1", runtime_id: "native-runtime", thread_id: "native-thread", revision: 4 });
    expect(taskDto(binding({ issueId: null })).issue_id).toBe("");
  });
  it("opens only the thread returned by Core and cancels with native CAS", async () => {
    const f = fixture();
    expect((await f.adapter.transport("/api/tasks/binding-1/open", init("POST"))).status).toBe(200);
    expect(f.bridge.openThread).toHaveBeenCalledWith("native-thread");
    const cancel = await f.adapter.transport("/api/issues/issue-1/tasks/binding-1/cancel", init("POST", {}, "cancel-command"));
    expect(await cancel.json()).toMatchObject({ status: "cancelled", revision: 5 });
    expect(f.postJson.mock.calls.at(-1)).toEqual(["/multica/executions/cancel", { bindingId: "binding-1", expectedRevision: 4, idempotencyKey: "cancel-command" }]);
  });
  it("creates a rerun via the existing native endpoint without forwarding paths", async () => {
    const f = fixture({ issues: [issue("issue-1", { assignee_type: "agent", assignee_id: "agent-1" })] });
    f.executions[0].state = "completed";
    const result = await f.adapter.transport("/api/issues/issue-1/rerun", init("POST", { task_id: "binding-1" }, "rerun-command"));
    expect(result.status).toBe(200);
    expect(f.postJson.mock.calls.at(-1)).toEqual(["/multica/executions/create", { workspaceId: "workspace-1", issueId: "issue-1", agentId: "agent-1", prompt: "Task issue-1\n\nDescription", idempotencyKey: "rerun-command", executionKind: "thread" }]);
  });
  it("preserves idempotency for three manual triggers and maps run timestamps", async () => {
    const f = fixture({ autopilots: [autopilot()] });
    for (let i = 0; i < 3; i++) {
      const result = await f.adapter.transport("/api/autopilots/auto-1/trigger", init("POST", {}, "trigger-key"));
      const dto = await result.json();
      expect(AutopilotRunSchema.safeParse(dto).success).toBe(true);
      expect(dto).toMatchObject({ triggered_at: date, task_id: "binding-1" });
    }
    expect(f.postJson.mock.calls.filter(([path]) => path.endsWith("/trigger"))).toHaveLength(1);
    expect(f.postJson.mock.calls.find(([path]) => path.endsWith("/trigger"))?.[1]).toEqual({ autopilotId: "auto-1", occurrenceId: "trigger-key", source: "manual" });
    expect(await (await f.adapter.transport("/api/autopilots/auto-1/runs?offset=1&limit=1")).json()).toMatchObject({ total: 2, runs: [{ id: "run-2" }] });
  });
  it("derives activity only from actual execution timestamps", async () => {
    const f = fixture();
    f.executions.push(binding({ bindingId: "old", state: "failed", completedAtMs: Date.now() - 86400000, createdAtMs: Date.now() - 86400000 }));
    const result = await (await f.adapter.transport("/api/agent-activity-30d")).json();
    expect(result).toMatchObject([{ agent_id: "agent-1", task_count: 1, failed_count: 1 }]);
  });
  it("maps Core message summaries without inventing a full transcript", async () => {
    const f = fixture();
    await f.adapter.transport("/api/agents/agent-1/tasks");
    f.postJson.mockResolvedValueOnce({ status: "ok", items: [{ bindingId: "binding-1", messageId: "msg-1", seq: 1, messageType: "text", summary: "Summary", createdAtMs: Date.parse(date) }] });
    expect(await (await f.adapter.transport("/api/tasks/binding-1/messages")).json()).toEqual([{ task_id: "binding-1", issue_id: "issue-1", seq: 1, type: "text", content: "Summary", summary_only: true, created_at: date }]);
  });
});

describe("transport trust boundary and errors", () => {
  it.each(["https://host/api/issues", "//host/api/issues", "/api/issues/../agents", "/api/issues/%2e%2e", "/api/issues/%2fsecret", "/api/issues#fragment", "/api/issues\\agents"])("rejects %s before invoking bridge", async (path) => {
    const f = fixture();
    expect((await f.adapter.transport(path)).status).toBe(400);
    expect(f.postJson).not.toHaveBeenCalled();
  });
  it.each<RequestInit>([{ headers: { Authorization: "secret" } }, { headers: { "X-Forwarded-Host": "remote" } }, { body: '{"title":"task","cwd":"D:/secret"}', method: "POST" }, { body: '{"title":"task","metadata":{"env":{"KEY":"secret"}}}', method: "POST" }, { body: "[]", method: "POST" }, { body: "{", method: "POST" }])("rejects arbitrary forwarding or malformed requests", async (request) => {
    const f = fixture();
    expect((await f.adapter.transport("/api/issues", request)).status).toBe(400);
    expect(f.postJson).not.toHaveBeenCalled();
  });
  it("rejects foreign workspaces and oversized bodies", async () => {
    const f = fixture();
    expect((await f.adapter.transport("/api/issues?workspace_id=other")).status).toBe(403);
    expect((await f.adapter.transport("/api/issues", init("POST", { title: "x".repeat(40000) }))).status).toBe(413);
  });
  it.each([{ title: 12 }, { title: "" }, { title: "Task", status: [] }, { title: "Task", priority: "invalid" }, { title: "Task", assignee_id: "../outside" }, { title: "Task", expected_revision: 1.5 }])("schema-validates mutation fields before writing", async (data) => {
    const f = fixture();
    expect((await f.adapter.transport("/api/issues", init("POST", data))).status).toBe(400);
    expect(f.postJson.mock.calls.some(([path]) => path.endsWith("/upsert"))).toBe(false);
  });
  it.each([["permission_denied", 403], ["revision_conflict", 409], ["execution_timeout", 504], ["codex_page_host_unavailable", 503]] as const)("maps %s without leaking bridge messages", async (code, status) => {
    const f = fixture();
    await f.adapter.transport("/api/me");
    f.postJson.mockResolvedValueOnce({ status: "failed", code, message: "TOKEN=secret D:/private prompt" });
    const result = await f.adapter.transport("/api/issues");
    expect(result.status).toBe(status);
    expect(await result.json()).toEqual({ code, error: code });
  });
  it("recovers bootstrap after disconnection and never falls back to network", async () => {
    const f = fixture();
    const network = vi.spyOn(globalThis, "fetch").mockRejectedValue(new Error("unexpected"));
    f.postJson.mockRejectedValueOnce(new Error("TOKEN=secret"));
    const first = await f.adapter.transport("/api/me");
    expect(first.status).toBe(503);
    expect(await first.text()).not.toContain("secret");
    expect((await f.adapter.transport("/api/me")).status).toBe(200);
    expect(network).not.toHaveBeenCalled();
  });
  it("rejects an aborted request without dispatching a mutation", async () => {
    const f = fixture(), controller = new AbortController();
    controller.abort();
    expect((await f.adapter.transport("/api/issues", { ...init("POST", { title: "No write" }), signal: controller.signal })).status).toBe(408);
    expect(f.postJson).not.toHaveBeenCalled();
  });
});
