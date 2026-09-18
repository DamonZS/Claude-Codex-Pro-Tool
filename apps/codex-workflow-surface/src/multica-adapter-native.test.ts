// @vitest-environment node
import { describe, expect, it, vi } from "vitest";
import { MulticaApiAdapter } from "./multica-api-adapter";
import { type JsonRecord } from "./multica-adapter-dto";
import { AutopilotQuotaUsageSchema, CommentSchema, IssueLimitUsageSchema, IssueTriggerPreviewSchema } from "../vendor/multica/packages/core/api/schemas";

const date = "2026-09-18T00:00:00.000Z";
const init = (method: string, data: JsonRecord = {}, key?: string): RequestInit => ({ method, body: JSON.stringify(data), ...(key ? { headers: { "Idempotency-Key": key } } : {}) });

function fixture() {
  const issue = { id: "issue-1", revision: 4, title: "Review" };
  const action = { id: "action-1", revision: 2, name: "Review", assignee_type: "agent", assignee_id: "agent-1", prompt: "Review this task" };
  const usage = { action: "off", used: null, reserved: null, total: null, limit: null, reached: null, period_start: null, period_end: null, reset_at: null, blocked_counts: null };
  const postJson = vi.fn(async (path: string, payload: JsonRecord): Promise<unknown> => {
    if (path === "/multica/workspace/bootstrap") return { status: "ok", workspace: { id: "workspace-1" }, user: { id: "user-1" } };
    if (path === "/multica/workspace/query") return { status: "ok", items: payload.resource === "issues" ? [issue] : payload.resource === "quick_actions" ? [action] : [], total: ["issues", "quick_actions"].includes(String(payload.resource)) ? 1 : 0 };
    if (path === "/multica/issues/limit-usage") return { usage: { used: 27, limit: 10000 } };
    if (path === "/multica/autopilots/usage") return { usage };
    if (path === "/multica/issues/preview-trigger") return { triggers: [{ issue_id: "issue-1", agent_id: "agent-1", source: "assignment", handoff_supported: false }], total_count: 1 };
    if (path === "/multica/quick-actions/render") return { content: "@Reviewer Review this task" };
    if (path === "/multica/quick-actions/run") return { id: "comment-1", issue_id: "issue-1", author_type: "member", author_id: "user-1", content: "@Reviewer Review this task", type: "comment", parent_id: null, created_at: date, updated_at: date, quick_action_id: "action-1", trigger_outcomes: [{ status: "queued", task_id: "binding-1" }] };
    return { status: "failed", code: "capability_unavailable" };
  });
  return { usage, postJson, adapter: new MulticaApiAdapter({ postJson }) };
}

describe("native domain bounded contracts", () => {
  it("keeps environment management on hold and rejects environment pass-through", async () => {
    const f = fixture();
    expect((await f.adapter.transport("/api/agents/agent-1/env")).status).toBe(503);
    expect((await f.adapter.transport("/api/agents/agent-1/env", init("PUT", { custom_env: { EXAMPLE: "value" } }))).status).toBe(400);
    expect((await f.adapter.transport("/api/agents/agent-1", init("PUT", { custom_env: { EXAMPLE: "value" } }))).status).toBe(400);
    expect(f.postJson.mock.calls.every(([path]) => path === "/multica/workspace/bootstrap")).toBe(true);
  });
  it("returns authoritative usage and explicit off state without manufacturing a quota", async () => {
    const f = fixture();
    const issueUsage = await (await f.adapter.transport("/api/issues/limit-usage")).json();
    expect(IssueLimitUsageSchema.safeParse(issueUsage).success).toBe(true);
    expect(issueUsage).toEqual({ used: 27, limit: 10000 });
    const automationUsage = await (await f.adapter.transport("/api/autopilots/usage")).json();
    expect(AutopilotQuotaUsageSchema.safeParse(automationUsage).success).toBe(true);
    expect(automationUsage).toEqual(f.usage);
    const original = f.postJson.getMockImplementation()!;
    f.postJson.mockImplementation(async (path, payload) => path === "/multica/issues/limit-usage" ? { usage: null } : original(path, payload));
    expect(await (await f.adapter.transport("/api/issues/limit-usage")).json()).toBeNull();
    f.postJson.mockImplementation(async (path, payload) => path === "/multica/autopilots/usage" ? { usage: { action: "off" } } : original(path, payload));
    expect((await f.adapter.transport("/api/autopilots/usage")).status).toBe(502);
  });
  it("uses Core preview with upstream fields and does not dispatch or retain a mutation receipt", async () => {
    const f = fixture();
    const request = init("POST", { issue_ids: ["issue-1"], assignee_type: "agent", assignee_id: "agent-1", status: "todo" }, "preview");
    for (let n = 0; n < 2; n++) {
      const result = await (await f.adapter.transport("/api/issues/preview-trigger", request)).json();
      expect(IssueTriggerPreviewSchema.safeParse(result).success).toBe(true);
      expect(result.total_count).toBe(1);
    }
    expect(f.postJson.mock.calls.at(-1)).toEqual(["/multica/issues/preview-trigger", { issueIds: ["issue-1"], isCreate: false, assigneeType: "agent", assigneeId: "agent-1", status: "todo" }]);
    expect(f.postJson.mock.calls.filter(([path]) => path === "/multica/issues/preview-trigger")).toHaveLength(2);
    expect(f.postJson.mock.calls.some(([path]) => path.includes("/executions/") || path === "/multica/workspace/command")).toBe(false);
  });
  it("keeps quick-action render read-only and returns native queued comment results for run", async () => {
    const f = fixture();
    expect(await (await f.adapter.transport("/api/issues/issue-1/quick-actions/action-1/render", init("POST"))).json()).toEqual({ content: "@Reviewer Review this task" });
    expect(f.postJson.mock.calls.at(-1)).toEqual(["/multica/quick-actions/render", { issueId: "issue-1", quickActionId: "action-1" }]);
    const request = init("POST", {}, "run-action");
    const result = await (await f.adapter.transport("/api/issues/issue-1/quick-actions/action-1/run", request)).json();
    expect(CommentSchema.safeParse(result).success).toBe(true);
    expect(result).toMatchObject({ type: "comment", quick_action_id: "action-1", trigger_outcomes: [{ status: "queued", task_id: "binding-1" }] });
    expect(f.postJson.mock.calls.at(-1)).toEqual(["/multica/quick-actions/run", { issueId: "issue-1", quickActionId: "action-1", expectedIssueRevision: 4, expectedActionRevision: 2, commandId: "run-action", commandSignature: expect.stringMatching(/^[a-f0-9]{64}$/) }]);
    expect(await (await f.adapter.transport("/api/issues/issue-1/quick-actions/action-1/run", request)).json()).toEqual(result);
    expect(f.postJson.mock.calls.filter(([path]) => path === "/multica/quick-actions/run")).toHaveLength(1);
  });
  it("leaves native capacity and admission errors visible without posting a fake comment", async () => {
    const f = fixture(), original = f.postJson.getMockImplementation()!;
    f.postJson.mockImplementation(async (path, payload) => path === "/multica/quick-actions/run" ? { status: "failed", code: "agent_capacity_exceeded", statusCode: 429 } : original(path, payload));
    const result = await f.adapter.transport("/api/issues/issue-1/quick-actions/action-1/run", init("POST"));
    expect(result.status).toBe(429);
    expect(await result.json()).toMatchObject({ code: "agent_capacity_exceeded" });
    expect(f.postJson.mock.calls.some(([path]) => path === "/multica/workspace/upsert")).toBe(false);
  });
  it.each([
    { type: "quick_action" },
    { type: undefined },
    { id: undefined },
    { id: 123 },
    { issue_id: "other-issue" },
    { quick_action_id: undefined },
    { quick_action_id: "other-action" },
    { quick_action_id: 123 },
  ])("rejects malformed quick-action comment identity with 502: %j", async (invalid) => {
    const f = fixture(), original = f.postJson.getMockImplementation()!;
    f.postJson.mockImplementation(async (path, payload) => {
      const result = await original(path, payload);
      return path === "/multica/quick-actions/run" ? { ...(result as JsonRecord), ...invalid } : result;
    });
    const result = await f.adapter.transport("/api/issues/issue-1/quick-actions/action-1/run", init("POST", {}, "malformed-action"));
    expect(result.status).toBe(502);
    expect(await result.json()).toMatchObject({ code: "invalid_quick_action_response" });
    expect(f.postJson.mock.calls.filter(([path]) => path === "/multica/quick-actions/run")).toHaveLength(1);
    expect(f.postJson.mock.calls.some(([path]) => path === "/multica/workspace/upsert")).toBe(false);
  });
});
