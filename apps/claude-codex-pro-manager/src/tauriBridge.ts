import { invoke as tauriInvoke } from "@tauri-apps/api/core";

const hasTauriInternals = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function invokeCommand<T>(command: string, args?: Record<string, unknown>) {
  if (hasTauriInternals()) return tauriInvoke<T>(command, args);
  // Preview mock (tauriPreviewMock.ts) is a separate chunk loaded only here.
  return import("./tauriPreviewMock").then(({ mockInvoke }) => mockInvoke(command, args) as Promise<T>);
}
