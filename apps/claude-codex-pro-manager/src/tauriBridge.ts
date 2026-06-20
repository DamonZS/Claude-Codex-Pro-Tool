import { invoke as tauriInvoke } from "@tauri-apps/api/core";

type Status = "ok" | "failed" | "not_checked" | string;

type CommandResult<T extends Record<string, unknown>> = T & {
  status: Status;
  message: string;
};

const hasTauriInternals = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function invokeCommand<T>(command: string, args?: Record<string, unknown>) {
  if (hasTauriInternals()) return tauriInvoke<T>(command, args);
  return mockInvoke(command, args) as Promise<T>;
}

async function mockInvoke(command: string, _args?: Record<string, unknown>) {
  if (command === "open_external_url") return ok("预览模式不打开外部链接。", {});
  return {
    status: "not_implemented",
    message: `当前是无 Tauri 预览环境，命令未执行：${command}`,
  } as CommandResult<Record<string, unknown>>;
}

function ok<T extends Record<string, unknown>>(message: string, payload: T): CommandResult<T> {
  return { status: "ok", message, ...payload };
}
