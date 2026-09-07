/*
 * Codex Runtime Adapter for the derived Multica UI.
 *
 * This is deliberately a small allowlist rather than a fetch shim.  The UI
 * receives upstream-shaped HTTP responses, but every persistent operation is
 * translated to the CCP local control plane and retains its revision check.
 */
import type { WorkflowSurfaceBridge } from "./runtime-bridge";

type JsonRecord = Record<string, unknown>;

const statusCategories = [
  "backlog",
  "todo",
  "in_progress",
  "in_review",
  "done",
  "blocked",
  "cancelled",
] as const;

function now(): string {
  return new Date().toISOString();
}

function object(value: unknown): JsonRecord {
  return value && typeof value === "object" && !Array.isArray(value)
    ? value as JsonRecord
    : {};
}

function string(value: unknown, fallback = ""): string {
  return typeof value === "string" && value.trim() ? value : fallback;
}

function number(value: unknown, fallback = 0): number {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function nullableString(value: unknown): string | null {
  return typeof value === "string" && value.trim() ? value : null;
}

function jsonResponse(value: unknown, status = 200): Response {
  return new Response(status === 204 ? undefined : JSON.stringify(value), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

function errorResponse(status: number, code: string, error: string): Response {
  return jsonResponse({ code, error }, status);
}

async function readBody(init: RequestInit): Promise<JsonRecord> {
  if (typeof init.body !== "string" || !init.body) return {};
  try {
    return object(JSON.parse(init.body));
  } catch {
    throw new Error("invalid_json_request");
  }
}

function collection(result: JsonRecord): JsonRecord[] {
  return Array.isArray(result.items) ? result.items.map(object) : [];
}

function workspaceFromBootstrap(result: JsonRecord): JsonRecord {
  const source = object(result.workspace);
  const createdAt = now();
  return {
    id: string(source.id, "local-workspace"),
    slug: string(source.slug, "local-workspace"),
    name: string(source.name, "Local Codex Workspace"),
    description: null,
    context: null,
    settings: {},
    repos: [],
    issue_prefix: "CODEX",
    avatar_url: null,
    created_at: createdAt,
    updated_at: createdAt,
  };
}

function userFromBootstrap(result: JsonRecord): JsonRecord {
  const source = object(result.user);
  const timestamp = now();
  return {
    id: string(source.id, "local-codex-user"),
    name: "Codex 本地用户",
    email: "local@codex.invalid",
    avatar_url: null,
    onboarded_at: timestamp,
    onboarding_questionnaire: {},
    starter_content_state: "imported",
    language: "zh-CN",
    profile_description: "",
    timezone: null,
    created_at: timestamp,
    updated_at: timestamp,
  };
}

function issueFromEntity(entity: JsonRecord, workspaceId: string, userId: string): JsonRecord {
  const timestamp = now();
  const status = string(entity.status, "todo");
  return {
    ...entity,
    id: string(entity.id),
    workspace_id: string(entity.workspace_id, workspaceId),
    number: number(entity.number, 0),
    identifier: string(entity.identifier, string(entity.id)),
    title: string(entity.title, "未命名任务"),
    description: nullableString(entity.description),
    status,
    status_category: statusCategories.includes(status as typeof statusCategories[number]) ? status : undefined,
    priority: string(entity.priority, "none"),
    assignee_type: nullableString(entity.assignee_type),
    assignee_id: nullableString(entity.assignee_id),
    creator_type: string(entity.creator_type, "member"),
    creator_id: string(entity.creator_id, userId),
    parent_issue_id: nullableString(entity.parent_issue_id),
    project_id: nullableString(entity.project_id),
    position: number(entity.position, 0),
    stage: entity.stage === null ? null : number(entity.stage, 0),
    start_date: nullableString(entity.start_date),
    due_date: nullableString(entity.due_date),
    metadata: object(entity.metadata),
    properties: object(entity.properties),
    labels: Array.isArray(entity.labels) ? entity.labels : [],
    created_at: string(entity.created_at, timestamp),
    updated_at: string(entity.updated_at, timestamp),
    revision: number(entity.revision, 1),
  };
}

function statusFromEntity(entity: JsonRecord, workspaceId: string): JsonRecord {
  const timestamp = now();
  const category = string(entity.category, "todo");
  return {
    ...entity,
    id: string(entity.id),
    workspace_id: string(entity.workspace_id, workspaceId),
    key: string(entity.key, category),
    name: string(entity.name, category),
    description: string(entity.description),
    category,
    color: string(entity.color, "#6B7280"),
    is_system: entity.is_system === true,
    position: number(entity.position),
    archived_at: nullableString(entity.archived_at),
    created_at: string(entity.created_at, timestamp),
    updated_at: string(entity.updated_at, timestamp),
  };
}

function responseFailure(result: JsonRecord): Response | null {
  if (result.status !== "failed") return null;
  const code = string(result.code, "capability_unavailable");
  const message = string(result.message, "当前 Codex 执行能力不可用");
  const status = code.includes("revision") ? 409 : code.includes("permission") ? 403 : 503;
  return errorResponse(status, code, message);
}

export class MulticaApiAdapter {
  private bootstrapSnapshot: JsonRecord | null = null;

  constructor(private readonly bridge: WorkflowSurfaceBridge) {}

  private async call(path: string, payload: JsonRecord): Promise<JsonRecord> {
    const result = object(await this.bridge.postJson(path, payload));
    const failure = responseFailure(result);
    if (failure) {
      const data = await failure.json();
      throw new AdapterHttpError(failure.status, string(data.error), data);
    }
    return result;
  }

  private async bootstrap(): Promise<JsonRecord> {
    if (!this.bootstrapSnapshot) {
      this.bootstrapSnapshot = await this.call("/multica/workspace/bootstrap", {});
    }
    return this.bootstrapSnapshot;
  }

  private async query(resource: string, limit = 100, offset = 0): Promise<JsonRecord> {
    return this.call("/multica/workspace/query", { resource, limit, offset });
  }

  async transport(path: string, init: RequestInit): Promise<Response> {
    try {
      const url = new URL(path, "https://codex.local");
      const method = (init.method ?? "GET").toUpperCase();
      const bootstrap = await this.bootstrap();
      const workspace = workspaceFromBootstrap(bootstrap);
      const user = userFromBootstrap(bootstrap);
      const workspaceId = string(workspace.id);
      const userId = string(user.id);

      if (method === "GET" && url.pathname === "/api/me") return jsonResponse(user);
      if (method === "GET" && url.pathname === "/api/workspaces") return jsonResponse([workspace]);
      if (method === "GET" && url.pathname === `/api/workspaces/${workspaceId}/members`) {
        return jsonResponse([{ id: `${userId}-member`, workspace_id: workspaceId, user_id: userId, role: "owner", created_at: now(), name: user.name, email: user.email, avatar_url: null }]);
      }
      if (method === "GET" && url.pathname === "/api/issue-statuses") {
        const result = await this.query("issue_statuses");
        const statuses = collection(result).map((item) => statusFromEntity(item, workspaceId));
        return jsonResponse({ statuses, categories: [...statusCategories], total: statuses.length });
      }
      if (method === "GET" && url.pathname === "/api/properties") return jsonResponse({ properties: [], total: 0 });
      if (method === "GET" && url.pathname === "/api/issue-views") return jsonResponse({ views: [], total: 0 });
      if (method === "GET" && url.pathname === "/api/issues") {
        const result = await this.query("issues", Number(url.searchParams.get("limit") ?? 100), Number(url.searchParams.get("offset") ?? 0));
        const issues = collection(result).map((item) => issueFromEntity(item, workspaceId, userId));
        return jsonResponse({ issues, total: number(result.total, issues.length) });
      }
      if (method === "POST" && url.pathname === "/api/issues/query") {
        const result = await this.query("issues");
        const issues = collection(result).map((item) => issueFromEntity(item, workspaceId, userId));
        return jsonResponse({ issues, total: number(result.total, issues.length) });
      }
      if (method === "POST" && url.pathname === "/api/issues") {
        const data = await readBody(init);
        const id = crypto.randomUUID();
        const entity = {
          ...data,
          id,
          workspace_id: workspaceId,
          revision: 1,
          number: Date.now(),
          identifier: `CODEX-${Date.now()}`,
          title: string(data.title, "未命名任务"),
          status: string(data.status, "todo"),
          priority: string(data.priority, "none"),
          assignee_type: nullableString(data.assignee_type),
          assignee_id: nullableString(data.assignee_id),
          creator_type: "member",
          creator_id: userId,
          position: Date.now(),
          metadata: object(data.metadata),
          properties: object(data.properties),
          created_at: now(),
          updated_at: now(),
        };
        const saved = object(await this.call("/multica/workspace/upsert", { resource: "issues", entity, expectedRevision: 0 }));
        return jsonResponse(issueFromEntity(object(saved.entity ?? saved), workspaceId, userId));
      }

      const issueMatch = /^\/api\/issues\/([^/]+)(\/move)?$/.exec(url.pathname);
      if (issueMatch) {
        const issueId = decodeURIComponent(issueMatch[1]);
        if (method === "GET") {
          const result = await this.query("issues");
          const item = collection(result).find((candidate) => string(candidate.id) === issueId);
          return item ? jsonResponse(issueFromEntity(item, workspaceId, userId)) : errorResponse(404, "not_found", "任务不存在");
        }
        const data = await readBody(init);
        if (method === "POST" && issueMatch[2] === "/move") {
          const moved = object(await this.call("/multica/workspace/move-issue", {
            issueId,
            status: nullableString(data.status),
            assigneeType: Object.hasOwn(data, "assignee_type") ? nullableString(data.assignee_type) : undefined,
            assigneeId: Object.hasOwn(data, "assignee_id") ? nullableString(data.assignee_id) : undefined,
            parentIssueId: Object.hasOwn(data, "parent_issue_id") ? nullableString(data.parent_issue_id) : undefined,
            projectId: Object.hasOwn(data, "project_id") ? nullableString(data.project_id) : undefined,
            beforeId: nullableString(data.before_id),
            afterId: nullableString(data.after_id),
            expectedRevision: number(data.expected_revision, 0),
          }));
          return jsonResponse(issueFromEntity(object(moved.entity ?? moved), workspaceId, userId));
        }
        if (method === "PUT") {
          const existing = object((await this.query("issues")).items ? collection(await this.query("issues")).find((candidate) => string(candidate.id) === issueId) : {});
          if (!string(existing.id)) return errorResponse(404, "not_found", "任务不存在");
          const entity = { ...existing, ...data, id: issueId, workspace_id: workspaceId, updated_at: now() };
          const saved = object(await this.call("/multica/workspace/upsert", { resource: "issues", entity, expectedRevision: number(data.expected_revision, number(existing.revision, 0)) }));
          return jsonResponse(issueFromEntity(object(saved.entity ?? saved), workspaceId, userId));
        }
      }
      return errorResponse(404, "capability_unavailable", "当前 Codex Runtime 未提供该工作流能力");
    } catch (error) {
      if (error instanceof AdapterHttpError) return errorResponse(error.status, string(object(error.body).code, "runtime_error"), error.message);
      return errorResponse(503, "runtime_unavailable", error instanceof Error ? error.message : "本地工作流连接失败");
    }
  }
}

class AdapterHttpError extends Error {
  constructor(readonly status: number, message: string, readonly body: unknown) {
    super(message);
  }
}
