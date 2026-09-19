import type { Issue } from "@multica/core/types";

export const isExecutionIssueId = (id: string) => id.startsWith("codex-native:") || id.startsWith("ccp-execution:");
export const isReadonlyExecutionIssue = (issue: Issue) => issue.metadata?.ccp_read_only === true || isExecutionIssueId(issue.id);
const executionLabels: Record<string, string> = {
  binding_pending: "等待执行", pending: "等待执行", queued: "排队中", running: "执行中",
  inProgress: "执行中", in_progress: "执行中", completed: "已完成", failed: "失败",
  interrupted: "已中断", blocked: "已阻塞", cancelled: "已取消", canceled: "已取消",
  dispatched: "已派发", waiting_local_directory: "等待本地目录", cancel_pending: "取消中",
};
export function ExecutionBadge({ issue }: { issue: Issue }) {
  const source = issue.metadata?.ccp_source;
  if (!source) return null;
  const state = String(issue.metadata?.ccp_execution_state ?? "unknown");
  const name = issue.metadata?.ccp_agent_name;
  return <span className="ccp-execution-badge" data-ccp-execution-state={state}>
    {source === "codex-native" ? "Codex 原生" : "Multica 执行"} · {executionLabels[state] ?? "状态待确认"}
    {typeof name === "string" && name && name !== issue.title ? ` · ${name}` : ""}
  </span>;
}
