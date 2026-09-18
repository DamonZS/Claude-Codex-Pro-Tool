export interface WorkflowSurfaceBridge {
  postJson(path: string, payload: Record<string, unknown>): Promise<unknown>;
  openThread?(id: string): Promise<unknown>;
}

declare global {
  interface Window {
    __CODEX_WORKFLOW_BRIDGE__?: WorkflowSurfaceBridge;
  }
}

export function requireWorkflowBridge(): WorkflowSurfaceBridge {
  const bridge = window.__CODEX_WORKFLOW_BRIDGE__;
  if (!bridge || typeof bridge.postJson !== "function") {
    throw new Error("Codex 本地工作流桥接未连接");
  }
  return bridge;
}
