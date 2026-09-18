import { act, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { runtime } from "./main";
import { useModalStore } from "@multica/core/modals";
import { useIssueDraftStore } from "@multica/core/issues/stores/draft-store";

let host: HTMLDivElement;
let container: HTMLDivElement;
afterEach(async () => {
  if (container) await act(async () => runtime.unmount(container));
  host?.remove();
  delete window.__CODEX_WORKFLOW_BRIDGE__;
});

describe("original quick-create modal with Core native capability", () => {
  it.each([true, false])("sets the actual Create button from nativeTaskHostSupported=%s", async (supported) => {
    localStorage.clear();
    useIssueDraftStore.getState().clearDraft();
    const workspace = { id: "capability-workspace", slug: "capability", name: "Capability test" };
    const collections: Record<string, Record<string, unknown>[]> = {
      agents: [{ id: "agent-1", name: "Native agent", runtime_id: "native-runtime", owner_id: "user-1", permission_mode: "private", revision: 1 }],
      runtimes: [{ id: "native-runtime", kind: "codex_page_host", provider: "codex", status: "available", native_task_host_supported: supported }],
    };
    window.__CODEX_WORKFLOW_BRIDGE__ = {
      postJson: async (path, payload) => {
        if (path === "/multica/workspace/bootstrap") return {
          status: "ok", workspace, user: { id: "user-1", kind: "local_control_plane" },
          runtime: { available: true, runtimeId: "native-runtime", nativeTaskHostSupported: supported },
        };
        if (path === "/multica/workspace/query") {
          const items = collections[String(payload.resource)] ?? [];
          return { status: "ok", items, total: items.length };
        }
        return { status: "failed", code: "capability_unavailable" };
      },
      openThread: async () => undefined,
    };
    host = document.createElement("div");
    document.body.append(host);
    container = document.createElement("div");
    host.attachShadow({ mode: "open" }).append(container);
    await act(async () => runtime.mount(container, { route: "agents", workspaceId: workspace.id, workspaceSlug: workspace.slug }));
    await waitFor(() => expect(container.querySelector(".ccp-workflow-page h1")).not.toBeNull());
    act(() => useModalStore.getState().open("quick-create-issue", { agent_id: "agent-1", prompt: "Inspect the current workspace" }));
    const dialog = await within(container).findByRole("dialog");
    await waitFor(() => expect(dialog.textContent).toContain("Native agent"));
    const create = within(dialog).getByRole("button", { name: /^创建(?:\s|$)/ }) as HTMLButtonElement;
    await waitFor(() => {
      expect(create.disabled).toBe(!supported);
      expect(create.title.length > 0).toBe(!supported);
      expect(dialog.textContent?.includes("守护进程没有报告 CLI 版本")).toBe(!supported);
    });
  });
});
