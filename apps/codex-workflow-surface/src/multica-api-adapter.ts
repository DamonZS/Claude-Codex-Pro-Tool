/* Codex Runtime Adapter for the derived Multica UI. No HTTP fallback. */
import type { WorkflowSurfaceBridge } from "./runtime-bridge";
import {
  AdapterError, MAX_BODY_BYTES, MAX_SCAN_ITEMS, agentDto, autopilotDto, boundedJson,
  bridgeError, categories, collectionSchema, entity, identifier, integer, issueDto,
  keys, pick, preferenceDto, propertyDto, record, runDto, statusDto, statusKey, taskDto, text, timestamp, validateMutation, type JsonRecord,
} from "./multica-adapter-dto";
import { filterIssues, fingerprint, involved, list, page, tableQuery, type QueryContext } from "./multica-adapter-query";
import { MulticaBuilderAdapter } from "./multica-adapter-builder";
import { MulticaWebhookAdapter, validateEventFilters, webhookTrigger, type WebhookPath } from "./multica-adapter-webhooks";
import { MulticaNativeDomainAdapter, readNativeExecutionRows, type NativeDomainPath } from "./multica-adapter-native";
import { compareExecutionTasks, isVirtualIssue, projectExecutionBoard, setExecutionBoardStatus, type ExecutionBoard } from "./multica-execution-board";

type Resource = "issues" | "comments" | "labels" | "subscribers" | "reactions" | "projects" | "agents" | "squads" | "autopilots" | "issue_views" | "issue_statuses" | "runtimes" | "skills" | "properties" | "issue_view_preferences" | "quick_actions";
type MutableResource = "issues" | "comments" | "labels" | "agents" | "autopilots" | "issue_views" | "quick_actions";
type StoredResource = MutableResource | "issue_statuses" | "properties" | "issue_view_preferences" | "subscribers" | "reactions";
type BridgePath = "/multica/workspace/bootstrap" | "/multica/workspace/query" | "/multica/workspace/upsert" | "/multica/workspace/delete" | "/multica/workspace/move-issue" | "/multica/workspace/command" | "/multica/workspace/reorder-statuses"
  | "/multica/agents/create" | "/multica/executions/list" | "/multica/executions/create" | "/multica/executions/continue" | "/multica/executions/cancel" | "/multica/executions/open" | "/multica/executions/status" | "/multica/executions/messages/list"
  | "/multica/autopilots/runs" | "/multica/autopilots/run" | "/multica/autopilots/trigger" | "/multica/autopilots/cron-preview"
  | "/multica/skills/bindings" | "/multica/skills/bindings/replace" | "/multica/skills/bind" | "/multica/skills/unbind" | WebhookPath | NativeDomainPath;
export interface MulticaAdapterBridge extends WorkflowSurfaceBridge { openThread?(threadId: string): Promise<unknown> }

const controlFields = ["expected_revision", "command_id", "idempotency_key"];
const fields: Record<MutableResource, readonly string[]> = {
  quick_actions: ["name", "description", "assignee_type", "assignee_id", "prompt", "visibility", "status"],
  issues: ["title", "description", "status", "priority", "assignee_type", "assignee_id", "parent_issue_id", "project_id", "stage", "position", "start_date", "due_date", "metadata", "properties", "label_ids"],
  agents: ["name", "description", "instructions", "avatar_url", "runtime_id", "conversation_starters", "visibility", "permission_mode", "invocation_targets", "max_concurrent_tasks", "model", "thinking_level", "service_tier", "template"],
  autopilots: ["title", "description", "project_id", "assignee_type", "assignee_id", "status", "execution_mode", "issue_title_template", "subscribers"],
  issue_views: ["name", "scope_type", "scope_id", "scope_variant", "visibility", "definition_version", "query", "display"],
  comments: ["content", "type", "parent_id"], labels: ["name", "color", "description", "resource_type"],
};
const readOnlyEntityFields = ["can_write", "can_manage_access", "permissions", "labels", "activities", "timeline", "reactions", "queue", "runtime", "runs", "trust_state", "dispatch_allowed", "installed", "compatible"];
const routeResources: Record<string, MutableResource> = { issues: "issues", agents: "agents", autopilots: "autopilots", "issue-views": "issue_views", "quick-actions": "quick_actions", labels: "labels", comments: "comments" };
const activeStates = ["queued", "dispatched", "running", "waiting_local_directory", "cancel_pending", "reconciling"];

function response(value: unknown, status = 200): Response {
  return new Response(status === 204 ? null : JSON.stringify(value), { status, headers: { "Content-Type": "application/json" } });
}
function unavailable(): never { throw new AdapterError(503, "capability_unavailable"); }
function rows(value: unknown): JsonRecord[] {
  if (!Array.isArray(value)) throw new AdapterError(502, "invalid_bridge_response");
  return value.map(record);
}

interface RequestContext { workspaceId: string; userId: string; commandId: string; commandSignature?: string; receipt?: Promise<JsonRecord | null>; signal?: AbortSignal | null }
interface Command { signature: string; promise: Promise<Response> }
type IssueRunControls = { suppressRun?: boolean; handoffNote?: string };

export class MulticaApiAdapter {
  private bootstrapSnapshot: Promise<JsonRecord> | null = null;
  private readonly seen = new Map<string, JsonRecord>();
  private readonly commands = new Map<string, Command>();
  private readonly requestIds = new WeakMap<RequestInit, string>();
  private completeBoard: ExecutionBoard | null = null;
  private usableBoard: ExecutionBoard | null = null;
  private boardUpdatedAt: number | null = null;
  private pendingBoard: Promise<ExecutionBoard> | null = null;
  private readonly builder = new MulticaBuilderAdapter((payload, signal) => this.callValue("/multica/builder", payload, signal));
  private readonly webhooks = new MulticaWebhookAdapter((path, payload, signal) => this.call(path, payload, signal));
  private readonly nativeDomains = new MulticaNativeDomainAdapter((path, payload, signal) => this.call(path, payload, signal));

  constructor(private readonly bridge: MulticaAdapterBridge) {}

  private async call(path: BridgePath, payload: JsonRecord, signal?: AbortSignal | null): Promise<JsonRecord> {
    return record(await this.callValue(path, payload, signal));
  }

  private async callValue(path: BridgePath | "/multica/builder", payload: JsonRecord, signal?: AbortSignal | null): Promise<unknown> {
    if (new TextEncoder().encode(JSON.stringify(payload)).length > MAX_BODY_BYTES) throw new AdapterError(413, "request_too_large");
    if (signal?.aborted) throw new AdapterError(408, "request_aborted");
    let timer: ReturnType<typeof setTimeout> | undefined;
    let abort: (() => void) | undefined;
    try {
      const result = await Promise.race([
        this.bridge.postJson(path, payload),
        new Promise<never>((_, reject) => {
          timer = setTimeout(() => reject(new AdapterError(504, "bridge_timeout")), 30000);
          abort = () => reject(new AdapterError(408, "request_aborted"));
          signal?.addEventListener("abort", abort, { once: true });
        }),
      ]);
      if (result && typeof result === "object" && !Array.isArray(result)) {
        const body = record(result);
        const domainResult = (path === "/multica/builder" || path.startsWith("/multica/webhooks/")) && typeof body.id === "string" && !Object.hasOwn(body, "code");
        if (body.status === "failed" && !domainResult || body.status === "error" || body.ok === false || typeof body.status === "number" && body.status >= 400) throw bridgeError(body);
      }
      return result;
    } catch (error) {
      throw error instanceof AdapterError ? error : bridgeError(error);
    } finally {
      clearTimeout(timer);
      if (abort) signal?.removeEventListener("abort", abort);
    }
  }

  private async bootstrap(signal?: AbortSignal | null): Promise<JsonRecord> {
    this.bootstrapSnapshot ??= this.call("/multica/workspace/bootstrap", {}, signal).then((value) => {
      identifier(record(value.workspace).id);
      identifier(record(value.user).id);
      return value;
    }).catch((error: unknown) => { this.bootstrapSnapshot = null; throw error; });
    return this.bootstrapSnapshot;
  }

  private remember(resource: string, value: JsonRecord): JsonRecord {
    if (typeof value.id === "string" && Number.isSafeInteger(value.revision)) {
      const key = `${resource}:${value.id}`;
      const previous = this.seen.get(key);
      if (!previous || Number(previous.revision) <= Number(value.revision)) this.seen.set(key, value);
      if (this.seen.size > MAX_SCAN_ITEMS) this.seen.delete(this.seen.keys().next().value!);
    }
    return value;
  }

  // Bridge queries have a hard 100-row page. Never filter a truncated window.
  private async scan(resource: Resource, context: RequestContext): Promise<JsonRecord[]> {
    const all: JsonRecord[] = [];
    const ids = new Set<unknown>();
    let total: number | undefined;
    for (let offset = 0; offset < MAX_SCAN_ITEMS; offset += 100) {
      const raw = await this.call("/multica/workspace/query", { resource, limit: 100, offset }, context.signal);
      const parsed = collectionSchema.safeParse(raw);
      if (!parsed.success) throw new AdapterError(502, "invalid_collection_response");
      const current = parsed.data;
      if (current.stale) throw new AdapterError(503, "capability_unavailable");
      if (total !== undefined && total !== current.total) throw new AdapterError(409, "query_snapshot_changed");
      total = current.total;
      if (total > MAX_SCAN_ITEMS) throw new AdapterError(413, "query_window_too_large");
      for (const item of current.items) {
        if (item.workspace_id !== undefined && item.workspace_id !== context.workspaceId) throw new AdapterError(403, "workspace_mismatch");
        if (ids.has(item.id)) throw new AdapterError(409, "query_snapshot_changed");
        ids.add(item.id);
        all.push(this.remember(resource, item));
      }
      if (all.length === total) return all;
      if (current.items.length !== 100 || all.length > total) throw new AdapterError(502, "incomplete_collection_response");
    }
    throw new AdapterError(413, "query_window_too_large");
  }

  private async find(resource: Resource, id: string, context: RequestContext, cached = false): Promise<JsonRecord> {
    if (cached && this.seen.has(`${resource}:${id}`)) return this.seen.get(`${resource}:${id}`)!;
    const result = (await this.scan(resource, context)).find((item) => item.id === id || resource === "issues" && item.identifier === id);
    if (!result) throw new AdapterError(404, "not_found");
    return result;
  }

  private dto(resource: StoredResource, value: unknown, context: RequestContext): JsonRecord {
    const source = entity(value);
    this.remember(resource, source);
    if (resource === "issues") return issueDto(source, context.workspaceId);
    if (resource === "agents") return agentDto(source, context.workspaceId);
    if (resource === "autopilots") return autopilotDto(source, context.workspaceId);
    if (resource === "issue_statuses") return statusDto(source, context.workspaceId);
    if (resource === "properties") return propertyDto(source, context.workspaceId);
    if (["quick_actions", "subscribers", "reactions"].includes(resource)) return { ...source, created_at: timestamp(source.created_at_ms ?? source.created_at) ?? "", updated_at: timestamp(source.updated_at_ms ?? source.updated_at) ?? "" };
    return source;
  }

  private revision(data: JsonRecord, current: JsonRecord): number {
    const revision = integer(data.expected_revision ?? current.revision, 0, Number.MAX_SAFE_INTEGER);
    if (!revision) throw new AdapterError(409, "revision_required");
    return revision;
  }

  private async save(resource: StoredResource, value: JsonRecord, expectedRevision: number, context: RequestContext, runControls: IssueRunControls = {}): Promise<JsonRecord> {
    const replay = await this.receipt(context);
    if (replay) return this.dto(resource, replay.entity, context);
    const clean = Object.fromEntries(Object.entries(value).filter(([key]) => !readOnlyEntityFields.includes(key)));
    const result = await this.call("/multica/workspace/upsert", { resource, entity: clean, expectedRevision, commandId: context.commandId, commandSignature: context.commandSignature, ...runControls }, context.signal);
    return this.dto(resource, result.entity, context);
  }

  private receipt(context: RequestContext): Promise<JsonRecord | null> {
    return context.receipt ??= this.call("/multica/workspace/command", { commandId: context.commandId, commandSignature: context.commandSignature }, context.signal).then((reply) => {
      if (reply.found === false) return null;
      if (reply.found !== true) throw new AdapterError(502, "invalid_command_response");
      const result = record(reply.result);
      if (result.status === "failed" || result.status === "error" || result.ok === false) throw bridgeError(result);
      return result;
    });
  }

  private async propertyCatalog(id: string | null, method: string, params: JsonRecord, data: JsonRecord, context: RequestContext): Promise<Response> {
    if (method === "GET" && !id) {
      keys(params, ["include_archived"]);
      const properties = (await this.scan("properties", context)).filter((item) => params.include_archived === "true" || !item.archived).map((item) => propertyDto(item, context.workspaceId));
      return response({ properties, total: properties.length });
    }
    if (!(method === "POST" && !id || method === "PATCH" && id)) unavailable();
    keys(data, ["name", "description", "icon", "config", ...(id ? ["archived"] : ["type"]), ...controlFields]);
    if (data.config && Array.isArray(record(data.config).options)) {
      const config = record(data.config);
      const options = await Promise.all((config.options as unknown[]).map(async (value, index) => {
        const option = record(value);
        // The original editor leaves new option IDs blank. Keep IDs stable on
        // retries while preserving the raw request's command signature.
        return option.id === "" || option.id === undefined
          ? { ...option, id: await fingerprint({ commandId: context.commandId, option: index }) }
          : option;
      }));
      data = { ...data, config: { ...config, options } };
    }
    validateMutation("properties", data, !id);
    const replay = await this.receipt(context);
    if (replay) return response(this.dto("properties", replay.entity, context), id ? 200 : 201);
    const current = id ? await this.find("properties", id, context, true) : null;
    const value = { ...(current ?? { id: context.commandId, workspace_id: context.workspaceId, description: "", icon: "", config: {}, archived: false }), ...pick(data, ["name", "description", "icon", "config", "type", "archived"]) };
    return response(await this.save("properties", value, current ? this.revision(data, current) : 0, context), id ? 200 : 201);
  }

  private async viewPreference(method: string, params: JsonRecord, data: JsonRecord, context: RequestContext): Promise<Response> {
    if (!["GET", "PUT"].includes(method)) unavailable();
    const input = method === "GET" ? params : data;
    keys(input, ["scope_type", "scope_id", ...(method === "PUT" ? ["prefs", ...controlFields] : [])]);
    if (!["workspace", "my", "project"].includes(text(input.scope_type)) || input.scope_type === "project" && !input.scope_id || input.scope_type !== "project" && input.scope_id != null) throw new AdapterError(400, "invalid_preference_scope");
    if (input.scope_id != null) identifier(input.scope_id);
    const scope = { scope_type: input.scope_type, scope_id: input.scope_id ?? null };
    if (method === "PUT") {
      const prefs = input.prefs && typeof input.prefs === "object" && !Array.isArray(input.prefs) ? record(input.prefs) : {};
      keys(prefs, ["hidden", "order"]);
      for (const value of [prefs.hidden, prefs.order]) {
        if (!Array.isArray(value) || value.length > 256 || !value.every((id) => typeof id === "string") || new Set(value).size !== value.length) throw new AdapterError(400, "invalid_preferences");
        value.forEach(identifier);
      }
      const replay = await this.receipt(context);
      if (replay) return response(preferenceDto(entity(replay.entity)));
    }
    const current = (await this.scan("issue_view_preferences", context)).find((item) => item.user_id === context.userId && item.scope_type === scope.scope_type && (item.scope_id ?? null) === scope.scope_id);
    if (method === "GET") return response(preferenceDto(current ?? { ...scope, prefs: { hidden: [], order: [] }, updated_at: null }));
    const saved = await this.save("issue_view_preferences", { ...(current ?? { id: await fingerprint({ workspace: context.workspaceId, user: context.userId, ...scope }), workspace_id: context.workspaceId, user_id: context.userId }), ...scope, prefs: input.prefs }, current ? this.revision(data, current) : integer(data.expected_revision, 0, Number.MAX_SAFE_INTEGER), context);
    return response(preferenceDto(saved));
  }

  private async issueProperty(issueId: string, propertyId: string, method: string, data: JsonRecord, context: RequestContext): Promise<Response> {
    keys(data, [...controlFields, ...(method === "PUT" ? ["value"] : [])]);
    if (method === "PUT" && !(data.value === null || ["string", "boolean"].includes(typeof data.value) || typeof data.value === "number" && Number.isFinite(data.value) || Array.isArray(data.value) && data.value.every((item) => typeof item === "string"))) throw new AdapterError(400, "invalid_property_value");
    const replay = await this.receipt(context);
    if (replay) {
      const saved = this.dto("issues", replay.entity, context);
      return response({ properties: saved.properties, issue_revision: saved.revision });
    }
    const current = await this.find("issues", issueId, context, true);
    await this.find("properties", propertyId, context);
    const properties = { ...record(current.properties ?? {}) };
    if (method === "DELETE") delete properties[propertyId]; else properties[propertyId] = data.value;
    const saved = await this.save("issues", { ...current, properties }, this.revision(data, current), context);
    return response({ properties: saved.properties, issue_revision: saved.revision });
  }

  private async socialMutation(domain: "issues" | "comments", id: string, action: string, method: string, data: JsonRecord, context: RequestContext): Promise<Response> {
    const reaction = action === "reactions", resource = reaction ? "reactions" : "subscribers";
    const adding = reaction ? method === "POST" : action === "subscribe";
    keys(data, [...controlFields, ...(reaction ? ["emoji"] : ["user_type", "user_id"])]);
    if (reaction && (typeof data.emoji !== "string" || !data.emoji.trim() || [...data.emoji].length > 32)) throw new AdapterError(400, "invalid_emoji");
    const target = reaction ? { [`${domain === "issues" ? "issue" : "comment"}_id`]: id, actor_type: "member", actor_id: context.userId, emoji: data.emoji }
      : { issue_id: id, user_type: data.user_type ?? "member", user_id: identifier(data.user_id ?? context.userId) };
    if (!reaction && !["member", "agent"].includes(text(target.user_type))) throw new AdapterError(400, "invalid_subscriber");
    const replay = await this.receipt(context);
    if (replay) return reaction && adding ? response(this.dto(resource, replay.entity, context)) : response(null, 204);
    await this.find(domain, id, context);
    const current = (await this.scan(resource, context)).find((item) => Object.entries(target).every(([key, value]) => item[key] === value));
    if (adding) {
      const saved = await this.save(resource, current ?? { id: await fingerprint({ workspace: context.workspaceId, resource, ...target }), workspace_id: context.workspaceId, ...target, ...(!reaction ? { reason: "manual" } : {}) }, current ? this.revision(data, current) : 0, context);
      return reaction ? response(this.dto(resource, saved, context)) : response(null, 204);
    }
    if (current) await this.call("/multica/workspace/delete", { resource, entityId: current.id, expectedRevision: this.revision(data, current), commandId: context.commandId, commandSignature: context.commandSignature }, context.signal);
    return response(null, 204);
  }

  private async collaborators(autopilotId: string, userId: string | undefined, method: string, data: JsonRecord, context: RequestContext): Promise<Response> {
    keys(data, [...controlFields, ...(method === "POST" ? ["user_id"] : [])]);
    const targetId = identifier(userId ?? data.user_id);
    const replay = await this.receipt(context);
    if (replay) return response({ collaborators: rows(entity(replay.entity).collaborators) });
    const current = await this.find("autopilots", autopilotId, context, true);
    const collaborators = rows(current.collaborators ?? []);
    const updated = collaborators.filter((item) => item.user_id !== targetId);
    if (method === "POST") updated.push(collaborators.find((item) => item.user_id === targetId) ?? { user_type: "member", user_id: targetId, granted_by: context.userId, created_at: new Date().toISOString() });
    const saved = await this.save("autopilots", { ...current, collaborators: updated }, this.revision(data, current), context);
    return response({ collaborators: rows(saved.collaborators) });
  }

  private async mutateStatus(id: string | null, method: string, data: JsonRecord, context: RequestContext): Promise<Response> {
    const editable = ["name", "description", "color", "position"];
    keys(data, [...controlFields, ...(method === "DELETE" ? [] : id ? editable : ["name", "description", "color", "key", "category"])]);
    validateMutation("issue_statuses", data, !id);
    const replay = await this.receipt(context);
    if (replay) return response(this.dto("issue_statuses", replay.entity, context), id ? 200 : 201);
    const previous = id ? await this.find("issue_statuses", id, context, true) : undefined;
    if (previous?.is_system === true || previous?.can_write === false) throw new AdapterError(403, "permission_denied");
    let value: JsonRecord;
    if (previous) {
      value = { ...previous, ...pick(data, editable), ...(method === "DELETE" ? { archived_at: new Date().toISOString() } : {}) };
    } else {
      const catalog = await this.scan("issue_statuses", context);
      const position = Math.max(0, ...catalog.filter((entry) => entry.category === data.category).map((entry) => integer(entry.position, 0, Number.MAX_SAFE_INTEGER))) + 1;
      if (!Number.isSafeInteger(position)) throw new AdapterError(409, "status_position_exhausted");
      value = { id: context.commandId, workspace_id: context.workspaceId, ...pick(data, ["name", "description", "category", "color"]),
        key: statusKey(data, catalog), description: data.description ?? "", is_system: false, position, archived_at: null };
    }
    if (typeof value.name === "string") value.name = value.name.trim();
    value.color = text(value.color).toLowerCase();
    const saved = await this.save("issue_statuses", value, previous ? this.revision(data, previous) : 0, context);
    return response(saved, previous ? 200 : 201);
  }

  private async reorderStatuses(data: JsonRecord, context: RequestContext): Promise<Response> {
    keys(data, ["category", "ids", ...controlFields]);
    if (!categories.includes(data.category as typeof categories[number]) || !Array.isArray(data.ids) || !data.ids.length || data.ids.length > 256 || new Set(data.ids).size !== data.ids.length) throw new AdapterError(400, "invalid_status_reorder");
    const orderedIds = data.ids.map(identifier);
    const replay = await this.receipt(context);
    const catalog = await this.scan("issue_statuses", context);
    let result = replay;
    if (!result) {
      const statuses = catalog.filter((status) => status.category === data.category && !status.archived_at);
      const builtins = statuses.filter((status) => status.is_system === true);
      if (orderedIds.some((id) => builtins.some((status) => status.id === id))) throw new AdapterError(403, "system_issue_status_immutable");
      const completeIds = [...builtins.map((status) => identifier(status.id)), ...orderedIds];
      if (completeIds.length !== statuses.length) throw new AdapterError(409, "status_catalog_changed");
      const expectedRevisions = Object.fromEntries(completeIds.map((id) => {
        const current = statuses.find((status) => status.id === id);
        if (!current || current.category !== data.category || current.archived_at) throw new AdapterError(409, "status_catalog_changed");
        return [id, this.revision({}, current)];
      }));
      result = await this.call("/multica/workspace/reorder-statuses", { category: data.category, ids: completeIds, expectedRevisions, commandId: context.commandId, commandSignature: context.commandSignature }, context.signal);
    }
    const changed = rows(result.statuses).map((item) => this.dto("issue_statuses", item, context));
    const custom = changed.filter((item) => item.is_system !== true);
    if (custom.length !== orderedIds.length || custom.some((item, index) => item.id !== orderedIds[index]) || changed.some((item) => item.category !== data.category)) throw new AdapterError(502, "invalid_status_reorder_response");
    const statuses = [...catalog.filter((item) => !changed.some((row) => row.id === item.id)).map((item) => statusDto(item, context.workspaceId)), ...changed]
      .sort((a, b) => categories.indexOf(a.category as typeof categories[number]) - categories.indexOf(b.category as typeof categories[number]) || Number(a.position) - Number(b.position));
    return response({ statuses, categories, total: statuses.length });
  }

  private async mutate(resource: MutableResource, id: string | null, method: string, data: JsonRecord, context: RequestContext): Promise<Response> {
    keys(data, [...fields[resource], ...controlFields, ...(resource === "issues" ? ["title_base", "description_base", "suppress_run", "handoff_note", "attachment_ids"] : []), ...(resource === "agents" ? ["skill_ids"] : [])]);
    validateMutation(resource, data, !id);
    if (Array.isArray(data.attachment_ids) && data.attachment_ids.length) unavailable();
    const runControls: IssueRunControls = {};
    if (Object.hasOwn(data, "suppress_run") || Object.hasOwn(data, "handoff_note")) {
      if (resource !== "issues" || !id || !["PUT", "PATCH"].includes(method)) throw new AdapterError(400, "unsupported_request_field");
      if (Object.hasOwn(data, "suppress_run")) runControls.suppressRun = data.suppress_run as boolean;
      if (Object.hasOwn(data, "handoff_note")) runControls.handoffNote = data.handoff_note as string;
    }
    const replay = await this.receipt(context);
    if (replay) {
      if (method === "DELETE") {
        if (replay.deleted !== true) throw new AdapterError(404, "not_found");
        this.seen.delete(`${resource}:${id}`);
        return response(null, 204);
      }
      return response(this.dto(resource, replay.entity ?? replay.agent, context));
    }
    const previous = id ? await this.find(resource, id, context, true) : undefined;
    if (previous?.can_write === false) throw new AdapterError(403, "permission_denied");
    if (previous && resource === "issues") for (const field of ["title", "description"]) {
      if (Object.hasOwn(data, `${field}_base`) && data[`${field}_base`] !== previous[field]) throw new AdapterError(409, "revision_conflict");
    }
    if (method === "DELETE") {
      if (!previous) throw new AdapterError(404, "not_found");
      const deleted = await this.call("/multica/workspace/delete", { resource, entityId: previous.id, expectedRevision: this.revision(data, previous), commandId: context.commandId, commandSignature: context.commandSignature }, context.signal);
      if (deleted.deleted !== true) throw new AdapterError(404, "not_found");
      this.seen.delete(`${resource}:${previous.id}`);
      return response(null, 204);
    }
    const now = new Date().toISOString();
    const value: JsonRecord = { ...(previous ?? { id: context.commandId, workspace_id: context.workspaceId, revision: 1, created_at: now }), ...pick(data, fields[resource]), updated_at: now };
    if (!previous) {
      if (resource === "issues") Object.assign(value, { creator_type: "member", creator_id: context.userId, status: data.status ?? "todo", priority: data.priority ?? "none", metadata: data.metadata ?? {}, properties: data.properties ?? {} });
      if (resource === "autopilots") Object.assign(value, { created_by_type: "member", created_by_id: context.userId, status: data.status ?? "active", assignee_type: data.assignee_type ?? "agent", triggers: [] });
      if (resource === "quick_actions") Object.assign(value, { created_by_id: context.userId, description: data.description ?? "", status: "active", visibility: data.visibility ?? "public", use_count: 0, last_used_at: null });
      if (resource === "agents" || resource === "issue_views") value.owner_id = context.userId;
      if (resource === "issue_views") Object.assign(value, { definition_version: data.definition_version ?? 1, visibility: data.visibility ?? "private", query: data.query ?? {}, display: data.display ?? {} });
    }
    if (resource === "agents" && !previous) {
      const skills = list(data.skill_ids).map((skillId) => ({ id: identifier(skillId) }));
      const saved = await this.call("/multica/agents/create", { entity: value, skills, commandId: context.commandId, commandSignature: context.commandSignature }, context.signal);
      return response(this.dto(resource, saved.entity ?? saved.agent, context));
    }
    return response(await this.save(resource, value, previous ? this.revision(data, previous) : 0, context, runControls));
  }

  private async executions(context: RequestContext, issueId?: string): Promise<JsonRecord[]> {
    const all: JsonRecord[] = [];
    const ids = new Set<unknown>();
    let total: number | undefined;
    for (let offset = 0; offset < MAX_SCAN_ITEMS; offset += 100) {
      const result = await this.call("/multica/executions/list", { workspaceId: context.workspaceId, ...(issueId ? { issueId } : {}), limit: 100, offset }, context.signal);
      const parsed = collectionSchema.safeParse(result);
      if (!parsed.success) throw new AdapterError(502, "invalid_execution_response");
      if (parsed.data.stale) throw new AdapterError(503, "execution_source_stale");
      if (total !== undefined && total !== parsed.data.total) throw new AdapterError(409, "query_snapshot_changed");
      total = parsed.data.total;
      if (total > MAX_SCAN_ITEMS) throw new AdapterError(413, "query_window_too_large");
      for (const item of parsed.data.items) {
        if (item.workspaceId !== context.workspaceId) throw new AdapterError(403, "workspace_mismatch");
        if (ids.has(item.bindingId)) throw new AdapterError(409, "query_snapshot_changed");
        ids.add(item.bindingId);
        all.push(this.remember("tasks", taskDto(item)));
      }
      if (all.length === total) return all;
      if (parsed.data.items.length !== 100) throw new AdapterError(502, "incomplete_collection_response");
    }
    throw new AdapterError(413, "query_window_too_large");
  }

  private async executionBoard(context: RequestContext): Promise<ExecutionBoard> {
    if (this.pendingBoard) return this.pendingBoard;
    this.pendingBoard = (async () => {
      const [issues, tasks, native, agents, squads] = await Promise.allSettled([
        this.scan("issues", context).then((items) => items.map((item) => issueDto(item, context.workspaceId))),
        this.executions(context),
        readNativeExecutionRows((payload) => this.call("/multica/workspace/query", payload, context.signal)),
        this.scan("agents", context), this.scan("squads", context),
      ]);
      if (context.signal?.aborted) throw new AdapterError(408, "request_aborted");
      const failure = [issues, tasks, native, agents, squads].find((source) => source.status === "rejected");
      const stale = !!failure || native.status === "fulfilled" && native.value.stale;
      if (stale) {
        const diagnostic = failure?.status === "rejected" ? bridgeError(failure.reason).code : "native_execution_source_stale";
        setExecutionBoardStatus({ stale: true, diagnostic, updatedAt: this.boardUpdatedAt });
        if (this.completeBoard) return this.completeBoard;
        if (failure && this.usableBoard) return this.usableBoard;
        if (issues.status === "rejected") throw issues.reason;
      }
      const board = projectExecutionBoard(issues.status === "fulfilled" ? issues.value : [], tasks.status === "fulfilled" ? tasks.value : [],
        native.status === "fulfilled" ? native.value.items : [], agents.status === "fulfilled" ? agents.value : [], squads.status === "fulfilled" ? squads.value : [], context.workspaceId, context.userId);
      if (!stale) {
        this.completeBoard = board;
        this.boardUpdatedAt = Date.now();
        setExecutionBoardStatus({ stale: false, diagnostic: null, updatedAt: this.boardUpdatedAt });
      }
      this.usableBoard = board;
      return board;
    })();
    try { return await this.pendingBoard; } finally { this.pendingBoard = null; }
  }

  private async quickCreate(data: JsonRecord, context: RequestContext): Promise<Response> {
    keys(data, ["agent_id", "squad_id", "prompt", "priority", "due_date", "project_id", "parent_issue_id", "attachment_ids", ...controlFields]);
    if (typeof data.prompt !== "string" || !data.prompt.trim() || data.prompt.length > 28000 || !!data.agent_id === !!data.squad_id) throw new AdapterError(400, "invalid_quick_create");
    if (data.squad_id) unavailable();
    const agentId = identifier(data.agent_id), prompt = data.prompt.trim();
    const issue = { ...pick(data, ["priority", "due_date", "project_id", "parent_issue_id", "attachment_ids"]),
      title: prompt.split(/\r?\n/, 1)[0].slice(0, 200), description: prompt, assignee_type: "agent", assignee_id: agentId };
    validateMutation("issues", issue, true);
    if (Array.isArray(data.attachment_ids) && data.attachment_ids.length) unavailable();
    const agent = await this.find("agents", agentId, context);
    if (agent.archived_at) throw new AdapterError(409, "agent_archived");
    if (!agent.runtime_id) unavailable();
    const created = record(await (await this.mutate("issues", null, "POST", issue, context)).json());
    const issueId = identifier(created.id);
    try {
      const tasks = await this.executions(context, issueId);
      const task = tasks.find((item) => item.agent_id === agentId);
      if (!task) throw new AdapterError(503, "assignment_binding_unavailable");
      if (["failed", "cancelled", "unsupported"].includes(text(task.status))) throw new AdapterError(503, "assignment_failed", { task_id: task.id });
      return response({ task_id: task.id }, 202);
    } catch (error) {
      const failure = error instanceof AdapterError ? error : bridgeError(error);
      throw new AdapterError(failure.status, failure.code, { ...failure.details, issue_id: issueId });
    }
  }

  private async issueQuery(path: string, params: JsonRecord, context: RequestContext): Promise<Response> {
    const board = await this.executionBoard(context), all = board.issues;
    const contextData: QueryContext = { userId: context.userId, agents: board.agents, squads: board.squads, tasks: board.tasks };
    if (path.startsWith("/api/issues/table/")) {
      const kind = path.slice("/api/issues/table/".length);
      if (kind !== "groups" && kind !== "rows" && kind !== "facets") unavailable();
      const queryContext = contextData;
      if (params.group && record(params.group).kind === "property") queryContext.properties = await this.scan("properties", context);
      return response(await tableQuery(kind, params, all, queryContext));
    }
    if (path === "/api/issues/child-progress") {
      keys(params, []);
      const parents = new Map<string, { parent_issue_id: string; total: number; done: number }>();
      for (const issue of all) if (issue.parent_issue_id) {
        const id = text(issue.parent_issue_id), progress = parents.get(id) ?? { parent_issue_id: id, total: 0, done: 0 };
        progress.total++;
        if (issue.status_category === "done") progress.done++;
        parents.set(id, progress);
      }
      return response({ progress: [...parents.values()] });
    }
    if (path === "/api/issues/search" && params.include_closed !== true && params.include_closed !== "true") params = { ...params, open_only: true };
    const filtered = filterIssues(all, params, contextData);
    if (path === "/api/issues/grouped") {
      if (params.group_by !== "assignee") throw new AdapterError(400, "invalid_group");
      const grouped = new Map<string, JsonRecord[]>();
      for (const issue of filtered) {
        const id = issue.assignee_id ? `${issue.assignee_type}:${issue.assignee_id}` : "none";
        grouped.set(id, [...(grouped.get(id) ?? []), issue]);
      }
      return response({ groups: [...grouped].map(([id, issues]) => ({ id, assignee_type: issues[0].assignee_type, assignee_id: issues[0].assignee_id, issues: page(issues, params).items, total: issues.length })) });
    }
    const result = page(filtered, params);
    return response({ issues: path === "/api/issues/search" ? result.items.map((item) => ({ ...item, match_source: "title" })) : result.items, total: result.total });
  }

  private async taskAction(action: string, task: JsonRecord, data: JsonRecord, context: RequestContext): Promise<Response> {
    const bindingId = identifier(task.binding_id ?? task.id);
    if (action === "open") {
      if (!this.bridge.openThread) unavailable();
      const result = await this.call("/multica/executions/open", { bindingId }, context.signal);
      const binding = record(result.binding), threadId = identifier(binding.codexThreadId);
      const opened = await this.bridge.openThread(threadId);
      if (opened === false || opened && typeof opened === "object" && (record(opened).status === "failed" || record(opened).ok === false)) throw new AdapterError(503, "thread_open_failed");
      return response({ thread_id: threadId, task: taskDto(binding) });
    }
    if (action === "messages") {
      const result = await this.call("/multica/executions/messages/list", { bindingId }, context.signal);
      return response(rows(result.messages ?? result.items).map((message) => {
        if (message.bindingId !== bindingId || !Number.isSafeInteger(message.seq) || typeof message.messageType !== "string") throw new AdapterError(502, "invalid_message_response");
        return { task_id: bindingId, issue_id: text(task.issue_id), seq: message.seq, type: message.messageType,
          ...(message.tool ? { tool: message.tool } : {}), ...(message.summary ? { content: message.summary } : {}),
          created_at: timestamp(message.createdAtMs), summary_only: true };
      }));
    }
    if (action === "continue" && (typeof data.prompt !== "string" || !data.prompt.trim())) throw new AdapterError(400, "prompt_required");
    const result = await this.call(action === "continue" ? "/multica/executions/continue" : action === "cancel" ? "/multica/executions/cancel" : "/multica/executions/status", {
      bindingId,
      ...(action === "continue" || action === "cancel" ? { expectedRevision: this.revision(data, task), idempotencyKey: context.commandId } : {}),
      ...(action === "continue" ? { prompt: data.prompt } : {}),
    }, context.signal);
    return response(this.remember("tasks", taskDto(record(result.binding))));
  }

  private async route(path: string, method: string, params: JsonRecord, data: JsonRecord, context: RequestContext, bootstrap: JsonRecord): Promise<Response> {
    if (path.startsWith("/api/agent-builder/") || path.startsWith("/api/chat/") || /^\/api\/tasks\/[^/]+\/cancel$/.test(path) && this.builder.knowsTask(path.split("/")[3])) {
      const result = await this.builder.route(path, method, params, data, context);
      if (result) return result;
    }
    if (path === "/api/properties") return this.propertyCatalog(null, method, params, data, context);
    if (/^\/api\/properties\/[^/]+$/.test(path)) return this.propertyCatalog(path.split("/")[3], method, params, data, context);
    if (path === "/api/issue-view-preferences") return this.viewPreference(method, params, data, context);
    if (method === "POST" && path === "/api/issues/quick-create") return this.quickCreate(data, context);
    if (method === "GET" && path === "/api/me") return response(record(bootstrap.user));
    if (method === "GET" && path === "/api/config") {
      keys(params, []);
      return response({ agent_conversation_starters_supported: true, allow_signup: false, workspace_creation_disabled: true, vcs_integration_available: false, local_worktree_supported: false, feature_flags: { local_property_catalog_management: record(bootstrap.permissions ?? {}).managePropertyCatalog === true } });
    }
    if (method === "GET" && path === "/api/workspaces") return response([record(bootstrap.workspace)]);
    if (method === "GET" && path === `/api/workspaces/${context.workspaceId}/members`) {
      if (Array.isArray(bootstrap.members)) return response(rows(bootstrap.members));
      const user = record(bootstrap.user);
      if (user.kind !== "local_control_plane") unavailable();
      return response([{ id: context.userId, user_id: context.userId, workspace_id: context.workspaceId,
        role: "member", name: text(user.name), email: text(user.email), avatar_url: user.avatar_url ?? null, created_at: timestamp(user.created_at) ?? "" }]);
    }
    if ((method === "GET" && ["/api/issues", "/api/issues/grouped", "/api/issues/search", "/api/issues/children", "/api/issues/child-progress"].includes(path)) || method === "POST" && ["/api/issues/query", "/api/issues/table/groups", "/api/issues/table/rows", "/api/issues/table/facets"].includes(path)) return this.issueQuery(path, method === "POST" ? data : params, context);
    if (method === "GET" && path === "/api/issue-statuses") {
      keys(params, ["include_archived"]);
      const statuses = (await this.scan("issue_statuses", context)).filter((s) => params.include_archived === "true" || !s.archived_at).map((item) => statusDto(item, context.workspaceId));
      return response({ statuses, categories, total: statuses.length });
    }
    if (method === "POST" && path === "/api/issue-statuses") return this.mutateStatus(null, method, data, context);
    if (method === "PATCH" && path === "/api/issue-statuses/reorder") return this.reorderStatuses(data, context);
    if (/^\/api\/issue-statuses\/[^/]+$/.test(path) && ["PATCH", "DELETE"].includes(method)) {
      const id = path.split("/")[3];
      if (id === "reorder") unavailable();
      return this.mutateStatus(id, method, data, context);
    }
    if (method === "GET" && path === "/api/runtimes") {
      keys(params, ["workspace_id", "owner"]);
      const host = bootstrap.runtime && typeof bootstrap.runtime === "object" ? record(bootstrap.runtime) : {};
      let runtimes: JsonRecord[] = (await this.scan("runtimes", context)).filter((r) => r.kind === "codex_page_host").map((r) => ({
        ...r, name: r.name ?? "Codex", status: r.status === "available" ? "online" : r.status, runtime_mode: "local",
        metadata: { ...(r.metadata && typeof r.metadata === "object" ? record(r.metadata) : {}), nativeTaskHostSupported: host.available === true && host.nativeTaskHostSupported === true && r.id === host.runtimeId && r.status === "available" && r.native_task_host_supported === true, nativeModelSelectionAuthoritative: host.available === true && host.nativeTaskHostSupported === true && r.id === host.runtimeId && r.status === "available" && r.native_task_host_supported === true, nativeHandoffSupported: host.available === true && host.nativeTaskHostSupported === true && r.id === host.runtimeId && r.status === "available" && r.native_task_host_supported === true },
        ...(r.owner_id === undefined && record(bootstrap.user).kind === "local_control_plane" && host.available === true && r.id === host.runtimeId ? { owner_id: context.userId } : {}),
      }));
      if (params.owner === "me") runtimes = runtimes.filter((r) => r.owner_id === context.userId);
      return response(runtimes);
    }
    if (method === "GET" && ["/api/skills", "/api/squads", "/api/projects"].includes(path)) {
      keys(params, ["workspace_id", "limit", "offset", "status"]);
      const resource = path.slice(5) as "skills" | "squads" | "projects";
      let items = await this.scan(resource, context);
      if (params.status) items = items.filter((item) => item.status === params.status);
      const result = page(items, params);
      return response(resource === "projects" ? { projects: result.items, total: result.total } : items);
    }
    if (method === "GET" && ["/api/agent-task-snapshot", "/api/agent-activity-30d", "/api/agent-run-counts", "/api/working-agents"].includes(path)) return this.agentActivity(path, params, context);
    if (method === "POST" && ["/api/issues/batch-update", "/api/issues/batch-delete"].includes(path)) {
      keys(data, ["issue_ids", "updates", ...controlFields]);
      const ids = list(data.issue_ids).map(identifier);
      if (!ids.length || ids.length > 100 || new Set(ids).size !== ids.length) throw new AdapterError(400, "invalid_batch");
      const updates = path.endsWith("batch-delete") ? {} : record(data.updates);
      keys(updates, [...fields.issues, ...controlFields, "title_base", "description_base", "suppress_run", "handoff_note", "attachment_ids"]);
      validateMutation("issues", updates, false);
      const completed: string[] = [];
      for (const id of ids) {
        try {
          const step = { operation: path, issueId: id };
          await this.mutate("issues", id, path.endsWith("batch-delete") ? "DELETE" : "PUT", updates, {
            ...context, receipt: undefined,
            commandId: await fingerprint({ commandId: context.commandId, step }),
            commandSignature: await fingerprint({ signature: context.commandSignature, step }),
          });
          completed.push(id);
        } catch (error) {
          const failure = error instanceof AdapterError ? error : bridgeError(error);
          throw new AdapterError(failure.status, failure.code, { ...failure.details, completed_ids: completed, failed_id: id });
        }
      }
      return response({ [path.endsWith("batch-delete") ? "deleted" : "updated"]: completed.length });
    }
    if (path === "/api/autopilots/cron-preview" && method === "GET") {
      keys(params, ["expr", "tz"]);
      if (typeof params.expr !== "string" || !params.expr || typeof params.tz !== "string" || !params.tz || params.expr.length > 256 || params.tz.length > 128) throw new AdapterError(400, "invalid_cron_request");
      const result = await this.call("/multica/autopilots/cron-preview", { expr: params.expr, tz: params.tz }, context.signal);
      if (!Array.isArray(result.next_runs) || !result.next_runs.every((value) => typeof value === "string" && timestamp(value))) throw new AdapterError(502, "invalid_cron_response");
      return response({ next_runs: result.next_runs });
    }
    if (["/api/autopilots/usage", "/api/issues/limit-usage"].includes(path) && method === "GET") return this.nativeDomains.usage(path, params, context);
    if (path === "/api/issues/preview-trigger" && method === "POST") return this.nativeDomains.preview(data, context);

    const parts = path.split("/").slice(2);
    const [domain, id, action, childId, childAction] = parts;
    if (domain === "autopilots") {
      const result = await this.webhooks.route(parts, method, params, data, context);
      if (result) return result;
    }
    const resource = Object.hasOwn(routeResources, domain) ? routeResources[domain] : undefined;
    if (parts.length === 1 && resource) {
      if (method === "POST") return this.mutate(resource, null, method, data, context);
      if (method === "GET") {
        keys(params, ["workspace_id", "limit", "offset", "status", "include_archived", "scope_type", "scope_id", "resource_type"]);
        let items = (await this.scan(resource, context)).map((item) => this.dto(resource, item, context));
        if (params.status) items = items.filter((item) => item.status === params.status);
        if (resource === "agents" && params.include_archived !== "true") items = items.filter((item) => !item.archived_at);
        if (resource === "quick_actions" && params.include_archived !== "true") items = items.filter((item) => item.status !== "archived");
        if (resource === "issue_views") items = items.filter((item) => (!params.scope_type || item.scope_type === params.scope_type) && (item.scope_id ?? null) === (params.scope_id ?? null));
        if (resource === "labels") items = items.filter((item) => (item.resource_type ?? "issue") === (params.resource_type ?? "issue"));
        const paged = page(items, params);
        return response(resource === "agents" || resource === "issue_views" ? items : { [resource]: paged.items, total: paged.total });
      }
    }
    if (!id) unavailable();
    identifier(id);
    if (domain === "issues" && action === "quick-actions" && parts.length === 5 && method === "POST" && ["render", "run"].includes(childAction)) {
      keys(params, []);
      return this.nativeDomains.quickAction(await this.find("issues", id, context, true), await this.find("quick_actions", childId, context, true), childAction, data, context);
    }
    if (domain === "issues" && action === "properties" && parts.length === 4 && ["PUT", "DELETE"].includes(method)) return this.issueProperty(id, childId, method, data, context);
    if ((domain === "issues" || domain === "comments") && action === "reactions" && parts.length === 3 && ["POST", "DELETE"].includes(method)) return this.socialMutation(domain, id, action, method, data, context);
    if (domain === "issues" && ["subscribe", "unsubscribe"].includes(action) && parts.length === 3 && method === "POST") return this.socialMutation(domain, id, action, method, data, context);
    if (domain === "autopilots" && action === "collaborators" && (parts.length === 3 && method === "POST" || parts.length === 4 && method === "DELETE")) return this.collaborators(id, childId, method, data, context);
    if (resource && parts.length === 2) {
      if (method === "GET") {
        keys(params, []);
        if (resource === "issues") {
          const found = (await this.executionBoard(context)).issues.find((issue) => issue.id === id || issue.identifier === id);
          if (!found) throw new AdapterError(404, "not_found");
          return response(found);
        }
        const found = this.dto(resource, await this.find(resource, id, context), context);
        if (resource === "autopilots") {
          found.triggers = await Promise.all(rows(found.triggers ?? []).map(async (trigger) => {
            if (!["webhook", "api"].includes(text(trigger.kind))) return trigger;
            const publicTrigger = webhookTrigger(await this.call("/multica/webhooks/trigger", { autopilotId: id, triggerId: trigger.id }, context.signal), id, text(trigger.id));
            return { ...publicTrigger, webhook_token: null };
          }));
        }
        return response(resource === "autopilots" ? { autopilot: found, triggers: found.triggers ?? [], ...(found.collaborators ? { collaborators: found.collaborators } : {}) } : found);
      }
      if (["PUT", "PATCH", "DELETE"].includes(method)) return this.mutate(resource, id, method, data, context);
    }
    if (domain === "issues" && action === "move" && parts.length === 3 && method === "POST") {
      keys(data, ["status", "assignee_type", "assignee_id", "parent_issue_id", "project_id", "before_id", "after_id", ...controlFields]);
      if (!Object.hasOwn(data, "before_id") || !Object.hasOwn(data, "after_id")) throw new AdapterError(400, "move_neighbors_required");
      const replay = await this.receipt(context);
      if (replay) return response(this.dto("issues", replay.entity, context));
      const current = await this.find("issues", id, context, true);
      const payload: JsonRecord = { issueId: current.id, expectedRevision: this.revision(data, current), beforeId: data.before_id, afterId: data.after_id, commandId: context.commandId, commandSignature: context.commandSignature };
      for (const [from, to] of [["status", "status"], ["assignee_type", "assigneeType"], ["assignee_id", "assigneeId"], ["parent_issue_id", "parentIssueId"], ["project_id", "projectId"]]) if (Object.hasOwn(data, from)) payload[to] = data[from];
      const moved = await this.call("/multica/workspace/move-issue", payload, context.signal);
      return response(this.dto("issues", moved.entity, context));
    }
    if (domain === "issues" && action === "labels" && (parts.length === 3 && method === "POST" || parts.length === 4 && method === "DELETE")) {
      keys(data, ["label_id", ...controlFields]);
      const labelId = identifier(childId ?? data.label_id);
      const label = await this.find("labels", labelId, context);
      if ((label.resource_type ?? "issue") !== "issue") throw new AdapterError(400, "invalid_label_resource");
      const current = await this.find("issues", id, context, true);
      const ids = new Set([...list(current.label_ids), ...rows(current.labels ?? []).map((item) => text(item.id))]);
      if (method === "POST") ids.add(labelId); else ids.delete(labelId);
      const saved = await this.save("issues", { ...current, label_ids: [...ids] }, this.revision(data, current), context);
      const catalog = await this.scan("labels", context);
      const savedIds = new Set(list(saved.label_ids));
      return response({ labels: catalog.filter((item) => savedIds.has(text(item.id))), issue_revision: saved.revision });
    }
    if (domain === "issues" && ["children", "comments", "labels", "subscribers", "timeline"].includes(action) && parts.length === 3) {
      const issue = await this.find("issues", id, context);
      if (method === "GET") {
        if (action === "children") return this.issueQuery("/api/issues/children", { parent_ids: id }, context);
        if (action === "labels") return response({ labels: issue.labels ?? [], issue_revision: issue.revision });
        if (action === "timeline") return response(rows(issue.timeline));
        return response((await this.scan(action as "comments" | "subscribers", context)).filter((item) => item.issue_id === id).map((item) => this.dto(action as "comments" | "subscribers", item, context)));
      }
      if (action === "comments" && method === "POST") {
        keys(data, [...fields.comments, ...controlFields]);
        if (typeof data.content !== "string" || !data.content.trim()) throw new AdapterError(400, "content_required");
        const now = new Date().toISOString();
        return response(await this.save("comments", { ...pick(data, fields.comments), id: context.commandId, workspace_id: context.workspaceId, revision: 1, issue_id: id, author_type: "member", author_id: context.userId, created_at: now, updated_at: now }, 0, context));
      }
    }
    if (domain === "autopilots" && action === "runs" && method === "GET" && parts.length <= 4) {
      keys(params, ["limit", "offset"]);
      if (childId) return response(runDto(record((await this.call("/multica/autopilots/run", { autopilotId: id, runId: identifier(childId) }, context.signal)).run)));
      const result = await this.call("/multica/autopilots/runs", { autopilotId: id }, context.signal);
      const paged = page(rows(result.runs).map(runDto), params);
      return response({ runs: paged.items, total: paged.total });
    }
    if (domain === "autopilots" && action === "trigger" && method === "POST" && parts.length === 3) {
      keys(data, controlFields);
      const current = await this.find("autopilots", id, context, true);
      if (current.can_write === false) throw new AdapterError(403, "permission_denied");
      const result = await this.call("/multica/autopilots/trigger", { autopilotId: id, occurrenceId: context.commandId, source: "manual" }, context.signal);
      return response(runDto(record(result.run)));
    }
    if (domain === "autopilots" && action === "triggers" && parts.length <= 4 && ["POST", "PATCH", "DELETE"].includes(method)) return this.autopilotTrigger(id, childId, method, data, context);
    if (domain === "agents" && ["archive", "restore"].includes(action) && method === "POST" && parts.length === 3) {
      keys(data, controlFields);
      const current = await this.find("agents", id, context, true);
      return response(await this.save("agents", { ...current, archived_at: action === "archive" ? new Date().toISOString() : null }, this.revision(data, current), context));
    }
    if (domain === "agents" && action === "skills" && parts.length <= 5) return this.agentSkills(id, childId, childAction, method, data, context);
    if (domain === "agents" && ["tasks", "cancel-tasks"].includes(action) && parts.length === 3) {
      keys(data, controlFields);
      const tasks = (await this.executions(context)).filter((task) => task.agent_id === id);
      if (action === "tasks" && method === "GET") return response(tasks);
      if (action === "cancel-tasks" && method === "POST") {
        let cancelled = 0;
        for (const task of tasks.filter((t) => activeStates.includes(text(t.status)))) {
          await this.taskAction("cancel", task, {}, { ...context, commandId: await fingerprint({ commandId: context.commandId, taskId: task.id }) });
          cancelled++;
        }
        return response({ cancelled });
      }
    }
    if (domain === "issues" && ["active-task", "task-runs", "rerun", "tasks"].includes(action)) {
      const tasks = await this.executions(context, id);
      if (method === "GET" && action === "active-task" && parts.length === 3) return response({ tasks: tasks.filter((t) => activeStates.includes(text(t.status))) });
      if (method === "GET" && action === "task-runs" && parts.length === 3) return response(tasks);
    if (method === "POST" && action === "tasks" && parts.length === 5 && ["cancel", "continue", "open"].includes(childAction)) {
        keys(data, [...controlFields, ...(childAction === "continue" ? ["prompt"] : [])]);
        const task = tasks.find((t) => t.id === childId);
        if (!task) throw new AdapterError(404, "not_found");
        return this.taskAction(childAction, task, data, context);
      }
      if (method === "POST" && action === "rerun" && parts.length === 3) {
        keys(data, ["task_id", ...controlFields]);
        const issue = await this.find("issues", id, context);
        const previous = data.task_id ? tasks.find((t) => t.id === data.task_id) : undefined;
        if (data.task_id && !previous) throw new AdapterError(404, "not_found");
        if (tasks.some((t) => activeStates.includes(text(t.status)))) throw new AdapterError(409, "execution_already_active");
        const agentId = previous?.agent_id ?? (issue.assignee_type === "agent" ? issue.assignee_id : null);
        if (!agentId) unavailable();
        const result = await this.call("/multica/executions/create", { workspaceId: context.workspaceId, issueId: id, agentId: identifier(agentId), prompt: [text(issue.title), text(issue.description)].filter(Boolean).join("\n\n"), idempotencyKey: context.commandId, executionKind: "thread" }, context.signal);
        return response(this.remember("tasks", taskDto(record(result.binding))));
      }
    }
    if (domain === "tasks" && ["messages", "open", "continue", "cancel", "status"].includes(action) && parts.length === 3) {
      if (!(method === "GET" && ["messages", "status"].includes(action) || method === "POST" && ["open", "continue", "cancel"].includes(action))) unavailable();
      keys(data, [...controlFields, ...(action === "continue" ? ["prompt"] : [])]);
      const task = this.seen.get(`tasks:${id}`) ?? (await this.executions(context)).find((t) => t.id === id);
      if (!task && action === "cancel") {
        const result = await this.builder.route(path, method, params, data, context);
        if (result) return result;
      }
      if (!task) throw new AdapterError(404, "not_found");
      return this.taskAction(action, task, data, context);
    }
    unavailable();
  }

  private async autopilotTrigger(id: string, childId: string | undefined, method: string, data: JsonRecord, context: RequestContext): Promise<Response> {
    keys(data, ["kind", "enabled", "cron_expression", "timezone", "label", "event_filters", ...controlFields]);
    if (data.enabled !== undefined && typeof data.enabled !== "boolean" || data.label !== undefined && (typeof data.label !== "string" || data.label.length > 512)) throw new AdapterError(400, "invalid_trigger_request");
    validateEventFilters(data.event_filters);
    if (data.kind !== undefined && !["api", "schedule", "webhook"].includes(text(data.kind))) throw new AdapterError(400, "invalid_trigger_kind");
    const replay = await this.receipt(context);
    if (replay) {
      if (method === "DELETE") return response(null, 204);
      const saved = this.dto("autopilots", replay.entity, context);
      const trigger = rows(saved.triggers).find((item) => item.id === (childId ?? context.commandId));
      if (!trigger) throw new AdapterError(502, "invalid_trigger_response");
      if (["webhook", "api"].includes(text(trigger.kind))) return response(method === "POST" ? await this.provisionTrigger(id, trigger, context) : webhookTrigger(await this.call("/multica/webhooks/trigger", { autopilotId: id, triggerId: trigger.id }, context.signal), id, text(trigger.id)));
      return response(trigger);
    }
    const current = await this.find("autopilots", id, context, true);
    if (current.can_write === false) throw new AdapterError(403, "permission_denied");
    const triggers = rows(current.triggers ?? []);
    const existing = childId ? triggers.find((t) => t.id === childId) : undefined;
    if (childId && !existing) throw new AdapterError(404, "not_found");
    const kind = data.kind ?? existing?.kind;
    if (kind !== "api" && kind !== "schedule" && kind !== "webhook") unavailable();
    if (existing && data.kind !== undefined && data.kind !== existing.kind) throw new AdapterError(400, "invalid_trigger_kind");
    const schedule: JsonRecord = {};
    if (kind === "schedule") {
      const expr = data.cron_expression ?? existing?.cron_expression, tz = data.timezone ?? existing?.timezone ?? "UTC";
      if (typeof expr !== "string" || !expr || expr.length > 256 || typeof tz !== "string" || !tz || tz.length > 128) throw new AdapterError(400, "invalid_cron_request");
      if (method !== "DELETE") {
        const preview = await this.call("/multica/autopilots/cron-preview", { expr, tz }, context.signal);
        if (!Array.isArray(preview.next_runs) || !preview.next_runs.every((value) => typeof value === "string" && timestamp(value))) throw new AdapterError(502, "invalid_cron_response");
      }
      Object.assign(schedule, { cron_expression: expr, timezone: tz });
    }
    const now = new Date().toISOString();
    const trigger: JsonRecord = { ...(existing ?? { id: context.commandId, autopilot_id: id, kind, enabled: true, created_at: now, webhook_token: null, cron_expression: null, timezone: null, next_run_at: null, last_fired_at: null }), ...pick(data, ["kind", "enabled", "label", "cron_expression", "timezone", "event_filters"]), ...schedule, updated_at: now };
    // A removed target fails ingress resolution even if cleanup is retried later.
    const updated = triggers.filter((t) => t.id !== childId);
    if (method !== "DELETE") updated.push(trigger);
    const saved = await this.save("autopilots", { ...current, triggers: updated }, this.revision(data, current), context);
    if (method === "DELETE") return response(null, 204);
    const persisted = rows(saved.triggers).find((item) => item.id === trigger.id);
    if (!persisted) throw new AdapterError(502, "invalid_trigger_response");
    if (kind === "webhook" || kind === "api") return response(method === "POST" ? await this.provisionTrigger(id, persisted, context) : webhookTrigger(await this.call("/multica/webhooks/trigger", { autopilotId: id, triggerId: persisted.id }, context.signal), id, text(persisted.id)));
    return response(persisted);
  }

  private async provisionTrigger(autopilotId: string, trigger: JsonRecord, context: RequestContext): Promise<JsonRecord> {
    const triggerId = identifier(trigger.id);
    try {
      const result = await this.call("/multica/webhooks/provision", { autopilotId, triggerId, commandId: await fingerprint({ commandId: context.commandId, step: "provision" }) }, context.signal);
      return webhookTrigger(result, autopilotId, triggerId);
    } catch (error) {
      const failure = error instanceof AdapterError ? error : bridgeError(error);
      throw new AdapterError(failure.status, failure.code, { ...failure.details, autopilot_id: autopilotId, trigger_id: triggerId, trigger_saved: true });
    }
  }

  private async agentActivity(path: string, params: JsonRecord, context: RequestContext): Promise<Response> {
    keys(params, path === "/api/working-agents" ? ["type", "scope", "relation", "parent"] : []);
    const board = ["/api/agent-task-snapshot", "/api/working-agents"].includes(path) ? await this.executionBoard(context) : null;
    const tasks = board ? board.tasks : await this.executions(context);
    if (path === "/api/agent-task-snapshot") {
      const latest = new Map<string, JsonRecord>();
      const active = tasks.filter((t) => ["queued", "running"].includes(text(t.status)));
      const activeAgents = new Set(active.map((t) => t.agent_id));
      for (const task of tasks) if (task.agent_id && !activeAgents.has(task.agent_id) && (!latest.has(text(task.agent_id)) || compareExecutionTasks(task, latest.get(text(task.agent_id))!) > 0)) latest.set(text(task.agent_id), task);
      return response([...active, ...latest.values()]);
    }
    if (path === "/api/working-agents") {
      const issues = board!.issues, actors: QueryContext = { userId: context.userId, ...board! };
      const running = tasks.filter((t) => t.status === "running" && t.agent_id).filter((t) => {
        const issue = issues.find((i) => i.id === t.issue_id);
        if (params.type && params.type !== "issue") return params.type === "autopilot" ? !!t.autopilot_run_id : !!t.chat_session_id;
        if (params.type === "issue" && !issue || params.parent && issue?.parent_issue_id !== params.parent) return false;
        if (params.scope !== "mine") return true;
        if (!issue) return false;
        const assigned = issue.assignee_type === "member" && issue.assignee_id === context.userId;
        const created = issue.creator_type === "member" && issue.creator_id === context.userId;
        const indirect = involved(issue, context.userId, actors);
        return params.relation === "assigned" ? assigned : params.relation === "created" ? created : params.relation === "involved" ? indirect : assigned || created || indirect;
      });
      return response(actors.agents.filter((a) => running.some((t) => t.agent_id === a.id)).map((a) => ({ id: a.id, name: a.name, avatar_url: a.avatar_url ?? null, running_task_count: running.filter((t) => t.agent_id === a.id).length, issue_ids: [...new Set(running.filter((t) => t.agent_id === a.id && t.issue_id).map((t) => t.issue_id))] })));
    }
    const cutoff = Date.now() - 30 * 86400000, buckets = new Map<string, JsonRecord>();
    for (const task of tasks) {
      const date = timestamp(path === "/api/agent-run-counts" ? task.created_at : task.completed_at);
      if (!task.agent_id || !date || Date.parse(date) < cutoff) continue;
      const bucket = `${date.slice(0, 10)}T00:00:00.000Z`;
      const key = path === "/api/agent-run-counts" ? text(task.agent_id) : `${task.agent_id}:${bucket}`;
      const value = buckets.get(key) ?? (path === "/api/agent-run-counts" ? { agent_id: task.agent_id, run_count: 0 } : { agent_id: task.agent_id, bucket_at: bucket, task_count: 0, failed_count: 0 });
      if (path === "/api/agent-run-counts") value.run_count = Number(value.run_count) + 1;
      else { value.task_count = Number(value.task_count) + 1; value.failed_count = Number(value.failed_count) + (task.status === "failed" ? 1 : 0); }
      buckets.set(key, value);
    }
    return response([...buckets.values()]);
  }

  private async agentSkills(agentId: string, skillId: string | undefined, action: string | undefined, method: string, data: JsonRecord, context: RequestContext): Promise<Response> {
    const result = await this.call("/multica/skills/bindings", { scopeKind: "agent", scopeId: agentId }, context.signal);
    const bindings = rows(result.bindings ?? result.items);
    if (method === "GET" && !skillId) {
      const inventory = await this.scan("skills", context);
      return response(bindings.map((binding) => {
        const id = record(binding.skillRef).id, skill = inventory.find((item) => item.id === id);
        if (!skill) throw new AdapterError(503, "skill_inventory_unavailable");
        return { ...skill, enabled: binding.enabled, revision: binding.revision };
      }));
    }
    if (method === "PUT" && !skillId) {
      keys(data, ["skill_ids", ...controlFields]);
      const skills = list(data.skill_ids).map((id) => ({ id: identifier(id) }));
      const expectedRevision = integer(data.expected_revision, Math.max(0, ...bindings.map((binding) => integer(binding.revision, 0, Number.MAX_SAFE_INTEGER))), Number.MAX_SAFE_INTEGER);
      await this.call("/multica/skills/bindings/replace", { scopeKind: "agent", scopeId: agentId, skills, expectedRevision }, context.signal);
      return response(null, 204);
    }
    if (skillId && (method === "DELETE" && !action || method === "PUT" && action === "enabled")) {
      keys(data, [...controlFields, ...(method === "PUT" ? ["enabled"] : [])]);
      const binding = bindings.find((item) => record(item.skillRef).id === skillId);
      if (!binding) throw new AdapterError(404, "not_found");
      if (method === "PUT" && typeof data.enabled !== "boolean") throw new AdapterError(400, "invalid_enabled");
      await this.call(method === "PUT" ? "/multica/skills/bind" : "/multica/skills/unbind", { scopeKind: "agent", scopeId: agentId, expectedRevision: this.revision(data, binding), ...(method === "PUT" ? { skillRef: { id: identifier(skillId) }, enabled: data.enabled } : { skillId: identifier(skillId) }) }, context.signal);
      return response(null, 204);
    }
    unavailable();
  }

  readonly transport = async (path: string, init: RequestInit = {}): Promise<Response> => {
    try {
      if (init.signal?.aborted) throw new AdapterError(408, "request_aborted");
      if (typeof path !== "string" || path.length > MAX_BODY_BYTES || !path.startsWith("/api/") || /[\\#\u0000-\u0020]/.test(path)) throw new AdapterError(400, "invalid_api_path");
      const [rawPath, search = ""] = path.split("?");
      const segments = rawPath.split("/").slice(2).map((segment) => { try { return identifier(decodeURIComponent(segment)); } catch { throw new AdapterError(400, "invalid_api_path"); } });
      const pathname = `/api/${segments.join("/")}`;
      const method = text(init.method, "GET").toUpperCase();
      if (!["GET", "POST", "PUT", "PATCH", "DELETE"].includes(method)) throw new AdapterError(405, "method_not_allowed");
      const headers = new Headers(init.headers);
      for (const key of headers.keys()) if (!["content-type", "accept", "idempotency-key", "x-request-id", "x-workspace-slug"].includes(key) && !(key === "x-client-capabilities" && method === "POST" && /^\/api\/tasks\/[^/]+\/cancel$/.test(pathname) && headers.get(key) === "chat-draft-restore-v1")) throw new AdapterError(400, "unsupported_header");
      if (init.body !== undefined && init.body !== null && typeof init.body !== "string") throw new AdapterError(400, "invalid_json_request");
      if (typeof init.body === "string" && new TextEncoder().encode(init.body).length > MAX_BODY_BYTES) throw new AdapterError(413, "request_too_large");
      let data: JsonRecord = {};
      if (init.body) { try { data = record(JSON.parse(String(init.body))); } catch { throw new AdapterError(400, "invalid_json_request"); } }
      boundedJson(data);
      if (method === "GET" && Object.keys(data).length) throw new AdapterError(400, "unexpected_body");
      const params: JsonRecord = {};
      for (const [key, value] of new URLSearchParams(search)) {
        if (Object.hasOwn(params, key)) throw new AdapterError(400, "duplicate_query_parameter");
        params[key] = value;
      }
      const read = method === "GET" || method === "POST" && (["/api/issues/query", "/api/issues/table/groups", "/api/issues/table/rows", "/api/issues/table/facets", "/api/issues/preview-trigger"].includes(pathname) || /^\/api\/issues\/[^/]+\/quick-actions\/[^/]+\/render$/.test(pathname));
      if (!read) {
        const projectionId = (value: unknown) => isVirtualIssue(value) || typeof value === "string" && /^(codex-native-agent|ccp-execution-agent):/.test(value);
        const references = (value: JsonRecord): boolean => Object.entries(value).some(([key, item]) =>
          ["issue_id", "parent_issue_id", "before_id", "after_id", "agent_id", "assignee_id"].includes(key) && projectionId(item)
          || key === "issue_ids" && Array.isArray(item) && item.some(projectionId)
          || key === "updates" && !!item && typeof item === "object" && !Array.isArray(item) && references(record(item)));
        if (segments.some(projectionId) || references(data)) throw new AdapterError(403, "read_only_projection");
      }
      const bootstrap = await this.bootstrap(init.signal), workspace = record(bootstrap.workspace), user = record(bootstrap.user);
      if (params.workspace_id !== undefined && params.workspace_id !== workspace.id || data.workspace_id !== undefined && data.workspace_id !== workspace.id || headers.has("x-workspace-slug") && headers.get("x-workspace-slug") !== workspace.slug) throw new AdapterError(403, "workspace_mismatch");
      const provided = [data.command_id, data.idempotency_key, headers.get("idempotency-key")].filter((value) => value != null);
      if (new Set(provided).size > 1) throw new AdapterError(400, "idempotency_key_mismatch");
      const commandId = identifier(provided[0] ?? this.requestIds.get(init) ?? crypto.randomUUID());
      this.requestIds.set(init, commandId);
      const context: RequestContext = { workspaceId: identifier(workspace.id), userId: identifier(user.id), commandId, signal: init.signal };
      if (read) return await this.route(pathname, method, params, data, context, bootstrap);
      const signature = await fingerprint({ path: pathname, method, params, data });
      context.commandSignature = signature;
      const existing = this.commands.get(commandId);
      if (existing) {
        if (existing.signature !== signature) throw new AdapterError(409, "idempotency_conflict");
        return (await existing.promise).clone();
      }
      if (this.commands.size >= 1024) throw new AdapterError(503, "command_window_full");
      const promise = this.route(pathname, method, params, data, context, bootstrap);
      this.commands.set(commandId, { signature, promise });
      // Keep ambiguous failures: retries must not silently change CAS or create another execution.
      return (await promise).clone();
    } catch (error) {
      const failure = error instanceof AdapterError ? error : bridgeError(error);
      return response({ code: failure.code, error: failure.code, ...failure.details }, failure.status);
    }
  };
}
