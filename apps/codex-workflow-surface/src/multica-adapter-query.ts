import { AdapterError, integer, keys, nullable, record, text, type JsonRecord } from "./multica-adapter-dto";

const filterKeys = ["limit", "offset", "workspace_id", "q", "status", "statuses", "status_category", "status_categories", "priority", "priorities", "assignee_id", "assignee_ids", "assignee_types", "creator_id", "project_id", "assignee_filters", "include_no_assignee", "creator_filters", "project_ids", "include_no_project", "label_ids", "top_level_only", "ids", "involves_user_id", "metadata", "properties", "open_only", "scheduled", "date_field", "date_start", "date_end", "sort", "direction", "sort_by", "sort_direction", "parent_ids", "include_closed", "group_by", "group_assignee_type", "group_assignee_id"];
const priorities = ["urgent", "high", "medium", "low", "none"];
const statuses = ["backlog", "todo", "in_progress", "in_review", "done", "blocked", "cancelled"];
export interface QueryContext { userId: string; agents: JsonRecord[]; squads: JsonRecord[]; tasks: JsonRecord[]; properties?: JsonRecord[] }

export function list(value: unknown): string[] {
  if (value === undefined || value === null || value === "") return [];
  if (typeof value === "string") return value.split(",");
  if (Array.isArray(value) && value.every((item) => typeof item === "string")) return value;
  throw new AdapterError(400, "invalid_filter");
}

function flag(value: unknown): boolean { return value === true || value === "true"; }
function bag(value: unknown): JsonRecord {
  if (value === undefined) return {};
  try { return record(typeof value === "string" ? JSON.parse(value) : value); }
  catch { throw new AdapterError(400, "invalid_filter"); }
}
function actor(value: JsonRecord, prefix: string): string {
  return value[`${prefix}_id`] ? `${value[`${prefix}_type`]}:${value[`${prefix}_id`]}` : "none";
}
function actorList(value: unknown): string[] {
  if (Array.isArray(value) && value.every((item) => item && typeof item === "object")) {
    return value.map((item) => `${record(item).type}:${record(item).id}`);
  }
  return list(value);
}
export function involved(issue: JsonRecord, userId: string, context: QueryContext): boolean {
  const owned = new Set(context.agents.filter((a) => a.owner_id === userId).map((a) => a.id));
  if (issue.assignee_type === "agent") return owned.has(issue.assignee_id);
  if (issue.assignee_type !== "squad") return false;
  const squad = context.squads.find((s) => s.id === issue.assignee_id);
  if (!squad) return false;
  if (owned.has(squad.leader_id)) return true;
  return Array.isArray(squad.members) && squad.members.some((m) => {
    const member = record(m);
    return member.member_type === "member" && member.member_id === userId || member.member_type === "agent" && owned.has(member.member_id);
  });
}

function propertyMatch(actual: unknown, wanted: unknown): boolean {
  if (typeof wanted === "string") {
    if (wanted === "__none__") return actual === null || actual === undefined || actual === "" || Array.isArray(actual) && !actual.length;
    return Array.isArray(actual) ? actual.includes(wanted) : String(actual) === wanted;
  }
  const op = record(wanted);
  keys(op, ["op", "value", "min", "max"]);
  switch (op.op) {
    case "eq": return actual === op.value;
    case "neq": return actual !== op.value;
    case "contains": return text(actual).toLowerCase().includes(text(op.value).toLowerCase());
    case "gt": return typeof actual === "number" && Number.isFinite(Number(op.value)) && actual > Number(op.value);
    case "gte": return typeof actual === "number" && Number.isFinite(Number(op.value)) && actual >= Number(op.value);
    case "lt": return typeof actual === "number" && Number.isFinite(Number(op.value)) && actual < Number(op.value);
    case "lte": return typeof actual === "number" && Number.isFinite(Number(op.value)) && actual <= Number(op.value);
    case "before": return typeof actual === "string" && actual < text(op.value);
    case "after": return typeof actual === "string" && actual > text(op.value);
    default: throw new AdapterError(503, "capability_unavailable");
  }
}

export function filterIssues(issues: JsonRecord[], filters: JsonRecord, context: QueryContext): JsonRecord[] {
  keys(filters, filterKeys);
  const metadata = bag(filters.metadata);
  const properties = bag(filters.properties);
  const assignees = actorList(filters.assignee_filters);
  const creators = actorList(filters.creator_filters);
  const projectIds = list(filters.project_ids);
  const wantedLabels = list(filters.label_ids);
  const result = issues.filter((issue) => {
    for (const field of ["status", "priority", "assignee_id", "creator_id", "project_id", "workspace_id", "status_category"]) {
      if (filters[field] !== undefined && issue[field] !== filters[field]) return false;
    }
    for (const [field, plural] of [["id", "ids"], ["status", "statuses"], ["status_category", "status_categories"], ["priority", "priorities"], ["assignee_id", "assignee_ids"], ["assignee_type", "assignee_types"], ["parent_issue_id", "parent_ids"]]) {
      const selected = list(filters[plural]);
      if (filters[plural] !== undefined && (plural === "ids" || selected.length > 0) && !selected.includes(text(issue[field]))) return false;
    }
    if ((assignees.length || flag(filters.include_no_assignee)) && !(assignees.includes(actor(issue, "assignee")) || flag(filters.include_no_assignee) && !issue.assignee_id)) return false;
    if (creators.length && !creators.includes(actor(issue, "creator"))) return false;
    if ((projectIds.length || flag(filters.include_no_project)) && !(projectIds.includes(text(issue.project_id)) || flag(filters.include_no_project) && !issue.project_id)) return false;
    const labels = new Set([...list(issue.label_ids), ...(Array.isArray(issue.labels) ? issue.labels.map((label) => text(record(label).id)) : [])]);
    if (wantedLabels.length && !wantedLabels.some((id) => labels.has(id))) return false;
    if (flag(filters.top_level_only) && issue.parent_issue_id) return false;
    if (flag(filters.open_only) && ["done", "cancelled"].includes(text(issue.status_category, text(issue.status)))) return false;
    if (flag(filters.scheduled) && !issue.start_date && !issue.due_date) return false;
    if (filters.involves_user_id && !involved(issue, text(filters.involves_user_id), context)) return false;
    if (filters.group_assignee_type && (filters.group_assignee_type === "none" ? !!issue.assignee_id : issue.assignee_type !== filters.group_assignee_type)) return false;
    if (filters.group_assignee_id && issue.assignee_id !== filters.group_assignee_id) return false;
    if (!Object.entries(metadata).every(([key, value]) => bag(issue.metadata)[key] === value)) return false;
    if (!Object.entries(properties).every(([key, values]) => Array.isArray(values) && (!values.length || values.some((value) => propertyMatch(bag(issue.properties)[key], value))))) return false;
    const dateField = text(filters.date_field, "created_at");
    if (!["created_at", "updated_at"].includes(dateField)) throw new AdapterError(400, "invalid_filter");
    const date = text(issue[dateField]);
    if (filters.date_start && date < text(filters.date_start) || filters.date_end && date >= text(filters.date_end)) return false;
    const query = text(filters.q).trim().toLowerCase();
    if (query && !(query.split(/\s+/).every((term) => text(issue.title).toLowerCase().includes(term)) || text(issue.identifier).toLowerCase() === query || String(issue.number) === query.replace(/^#/, ""))) return false;
    return true;
  });
  const sort = text(filters.sort ?? filters.sort_by, "position");
  const direction = text(filters.direction ?? filters.sort_direction, "asc");
  if (!["position", "status", "priority", "title", "created_at", "updated_at", "last_activity", "start_date", "due_date"].includes(sort) && !/^property:[\w-]+$/.test(sort) || !["asc", "desc"].includes(direction)) throw new AdapterError(400, "invalid_sort");
  const value = (issue: JsonRecord): unknown => sort === "priority" ? priorities.indexOf(text(issue.priority)) : sort === "status" ? statuses.indexOf(text(issue.status_category, text(issue.status))) : sort === "last_activity" ? issue.last_activity_at ?? issue.updated_at : sort.startsWith("property:") ? bag(issue.properties)[sort.slice(9)] : issue[sort];
  return result.sort((a, b) => {
    const av = value(a), bv = value(b);
    if (av == null || bv == null) return av == null && bv == null ? text(a.id).localeCompare(text(b.id)) : av == null ? 1 : -1;
    const order = typeof av === "number" && typeof bv === "number" ? av - bv : String(av).localeCompare(String(bv));
    return (direction === "desc" ? -order : order) || text(a.id).localeCompare(text(b.id));
  });
}

export function page<T>(items: T[], params: JsonRecord): { items: T[]; total: number } {
  const limit = integer(params.limit, 100, 5000);
  if (!limit) throw new AdapterError(400, "invalid_limit");
  const offset = integer(params.offset, 0);
  return { items: items.slice(offset, offset + limit), total: items.length };
}

function tableFilters(query: JsonRecord): JsonRecord {
  keys(query, ["scope", "filters", "search", "sort"]);
  const filters = record(query.filters);
  keys(filters, ["statuses", "priorities", "assignees", "include_no_assignee", "creators", "project_ids", "include_no_project", "label_ids", "properties", "date", "working_only", "working_issue_ids", "include_sub_issues"]);
  const sort = record(query.sort);
  const date = filters.date ? record(filters.date) : {};
  return {
    ...Object.fromEntries(["statuses", "priorities", "include_no_assignee", "project_ids", "include_no_project", "label_ids", "properties"].filter((key) => filters[key] !== undefined).map((key) => [key, filters[key]])),
    assignee_filters: filters.assignees, creator_filters: filters.creators,
    date_field: date.field, date_start: date.start, date_end: date.end,
    q: query.search, sort: sort.field, direction: sort.direction,
    ...(filters.include_sub_issues === false ? { top_level_only: true } : {}),
    ...(filters.working_issue_ids !== undefined ? { ids: filters.working_issue_ids } : {}),
  };
}

function scoped(issues: JsonRecord[], scope: JsonRecord, context: QueryContext): JsonRecord[] {
  keys(scope, ["kind", "relation", "actor", "project_id", "assignee_types"]);
  return issues.filter((issue) => {
    if (list(scope.assignee_types).length && !list(scope.assignee_types).includes(text(issue.assignee_type))) return false;
    switch (scope.kind) {
      case "workspace": return true;
      case "project": return issue.project_id === scope.project_id;
      case "assignee": case "creator": return actor(issue, scope.kind) === `${record(scope.actor).type}:${record(scope.actor).id}`;
      case "my": {
        const assigned = issue.assignee_type === "member" && issue.assignee_id === context.userId;
        const created = issue.creator_type === "member" && issue.creator_id === context.userId;
        const indirect = involved(issue, context.userId, context);
        switch (scope.relation) {
          case "assigned": return assigned;
          case "created": return created;
          case "involved": return indirect;
          case "any": return assigned || created || indirect;
          default: throw new AdapterError(400, "invalid_scope");
        }
      }
      default: throw new AdapterError(400, "invalid_scope");
    }
  });
}

interface Group { key: string; value: JsonRecord; count: number; secondary_groups?: Group[] }
function groupOf(issue: JsonRecord, group: JsonRecord, all: JsonRecord[], property?: JsonRecord): Group {
  const kind = text(group.kind);
  switch (kind) {
    case "status": case "status_category": {
      const status = text(kind === "status" ? issue.status : issue.status_category, text(issue.status));
      return { key: `${kind}:${status}`, value: { kind: "status", status }, count: 0 };
    }
    case "assignee": return { key: `assignee:${issue.assignee_id ? actor(issue, "assignee") : "unassigned"}`, value: { kind, actor: issue.assignee_id ? { type: issue.assignee_type, id: issue.assignee_id } : null }, count: 0 };
    case "project": return { key: `project:${issue.project_id ?? "none"}`, value: { kind, project_id: issue.project_id ?? null }, count: 0 };
    case "parent": {
      const parent = all.find((i) => i.id === issue.parent_issue_id);
      return { key: `parent:${issue.parent_issue_id ?? "none"}`, value: { kind, parent_id: issue.parent_issue_id ?? null, parent: parent ? { id: parent.id, number: parent.number, identifier: parent.identifier, title: parent.title, status: parent.status } : null, value_state: parent ? "value" : issue.parent_issue_id ? "unavailable" : "unset" }, count: 0 };
    }
    case "property": {
      const values = bag(issue.properties), raw = values[text(group.property_id)];
      const options = property?.type === "select" ? (record(property.config ?? {}).options ?? []) as JsonRecord[] : [];
      const present = Object.hasOwn(values, text(group.property_id));
      const validType = property?.type === "select" ? typeof raw === "string" : typeof raw === "boolean";
      const state = !present ? "unset" : validType && (property?.type !== "select" || options.some((option) => option.id === raw)) ? "value" : "unavailable";
      const value = validType ? raw : null;
      const encoded = btoa(String.fromCharCode(...new TextEncoder().encode(value === null ? "" : String(value)))).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
      return { key: `property:${group.property_id}:${state}:${encoded}`, value: { kind, property_id: group.property_id, value, value_state: state }, count: 0 };
    }
    default: throw new AdapterError(400, "invalid_group");
  }
}

export async function fingerprint(value: unknown): Promise<string> {
  function stable(input: unknown): unknown {
    if (Array.isArray(input)) return input.map(stable);
    if (input && typeof input === "object") return Object.fromEntries(Object.entries(input).sort(([a], [b]) => a.localeCompare(b)).map(([k, v]) => [k, stable(v)]));
    return input;
  }
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(JSON.stringify(stable(value))));
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

export async function tableQuery(kind: "groups" | "rows" | "facets", body: JsonRecord, all: JsonRecord[], context: QueryContext): Promise<JsonRecord> {
  keys(body, ["query", "group", "page", "group_key", "hierarchy", "parent_id", "facets", "include_total"]);
  const query = record(body.query);
  const filters = tableFilters(query);
  const queryFingerprint = await fingerprint(query);
  const scopedIssues = scoped(all, record(query.scope), context);
  let matching = filterIssues(scopedIssues, filters, context);
  const queryFilters = record(query.filters);
  if (flag(queryFilters.working_only)) {
    const runningIds = new Set(context.tasks.filter((t) => t.status === "running").map((t) => t.issue_id));
    matching = matching.filter((issue) => runningIds.has(issue.id));
  }
  if (kind === "facets") {
    if (!Array.isArray(body.facets) || body.facets.length > 64) throw new AdapterError(400, "invalid_facets");
    const facets = body.facets.map((input) => {
      const spec = record(input);
      keys(spec, ["kind", "property_id"]);
      const facetFilters = { ...filters };
      const ownFields: Record<string, string[]> = { status: ["statuses"], priority: ["priorities"], assignee: ["assignee_filters", "include_no_assignee"], creator: ["creator_filters"], project: ["project_ids", "include_no_project"], label: ["label_ids"], working_agents: ["ids"] };
      for (const field of ownFields[text(spec.kind)] ?? []) delete facetFilters[field];
      if (spec.kind === "property") {
        facetFilters.properties = { ...bag(facetFilters.properties) };
        delete record(facetFilters.properties)[text(spec.property_id)];
      }
      let facetIssues = filterIssues(scopedIssues, facetFilters, context);
      if (flag(queryFilters.working_only) && spec.kind !== "working_agents") {
        const running = new Set(context.tasks.filter((t) => t.status === "running").map((t) => t.issue_id));
        facetIssues = facetIssues.filter((i) => running.has(i.id));
      }
      const counts = new Map<string, number>();
      const add = (key: string) => counts.set(key, (counts.get(key) ?? 0) + 1);
      if (spec.kind === "working_agents") {
        const ids = new Set(facetIssues.map((i) => i.id));
        context.tasks.filter((t) => t.status === "running" && ids.has(t.issue_id) && t.agent_id).forEach((t) => add(text(t.agent_id)));
      } else for (const issue of facetIssues) {
        switch (spec.kind) {
          case "status": case "priority": add(text(issue[spec.kind])); break;
          case "assignee": case "creator": add(issue[`${spec.kind}_id`] ? actor(issue, spec.kind) : "__none__"); break;
          case "project": add(text(issue.project_id, "__none__")); break;
          case "label": for (const label of Array.isArray(issue.labels) ? issue.labels : []) add(text(record(label).id)); break;
          case "property": {
            const value = bag(issue.properties)[text(spec.property_id)];
            for (const item of Array.isArray(value) && value.length ? value : [value == null || Array.isArray(value) ? "__none__" : value]) add(String(item));
            break;
          }
          default: throw new AdapterError(400, "invalid_facet");
        }
      }
      return { ...spec, values: [...counts].map(([key, count]) => ({ key, count })) };
    });
    return { query_fingerprint: queryFingerprint, total: matching.length, facets };
  }
  const group = record(body.group);
  keys(group, ["kind", "primary", "secondary", "secondary_values", "property_id", "include_empty"]);
  const property = group.kind === "property" ? context.properties?.find((item) => item.id === group.property_id) : undefined;
  if (group.kind === "property") {
    if (!property) throw new AdapterError(400, "property_not_found");
    if (property.archived) throw new AdapterError(400, "property_archived");
    if (!["select", "checkbox"].includes(text(property.type))) throw new AdapterError(400, "property_type_unsupported");
  }
  if (group.kind === "compound" && group.secondary_values !== undefined) matching = matching.filter((i) => list(group.secondary_values).includes(text(group.secondary === "status_category" ? i.status_category : i.status, text(i.status))));
  const total = matching.length;
  const descriptor = (issue: JsonRecord) => groupOf(issue, group.kind === "compound" ? { kind: group.primary } : group, all, property);
  const secondary = (issue: JsonRecord) => {
    const primary = descriptor(issue);
    const child = groupOf(issue, { kind: group.secondary }, all);
    return { ...child, key: `compound:${primary.key}:${child.key}` };
  };
  const pageInput = body.page ? record(body.page) : {};
  keys(pageInput, ["limit", "cursor"]);
  const limit = integer(pageInput.limit, 50, 500);
  if (!limit) throw new AdapterError(400, "invalid_limit");
  const cursorFingerprint = await fingerprint({ query, group, group_key: body.group_key, parent_id: body.parent_id, hierarchy: body.hierarchy, revisions: all.map((i) => [i.id, i.revision]), ...(property ? { property } : {}) });
  const cursor = text(pageInput.cursor);
  let offset = 0;
  if (cursor) {
    const [hash, position, extra] = cursor.split(":");
    if (extra || hash !== cursorFingerprint) throw new AdapterError(409, "query_cursor_conflict");
    offset = integer(position, 0);
  }
  const next = (length: number) => offset + limit < length ? `${cursorFingerprint}:${offset + limit}` : null;
  if (kind === "groups") {
    const groups = new Map<string, Group>();
    for (const issue of matching) {
      const item = descriptor(issue);
      const current = groups.get(item.key) ?? item;
      current.count++;
      if (group.kind === "compound") {
        current.secondary_groups ??= [];
        const child = secondary(issue);
        const existing = current.secondary_groups.find((g) => g.key === child.key);
        if (existing) existing.count++; else current.secondary_groups.push({ ...child, count: 1 });
      }
      groups.set(item.key, current);
    }
    let ordered = [...groups.values()];
    if (property) {
      const values: unknown[] = property.type === "checkbox" ? [false, true] : ((record(property.config ?? {}).options ?? []) as JsonRecord[]).map((option) => option.id);
      const expected = [...values.map((value) => descriptor({ properties: { [text(group.property_id)]: value } })), descriptor({ properties: {} })];
      if (group.include_empty === true) for (const item of expected) if (!groups.has(item.key)) groups.set(item.key, item);
      const order = (item: Group) => item.value.value_state === "unset" ? values.length + 1 : item.value.value_state === "unavailable" ? values.length : values.indexOf(item.value.value);
      ordered = [...groups.values()].sort((a, b) => order(a) - order(b) || a.key.localeCompare(b.key));
    }
    return { query_fingerprint: queryFingerprint, total, groups: ordered.slice(offset, offset + limit), next_cursor: next(groups.size) };
  }
  if (group.kind !== "none" && body.group_key) matching = matching.filter((i) => descriptor(i).key === body.group_key || group.kind === "compound" && secondary(i).key === body.group_key);
  const matchingIds = new Set(matching.map((i) => i.id));
  if (body.hierarchy && record(body.hierarchy).enabled) matching = matching.filter((i) => body.parent_id ? i.parent_issue_id === body.parent_id : !i.parent_issue_id || !matchingIds.has(i.parent_issue_id));
  else if (body.parent_id) matching = matching.filter((i) => i.parent_issue_id === body.parent_id);
  const rows = matching.slice(offset, offset + limit).map((issue) => ({ issue, direct_child_count: all.filter((i) => i.parent_issue_id === issue.id && matchingIds.has(i.id)).length }));
  return { query_fingerprint: queryFingerprint, group_key: nullable(body.group_key), parent_id: nullable(body.parent_id), total, branch_total: matching.length, rows, next_cursor: next(matching.length) };
}
