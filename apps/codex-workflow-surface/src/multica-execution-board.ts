import { issueDto, record, text, timestamp, type JsonRecord } from "./multica-adapter-dto";

export type ExecutionBoardStatus = { stale: boolean; diagnostic: string | null; updatedAt: number | null };
let status: ExecutionBoardStatus = { stale: false, diagnostic: null, updatedAt: null };
const listeners = new Set<() => void>();
// Stable snapshots support useSyncExternalStore; data stays in the adapter instance.
export function getExecutionBoardStatus(): ExecutionBoardStatus { return status; }
export function subscribeExecutionBoardStatus(listener: () => void): () => void { listeners.add(listener); return () => { listeners.delete(listener); }; }
export function setExecutionBoardStatus(next: ExecutionBoardStatus): void {
  if (next.stale === status.stale && next.diagnostic === status.diagnostic && next.updatedAt === status.updatedAt) return;
  status = next;
  listeners.forEach((listener) => listener());
}

export interface ExecutionBoard { issues: JsonRecord[]; tasks: JsonRecord[]; agents: JsonRecord[]; squads: JsonRecord[] }
export function isVirtualIssue(id: unknown): boolean { return typeof id === "string" && /^(codex-native|ccp-execution):/.test(id); }

export function executionStateToIssueStatus(state: string): string {
  switch (state) {
    case "binding_pending": case "pending": case "queued": case "dispatched": case "waiting_local_directory": return "todo";
    case "running": case "inProgress": case "in_progress": case "cancel_pending": return "in_progress";
    case "completed": return "done";
    case "cancelled": return "cancelled";
    default: return "blocked";
  }
}

export function compareExecutionTasks(a: JsonRecord, b: JsonRecord): number {
  const created = (value: JsonRecord) => typeof value.created_at_ms === "number" ? value.created_at_ms : Date.parse(text(value.created_at)) || 0;
  return created(a) - created(b)
    || Number(a.attempt ?? a.attemptNo ?? 0) - Number(b.attempt ?? b.attemptNo ?? 0)
    || text(a.binding_id ?? a.bindingId ?? a.id).localeCompare(text(b.binding_id ?? b.bindingId ?? b.id));
}

// Select execution state before any query scope, grouping or pagination is applied.
export function projectExecutionBoard(issues: JsonRecord[], executions: JsonRecord[], native: JsonRecord[], agents: JsonRecord[], squads: JsonRecord[], workspaceId: string, userId: string): ExecutionBoard {
  const result = new Map(issues.map((issue) => [text(issue.id), issue]));
  const actors = new Map(agents.map((agent) => [text(agent.id), agent]));
  const nativeByThread = new Map(native.map((item) => [text(item.id), item]));
  const bindingIssues = new Map<string, string>(), threadIssues = new Map<string, string>();
  for (const issue of issues) {
    const metadata = record(issue.metadata ?? {});
    if (metadata.ccp_binding_id) bindingIssues.set(text(metadata.ccp_binding_id), text(issue.id));
    if (metadata.ccp_thread_id) threadIssues.set(text(metadata.ccp_thread_id), text(issue.id));
  }
  const directlyLinkedIssues = new Set(executions.filter((task) => result.has(text(task.issue_id))).map((task) => text(task.issue_id)));
  // Associations from older attempts still suppress their native duplicate cards.
  for (const task of [...executions].sort(compareExecutionTasks)) if (result.has(text(task.issue_id))) {
    bindingIssues.set(text(task.binding_id ?? task.id), text(task.issue_id));
    if (task.thread_id) threadIssues.set(text(task.thread_id), text(task.issue_id));
  }
  const selected = new Map<string, JsonRecord>();
  for (const task of executions) {
    const issueId = result.has(text(task.issue_id)) ? text(task.issue_id) : bindingIssues.get(text(task.binding_id ?? task.id)) ?? threadIssues.get(text(task.thread_id));
    if (!result.has(text(task.issue_id)) && issueId && directlyLinkedIssues.has(issueId)) continue;
    const key = issueId ?? (task.thread_id ? `thread:${task.thread_id}` : `binding:${task.binding_id ?? task.id}`);
    const previous = selected.get(key);
    if (!previous || compareExecutionTasks(task, previous) > 0) selected.set(key, { ...task, issue_id: issueId ?? "" });
  }
  const tasks: JsonRecord[] = [];
  const claimedThreads = new Set([...threadIssues.keys(), ...executions.map((task) => text(task.thread_id)).filter(Boolean)]);
  function project(task: JsonRecord, nativeRow?: JsonRecord) {
    const existing = result.get(text(task.issue_id));
    const nativeResume = task.native_resume === true;
    const id = existing ? text(existing.id) : nativeRow ? `codex-native:${nativeRow.id}` : nativeResume ? `codex-native:${task.thread_id}` : `ccp-execution:${task.binding_id ?? task.id}`;
    const threadId = text(task.thread_id), bindingId = nativeRow && !nativeResume ? "" : text(task.binding_id ?? task.id);
    const agentId = text(task.agent_id) || (nativeRow || nativeResume ? `codex-native-agent:${threadId}` : `ccp-execution-agent:${bindingId}`);
    const name = text(nativeRow?.agent_nickname).trim() || text(actors.get(agentId)?.name) || text(task.agent_name).trim() || (nativeRow ? threadId.slice(0, 8) : `Execution ${bindingId.slice(0, 8)}`);
    if (!actors.has(agentId)) actors.set(agentId, { id: agentId, name, owner_id: userId });
    const state = text(task.execution_state, text(task.status, "unknown"));
    const category = executionStateToIssueStatus(state);
    const parentThreadId = text(nativeRow?.parent_thread_id ?? task.parent_thread_id);
    const metadata: JsonRecord = { ...(existing ? record(existing.metadata ?? {}) : {}),
      ...(!existing ? { ccp_read_only: true } : {}), ccp_source: nativeRow || nativeResume ? "codex-native" : "multica-execution", ccp_execution_state: state,
      ...(threadId ? { ccp_thread_id: threadId } : {}), ...(parentThreadId ? { ccp_parent_thread_id: parentThreadId } : {}),
      ...(bindingId ? { ccp_binding_id: bindingId } : {}), ccp_agent_name: name,
    };
    const updated = timestamp(task.updated_at) ?? timestamp(task.created_at) ?? "";
    const issue = existing ? { ...existing, status: category, status_category: category, metadata, updated_at: updated || existing.updated_at }
      : issueDto({ id, revision: 1, title: name, status: category, metadata, assignee_type: "agent", assignee_id: agentId,
        creator_type: "member", creator_id: userId, created_at: task.created_at, updated_at: updated, position: -(Date.parse(updated) || 0) }, workspaceId);
    result.set(id, issue);
    tasks.push({ ...task, issue_id: id, agent_id: agentId, status: category === "in_progress" ? "running" : category === "todo" ? "queued" : state, execution_state: state });
  }
  for (const task of selected.values()) {
    const nativeRow = nativeByThread.get(text(task.thread_id));
    project({ ...task, parent_thread_id: task.parent_thread_id ?? nativeRow?.parent_thread_id, agent_name: nativeRow?.agent_nickname }, task.native_resume === true ? nativeRow : undefined);
  }
  // A native subagent can be persisted on an Issue before a Multica binding is written.
  // Its thread metadata is still an authoritative execution association.
  for (const row of nativeByThread.values()) {
    const issueId = threadIssues.get(text(row.id));
    if (!issueId || selected.has(issueId)) continue;
    const updated = timestamp(row.updated_at_ms) ?? "";
    project({ id: `codex-native:${row.id}`, issue_id: issueId, thread_id: row.id, status: row.status, created_at: updated, updated_at: updated }, row);
  }
  for (const row of nativeByThread.values()) if (!claimedThreads.has(text(row.id))) {
    const updated = timestamp(row.updated_at_ms) ?? "";
    project({ id: `codex-native:${row.id}`, thread_id: row.id, status: row.status, created_at: updated, updated_at: updated }, row);
  }
  return { issues: [...result.values()], tasks, agents: [...actors.values()], squads };
}
