import { z } from "zod";

export type JsonRecord = Record<string, unknown>;
export const MAX_BODY_BYTES = 32 * 1024;
export const MAX_SCAN_ITEMS = 5000;
export const categories = ["backlog", "todo", "in_progress", "in_review", "done", "blocked", "cancelled"] as const;

export class AdapterError extends Error {
  constructor(readonly status: number, readonly code: string, readonly details: JsonRecord = {}) {
    super(code);
  }
}

export function record(value: unknown): JsonRecord {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new AdapterError(502, "invalid_bridge_response");
  return value as JsonRecord;
}

export function text(value: unknown, fallback = ""): string {
  return typeof value === "string" ? value : fallback;
}

export function nullable(value: unknown): string | null {
  return typeof value === "string" && value.length ? value : null;
}

export function integer(value: unknown, fallback: number, max = 100000): number {
  if (value === undefined || value === null) return fallback;
  if (value === "" || !["string", "number"].includes(typeof value)) throw new AdapterError(400, "invalid_integer");
  const result = Number(value);
  if (!Number.isSafeInteger(result) || result < 0 || result > max) throw new AdapterError(400, "invalid_integer");
  return result;
}

export function identifier(value: unknown): string {
  if (typeof value !== "string" || !/^[A-Za-z0-9_.:@-]{1,240}$/.test(value) || value === "." || value === "..") {
    throw new AdapterError(400, "invalid_identifier");
  }
  return value;
}

export function pick(value: JsonRecord, fields: readonly string[]): JsonRecord {
  return Object.fromEntries(fields.filter((key) => Object.hasOwn(value, key)).map((key) => [key, value[key]]));
}

export function keys(value: JsonRecord, allowed: readonly string[]): void {
  if (Object.keys(value).some((key) => !allowed.includes(key))) throw new AdapterError(400, "unsupported_request_field");
}

const sensitive = /^(?:authorization|headers?|extraHeaders|token|api_key|apiKey|access_token|custom_env|env|environment|cwd|path|local_path|work_dir|command|shell|runtime_action|can_write|can_manage_access|permissions|role|trusted|installed|compatible|__proto__|constructor|prototype)$/i;
export function boundedJson(value: unknown, depth = 0): void {
  if (depth > 16) throw new AdapterError(400, "request_too_deep");
  if (Array.isArray(value)) {
    if (value.length > MAX_SCAN_ITEMS) throw new AdapterError(413, "request_too_large");
    value.forEach((entry) => boundedJson(entry, depth + 1));
  } else if (value && typeof value === "object") {
    for (const [key, entry] of Object.entries(value)) {
      if (sensitive.test(key)) throw new AdapterError(400, "unsupported_request_field");
      boundedJson(entry, depth + 1);
    }
  }
}

const idSchema = z.string().min(1).max(240).refine((value) => {
  try { identifier(value); return true; } catch { return false; }
});
const shortText = z.string().max(512);
const dateSchema = z.string().regex(/^\d{4}-\d{2}-\d{2}$/).nullable();
const actorSchema = z.enum(["member", "agent", "squad"]).nullable();
const baseMutationSchema = z.object({
  expected_revision: z.number().int().positive().optional(), command_id: idSchema.optional(), idempotency_key: idSchema.optional(),
}).passthrough();
const mutationSchemas = {
  quick_actions: baseMutationSchema.extend({
    name: z.string().trim().min(1).max(128).optional(), description: z.string().max(4096).optional(),
    assignee_type: z.enum(["agent", "squad"]).optional(), assignee_id: idSchema.optional(),
    prompt: z.string().trim().min(1).max(24000).refine((value) => !/\{\{[^}]*\}\}/.test(value)).optional(),
    visibility: z.enum(["private", "public"]).optional(), status: z.enum(["active", "archived"]).optional(),
  }),
  properties: baseMutationSchema.extend({
    name: z.string().trim().min(1).max(128).optional(),
    type: z.enum(["text", "number", "select", "multi_select", "date", "checkbox", "url", "actor", "multi_actor"]).optional(),
    description: z.string().max(4096).optional(), icon: shortText.optional(), archived: z.boolean().optional(),
    config: z.object({ options: z.array(z.object({ id: idSchema, name: z.string().trim().min(1).max(128), color: z.string().regex(/^#[0-9a-fA-F]{6}$/).optional() }).strict()).max(256).optional() }).strict().optional(),
  }),
  issues: baseMutationSchema.extend({
    title: z.string().trim().min(1).max(2048).optional(), description: z.string().max(28000).nullable().optional(),
    status: idSchema.optional(), priority: z.enum(["urgent", "high", "medium", "low", "none"]).optional(),
    assignee_type: actorSchema.optional(), assignee_id: idSchema.nullable().optional(),
    parent_issue_id: idSchema.nullable().optional(), project_id: idSchema.nullable().optional(),
    stage: z.number().int().positive().nullable().optional(), position: z.number().finite().optional(),
    start_date: dateSchema.optional(), due_date: dateSchema.optional(), label_ids: z.array(idSchema).max(128).optional(),
    metadata: z.record(z.string(), z.union([z.string(), z.number(), z.boolean()])).optional(),
    properties: z.record(z.string(), z.union([z.string(), z.number(), z.boolean(), z.array(z.string()), z.null()])).optional(),
    suppress_run: z.boolean().optional(), handoff_note: z.string().max(4096).optional(), attachment_ids: z.array(idSchema).max(128).optional(),
  }),
  agents: baseMutationSchema.extend({
    name: z.string().trim().min(1).max(128).optional(), description: z.string().max(8192).optional(), instructions: z.string().max(24000).optional(),
    runtime_id: idSchema.optional(), permission_mode: z.enum(["private", "public_to"]).optional(), visibility: z.enum(["private", "workspace"]).optional(),
    invocation_targets: z.array(z.object({ target_type: z.enum(["workspace", "member", "team"]), target_id: idSchema.nullable().optional() }).strict()).max(256).optional(),
    max_concurrent_tasks: z.number().int().min(1).max(50).optional(), model: shortText.optional(), thinking_level: shortText.optional(), service_tier: shortText.optional(),
    skill_ids: z.array(idSchema).max(64).optional(), avatar_url: z.string().max(2048).nullable().optional(),
    conversation_starters: z.array(z.object({ label: shortText, prompt: z.string().max(8192) }).strict()).max(20).optional(), template: shortText.optional(),
  }),
  autopilots: baseMutationSchema.extend({
    title: z.string().trim().min(1).max(2048).optional(), description: z.string().max(24000).nullable().optional(),
    assignee_id: idSchema.optional(), assignee_type: z.enum(["agent", "squad"]).optional(), project_id: idSchema.nullable().optional(),
    status: z.enum(["active", "paused", "archived"]).optional(), execution_mode: z.enum(["create_issue", "run_only"]).optional(),
    issue_title_template: shortText.nullable().optional(), subscribers: z.array(z.object({ user_type: z.literal("member"), user_id: idSchema }).strict()).max(256).optional(),
  }),
  issue_views: baseMutationSchema.extend({ name: z.string().trim().min(1).max(128).optional(), scope_type: z.enum(["workspace", "my", "project"]).optional(), scope_id: idSchema.nullable().optional(), scope_variant: shortText.nullable().optional(), visibility: z.enum(["private", "workspace"]).optional(), definition_version: z.number().int().positive().optional(), query: z.record(z.string(), z.unknown()).optional(), display: z.record(z.string(), z.unknown()).optional() }),
  comments: baseMutationSchema.extend({ content: z.string().trim().min(1).max(28000).optional(), type: shortText.optional(), parent_id: idSchema.nullable().optional() }),
  labels: baseMutationSchema.extend({ name: z.string().trim().min(1).max(128).optional(), color: z.string().regex(/^#[0-9a-fA-F]{6}$/).optional(), description: shortText.nullable().optional(), resource_type: z.enum(["issue", "agent", "skill"]).optional() }),
  issue_statuses: baseMutationSchema.extend({
    name: z.string().trim().min(1).max(64).optional(), description: z.string().max(256).optional(),
    key: z.string().max(80).optional(), category: z.enum(categories).optional(),
    color: z.string().regex(/^#[0-9a-fA-F]{6}$/).optional(), position: z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER).optional(),
  }),
};

export function validateMutation(resource: keyof typeof mutationSchemas, data: JsonRecord, create: boolean): void {
  if (!mutationSchemas[resource].safeParse(data).success) throw new AdapterError(400, "invalid_mutation");
  const required = resource === "issues" || resource === "autopilots" ? "title" : resource === "comments" ? "content" : "name";
  if (create && (typeof data[required] !== "string" || !text(data[required]).trim())) throw new AdapterError(400, "invalid_mutation");
  if (create && resource === "autopilots" && (!data.assignee_id || !data.execution_mode)) throw new AdapterError(400, "invalid_mutation");
  if (create && resource === "issue_statuses" && (!data.category || !data.color)) throw new AdapterError(400, "invalid_mutation");
  if (create && resource === "properties" && !data.type) throw new AdapterError(400, "invalid_mutation");
  if (create && resource === "quick_actions" && (!data.assignee_type || !data.assignee_id || !data.prompt)) throw new AdapterError(400, "invalid_mutation");
  if (resource === "properties" && data.config) {
    const options = record(data.config).options;
    if (Array.isArray(options) && new Set(options.map((option) => record(option).id)).size !== options.length) throw new AdapterError(400, "invalid_property_options");
  }
}

export function propertyDto(source: JsonRecord, workspaceId: string): JsonRecord {
  entity(source);
  if (typeof source.name !== "string" || typeof source.type !== "string") throw new AdapterError(502, "invalid_property_response");
  return { ...source, workspace_id: text(source.workspace_id, workspaceId), description: text(source.description), icon: text(source.icon),
    config: source.config ?? {}, position: source.position ?? 0, archived: source.archived ?? false, archived_at: source.archived_at ?? null,
    created_at: timestamp(source.created_at_ms ?? source.created_at) ?? "", updated_at: timestamp(source.updated_at_ms ?? source.updated_at) ?? "" };
}

export function preferenceDto(source: JsonRecord): JsonRecord {
  const prefs = record(source.prefs);
  if (!["workspace", "my", "project"].includes(text(source.scope_type)) || ![prefs.hidden, prefs.order].every((value) => Array.isArray(value) && value.every((id) => typeof id === "string"))) throw new AdapterError(502, "invalid_preference_response");
  return { scope_type: source.scope_type, scope_id: source.scope_id ?? null, prefs, updated_at: timestamp(source.updated_at_ms ?? source.updated_at) ?? "", ...(source.revision ? { revision: source.revision } : {}) };
}

// Mirrors upstream issuestatus.DeriveKey, including archived keys and non-Latin names.
export function statusKey(data: JsonRecord, catalog: JsonRecord[]): string {
  const explicit = text(data.key).trim().toLowerCase();
  if (explicit) {
    if (!/^[a-z0-9][a-z0-9_]{0,31}$/.test(explicit) || categories.some((category) => category === explicit)) throw new AdapterError(400, "invalid_status_key");
    return explicit;
  }
  const slug = text(data.name).trim().toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_+|_+$/g, "").slice(0, 32).replace(/_+$/g, "");
  if (categories.some((category) => category === slug)) throw new AdapterError(400, "invalid_status_key");
  const taken = new Set([...categories, ...catalog.map((entry) => text(entry.key))]);
  const base = slug || text(data.category);
  if (!taken.has(base)) return base;
  for (let n = 2; n <= taken.size + 2; n++) {
    const suffix = `_${n}`, candidate = base.slice(0, 32 - suffix.length).replace(/_+$/g, "") + suffix;
    if (!taken.has(candidate)) return candidate;
  }
  throw new AdapterError(409, "status_key_conflict");
}

export const collectionSchema = z.object({
  items: z.array(z.record(z.string(), z.unknown())).max(100),
  total: z.number().int().nonnegative(),
  stale: z.boolean().optional(), diagnostic: z.string().optional(),
}).passthrough();

const entitySchema = z.object({ id: z.string().min(1), revision: z.number().int().positive() }).passthrough();
export function entity(value: unknown): JsonRecord {
  const parsed = entitySchema.safeParse(value);
  if (!parsed.success) throw new AdapterError(502, "invalid_entity_response");
  return parsed.data;
}

export function timestamp(value: unknown): string | null {
  if (typeof value === "string" && Number.isFinite(Date.parse(value))) return value;
  if (typeof value === "number" && Number.isFinite(value) && value >= 0 && value <= 8640000000000000) return new Date(value).toISOString();
  return null;
}

export function issueDto(source: JsonRecord, workspaceId: string): JsonRecord {
  entity(source);
  if (typeof source.title !== "string") throw new AdapterError(502, "invalid_issue_response");
  const status = text(source.status, "todo");
  return {
    ...source, workspace_id: text(source.workspace_id, workspaceId),
    number: source.number ?? 0, identifier: text(source.identifier, text(source.id)),
    description: nullable(source.description), status,
    ...(source.status_category ? {} : categories.includes(status as typeof categories[number]) ? { status_category: status } : {}),
    priority: text(source.priority, "none"), assignee_type: nullable(source.assignee_type), assignee_id: nullable(source.assignee_id),
    creator_type: text(source.creator_type, "member"), creator_id: text(source.creator_id),
    parent_issue_id: nullable(source.parent_issue_id), project_id: nullable(source.project_id),
    position: source.position ?? 0, stage: source.stage ?? null,
    start_date: nullable(source.start_date), due_date: nullable(source.due_date),
    metadata: source.metadata ?? {}, properties: source.properties ?? {}, labels: source.labels ?? [],
    created_at: timestamp(source.created_at_ms ?? source.created_at) ?? "", updated_at: timestamp(source.updated_at_ms ?? source.updated_at) ?? "",
  };
}

export function statusDto(source: JsonRecord, workspaceId: string): JsonRecord {
  entity(source);
  if (typeof source.key !== "string" || typeof source.name !== "string" || !categories.includes(source.category as typeof categories[number])) throw new AdapterError(502, "invalid_status_response");
  return { ...source, workspace_id: text(source.workspace_id, workspaceId), description: text(source.description),
    archived_at: nullable(source.archived_at), created_at: timestamp(source.created_at_ms ?? source.created_at) ?? "", updated_at: timestamp(source.updated_at_ms ?? source.updated_at) ?? "" };
}

export function autopilotDto(source: JsonRecord, workspaceId: string): JsonRecord {
  entity(source);
  if (typeof source.title !== "string" || typeof source.assignee_id !== "string") throw new AdapterError(502, "invalid_autopilot_response");
  return {
    ...source, workspace_id: text(source.workspace_id, workspaceId), description: nullable(source.description),
    project_id: nullable(source.project_id), assignee_type: text(source.assignee_type, "agent"),
    status: text(source.status, "active"), execution_mode: text(source.execution_mode, "create_issue"),
    issue_title_template: nullable(source.issue_title_template), created_by_type: text(source.created_by_type, "member"),
    created_by_id: text(source.created_by_id), last_run_at: timestamp(source.last_run_at),
    created_at: timestamp(source.created_at_ms ?? source.created_at) ?? "", updated_at: timestamp(source.updated_at_ms ?? source.updated_at) ?? "",
  };
}

export function agentDto(source: JsonRecord, workspaceId: string): JsonRecord {
  entity(source);
  if (typeof source.name !== "string") throw new AdapterError(502, "invalid_agent_response");
  // Availability and invocation grants are never inferred from a saved definition.
  return {
    ...source, workspace_id: text(source.workspace_id, workspaceId), description: text(source.description), instructions: text(source.instructions),
    avatar_url: nullable(source.avatar_url), runtime_id: text(source.runtime_id), owner_id: nullable(source.owner_id),
    archived_at: nullable(source.archived_at), skills: source.skills ?? [],
    custom_args: source.custom_args ?? [], runtime_config: source.runtime_config ?? {},
    created_at: timestamp(source.created_at_ms ?? source.created_at) ?? "", updated_at: timestamp(source.updated_at_ms ?? source.updated_at) ?? "",
  };
}

export function runDto(source: JsonRecord): JsonRecord {
  const id = source.id;
  const autopilotId = source.autopilot_id ?? source.autopilotId;
  const createdAt = timestamp(source.created_at ?? source.createdAtMs);
  const triggeredAt = timestamp(source.triggered_at ?? source.triggeredAtMs);
  if (typeof id !== "string" || typeof autopilotId !== "string" || !createdAt || !triggeredAt || typeof source.status !== "string") {
    throw new AdapterError(502, "invalid_run_response");
  }
  return {
    id, autopilot_id: autopilotId, trigger_id: source.trigger_id ?? source.triggerId ?? null,
    source: source.source, status: source.status, revision: source.revision,
    issue_id: source.issue_id ?? source.issueId ?? null, task_id: source.task_id ?? source.taskId ?? null,
    triggered_at: triggeredAt, created_at: createdAt, completed_at: timestamp(source.completed_at ?? source.completedAtMs),
    failure_reason: source.failure_reason ?? source.failureReason ?? null,
    reason_code: source.reason_code ?? source.reasonCode,
    trigger_payload: source.trigger_payload ?? null, result: source.result ?? null,
  };
}

export function taskDto(source: JsonRecord): JsonRecord {
  if (typeof source.id === "string" && typeof source.status === "string") return source;
  const createdAt = timestamp(source.createdAtMs);
  if (typeof source.bindingId !== "string" || !createdAt || typeof source.state !== "string") throw new AdapterError(502, "invalid_execution_response");
  const state = source.state;
  return {
    id: source.bindingId, agent_id: text(source.agentId), runtime_id: text(source.codexRuntimeId), issue_id: text(source.issueId),
    status: state === "binding_pending" ? "queued" : state,
    priority: 0, dispatched_at: timestamp(source.dispatchedAtMs), started_at: timestamp(source.startedAtMs),
    completed_at: timestamp(source.completedAtMs), result: null, error: nullable(source.lastErrorCode),
    ...(source.lastErrorCode ? { failure_reason: text(source.lastErrorCode) } : {}), created_at: createdAt, revision: source.revision,
    attempt: source.attemptNo, binding_id: source.bindingId, thread_id: source.codexThreadId ?? null,
  };
}

export function bridgeError(value: unknown): AdapterError {
  const source = value && typeof value === "object" ? value as JsonRecord : {};
  const candidate = text(source.code, text(source.message));
  const code = /^[a-z][a-z0-9_]{0,95}$/.test(candidate) ? candidate : "runtime_unavailable";
  const explicit = Number(source.statusCode ?? source.httpStatus ?? (typeof source.status === "number" ? source.status : 0));
  const status = [400, 401, 403, 404, 408, 409, 413, 429, 502, 503, 504].includes(explicit) ? explicit
    : /capacity|quota.*(?:exceeded|reached)/.test(code) ? 429
    : /conflict|revision|idempotency|not_active|task_active|property_archived/.test(code) ? 409
    : /permission|forbidden|denied|tenant|not_trusted|owner_required|creator_required|system_issue_status_immutable/.test(code) ? 403
    : /not_found|session_deleted/.test(code) ? 404
    : /timeout/.test(code) ? 504
    : /too_large/.test(code) ? 413
    : /invalid|sensitive_field|immutable/.test(code) ? 400 : 503;
  return new AdapterError(status, code, pick(source, ["expected_revision", "actual_revision", "current_revision", "retryable"]));
}
