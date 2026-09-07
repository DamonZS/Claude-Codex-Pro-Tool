/*
 * Codex mount for the derived Multica UI. Source attribution is recorded in
 * docs/third-party/multica/SOURCE_MANIFEST.md.
 */
import { StrictMode, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { QueryProvider } from "@multica/core/provider";
import "./styles.css";

export type WorkflowRoute = "my-issues" | "autopilots" | "agents";

export interface WorkflowSurfaceMount {
  route: WorkflowRoute;
  workspaceSlug: string;
  workspaceId: string;
}

export interface WorkflowSurfaceRuntime {
  mount(container: HTMLElement, options: WorkflowSurfaceMount): void;
  unmount(container: HTMLElement): void;
}

const roots = new WeakMap<HTMLElement, Root>();

function Unavailable({ route }: { route: WorkflowRoute }) {
  return (
    <div role="alert" className="flex min-h-full items-center justify-center p-6 text-sm text-muted-foreground">
      {route === "my-issues" ? "工作区适配器尚未连接" : "该原始页面适配器尚未连接"}
    </div>
  );
}

function Surface({ options }: { options: WorkflowSurfaceMount }): ReactNode {
  // The actual upstream page is selected only after the adapter has registered
  // the API/auth/workspace runtime. This prevents an upstream component from
  // issuing unmanaged HTTP/WebSocket requests during a partial mount.
  return <Unavailable route={options.route} />;
}

const runtime: WorkflowSurfaceRuntime = {
  mount(container, options) {
    roots.get(container)?.unmount();
    const root = createRoot(container);
    roots.set(container, root);
    root.render(
      <StrictMode>
        <QueryProvider>
          <Surface options={options} />
        </QueryProvider>
      </StrictMode>,
    );
  },
  unmount(container) {
    roots.get(container)?.unmount();
    roots.delete(container);
  },
};

Object.defineProperty(window, "__CODEX_WORKFLOW_SURFACE__", {
  configurable: true,
  value: runtime,
  writable: false,
});

declare global {
  interface Window {
    __CODEX_WORKFLOW_SURFACE__?: WorkflowSurfaceRuntime;
  }
}
