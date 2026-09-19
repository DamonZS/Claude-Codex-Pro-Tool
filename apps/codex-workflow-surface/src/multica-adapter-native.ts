import { AdapterError, collectionSchema, identifier, integer, keys, record, text, timestamp, type JsonRecord } from "./multica-adapter-dto";

export async function readNativeExecutionRows(call: (payload: JsonRecord) => Promise<JsonRecord>): Promise<{ items: JsonRecord[]; stale: boolean }> {
  const items: JsonRecord[] = [], ids = new Set<string>();
  let total: number | undefined, stale = false;
  for (let offset = 0; offset < 500; offset += 100) {
    const parsed = collectionSchema.safeParse(await call({ resource: "codex_native_agents", limit: 100, offset }));
    if (!parsed.success) throw new AdapterError(502, "invalid_native_execution_response");
    const current = parsed.data;
    if (total !== undefined && total !== current.total) throw new AdapterError(409, "query_snapshot_changed");
    total = current.total;
    stale ||= current.stale === true;
    for (const item of current.items) {
      if (typeof item.id !== "string" || !item.id || typeof item.parent_thread_id !== "string" || typeof item.status !== "string" || !(item.agent_nickname === null || typeof item.agent_nickname === "string") || !timestamp(item.updated_at_ms) || item.source !== "codex_native" || item.read_only !== true) throw new AdapterError(502, "invalid_native_execution_response");
      if (ids.has(item.id)) throw new AdapterError(409, "query_snapshot_changed");
      ids.add(item.id);
      items.push(item);
    }
    if (items.length === Math.min(total, 500)) return { items, stale };
    if (current.items.length !== 100 || items.length > total) throw new AdapterError(502, "incomplete_native_execution_response");
  }
  return { items, stale };
}

export type NativeDomainPath = "/multica/issues/limit-usage" | "/multica/autopilots/usage" | "/multica/issues/preview-trigger" | "/multica/quick-actions/render" | "/multica/quick-actions/run";
const controls = ["command_id", "idempotency_key", "expected_revision"];
type Context = { commandId: string; commandSignature?: string; signal?: AbortSignal | null };
const response = (value: unknown) => new Response(JSON.stringify(value), { headers: { "Content-Type": "application/json" } });
type Call = (path: NativeDomainPath, payload: JsonRecord, signal?: AbortSignal | null) => Promise<JsonRecord>;

export class MulticaNativeDomainAdapter {
  constructor(private readonly call: Call) {}

  async usage(path: string, params: JsonRecord, context: Context): Promise<Response> {
    keys(params, []);
    if (path === "/api/issues/limit-usage") {
      const result = await this.call("/multica/issues/limit-usage", {}, context.signal);
      if (result.usage === null) return response(null);
      const usage = record(result.usage);
      if (!Number.isSafeInteger(usage.used) || Number(usage.used) < 0 || !Number.isSafeInteger(usage.limit) || Number(usage.limit) < 1) throw new AdapterError(502, "invalid_issue_usage");
      return response(usage);
    }
    const result = await this.call("/multica/autopilots/usage", {}, context.signal);
    const usage = record(result.usage);
    if (!["off", "observe", "enforce"].includes(text(usage.action))) throw new AdapterError(502, "invalid_autopilot_usage");
    for (const key of ["used", "reserved", "total", "limit"]) if (!(usage[key] === null || typeof usage[key] === "number" && Number.isFinite(usage[key]) && Number(usage[key]) >= 0)) throw new AdapterError(502, "invalid_autopilot_usage");
    if (!(usage.reached === null || typeof usage.reached === "boolean")) throw new AdapterError(502, "invalid_autopilot_usage");
    for (const key of ["period_start", "period_end", "reset_at"]) if (!(usage[key] === null || timestamp(usage[key]))) throw new AdapterError(502, "invalid_autopilot_usage");
    if (usage.blocked_counts !== null && (!usage.blocked_counts || typeof usage.blocked_counts !== "object" || Array.isArray(usage.blocked_counts) || Object.values(usage.blocked_counts).some((value) => !Number.isSafeInteger(value) || Number(value) < 0))) throw new AdapterError(502, "invalid_autopilot_usage");
    return response(usage);
  }

  async preview(data: JsonRecord, context: Context): Promise<Response> {
    keys(data, ["issue_ids", "is_create", "assignee_type", "assignee_id", "status", ...controls]);
    if (data.issue_ids !== undefined && (!Array.isArray(data.issue_ids) || data.issue_ids.length > 100) || data.is_create !== undefined && typeof data.is_create !== "boolean" || data.assignee_type !== undefined && !["agent", "member", "squad"].includes(text(data.assignee_type))) throw new AdapterError(400, "invalid_trigger_preview");
    const issueIds = (data.issue_ids as unknown[] | undefined ?? []).map(identifier);
    if (new Set(issueIds).size !== issueIds.length) throw new AdapterError(400, "invalid_trigger_preview");
    const payload: JsonRecord = { issueIds, isCreate: data.is_create ?? false };
    for (const [source, target] of [["assignee_type", "assigneeType"], ["assignee_id", "assigneeId"], ["status", "status"]]) if (Object.hasOwn(data, source)) payload[target] = identifier(data[source]);
    const result = await this.call("/multica/issues/preview-trigger", payload, context.signal);
    if (!Array.isArray(result.triggers) || !Number.isSafeInteger(result.total_count) || Number(result.total_count) < result.triggers.length) throw new AdapterError(502, "invalid_trigger_preview");
    for (const raw of result.triggers) {
      const item = record(raw);
      if (typeof item.issue_id !== "string" || typeof item.agent_id !== "string" || typeof item.source !== "string" || typeof item.handoff_supported !== "boolean") throw new AdapterError(502, "invalid_trigger_preview");
    }
    return response(result);
  }

  async quickAction(issue: JsonRecord, action: JsonRecord, operation: string, data: JsonRecord, context: Context): Promise<Response> {
    keys(data, controls);
    const payload: JsonRecord = { issueId: identifier(issue.id), quickActionId: identifier(action.id) };
    if (operation === "run") Object.assign(payload, { expectedIssueRevision: integer(data.expected_revision ?? issue.revision, 0, Number.MAX_SAFE_INTEGER), expectedActionRevision: integer(action.revision, 0, Number.MAX_SAFE_INTEGER), commandId: context.commandId, commandSignature: context.commandSignature });
    const result = await this.call(operation === "run" ? "/multica/quick-actions/run" : "/multica/quick-actions/render", payload, context.signal);
    if (typeof result.content !== "string" || !result.content.trim()) throw new AdapterError(502, "invalid_quick_action_response");
    if (operation === "run" && (typeof result.id !== "string" || result.issue_id !== issue.id || result.type !== "comment" || result.quick_action_id !== action.id || !Array.isArray(result.trigger_outcomes) || !timestamp(result.created_at))) throw new AdapterError(502, "invalid_quick_action_response");
    return response(result);
  }
}
