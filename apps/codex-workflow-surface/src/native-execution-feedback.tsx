import type { NativeExecutionCard } from "./native-execution-actions";
import { useNativeExecutionActions, useNativeExecutionActionState } from "./native-execution-actions";

export function NativeExecutionFeedback({ issues }: { issues: NativeExecutionCard[] }) {
  const actions = useNativeExecutionActionState();
  const submit = useNativeExecutionActions();
  return <>{actions.map(action => {
    const issue = issues.find(value => value.workspace_id === action.issue.workspace_id &&
      value.metadata?.ccp_thread_id === action.issue.metadata?.ccp_thread_id);
    if (!issue || action.phase === "accepted" &&
      `${issue.updated_at}:${issue.metadata?.ccp_execution_state}` !== action.fingerprint) return null;
    return <div key={action.commandId} role={action.phase === "error" ? "alert" : "status"}>
      {action.phase === "pending" ? "正在提交执行指令…" : action.phase === "accepted" ? "执行指令已受理，等待实际状态更新。" : "执行指令提交失败，已保留实际状态。"}
      {action.phase === "error" && <button type="button" onClick={() => void submit(issue, action.category)}>重试执行指令</button>}
    </div>;
  })}</>;
}
