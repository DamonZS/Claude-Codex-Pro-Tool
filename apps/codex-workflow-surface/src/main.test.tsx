import { act, fireEvent, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { runtime, type WorkflowRoute } from "./main";
import { useModalStore } from "@multica/core/modals";
import { configStore } from "@multica/core/config";

const workspace = { id: "workspace-fixture", slug: "local-fixture", name: "Fixture workspace" };
const timestamp = "2026-09-18T00:00:00Z";
let container: HTMLDivElement;
let host: HTMLDivElement;
let calls: Array<{ path: string; payload: Record<string, unknown> }>;
let entities: Record<string, Record<string, unknown>[]>;
const options = (route: WorkflowRoute) => ({ route, workspaceId: workspace.id, workspaceSlug: workspace.slug });

beforeEach(() => {
  localStorage.clear();
  calls = [];
  const receipts = new Map<string, { signature: unknown; result: unknown }>();
  entities = {
    issues: [], agents: [], autopilots: [], labels: [], issue_views: [], runtimes: [],
    issue_statuses: ["backlog", "todo", "in_progress", "in_review", "done", "blocked", "cancelled"].map((category, position) => ({
      id: `status-${category}`, workspace_id: workspace.id, key: category, category,
      name: category, position, color: "#888888", revision: 1, is_system: true,
    })),
  };
  window.__CODEX_WORKFLOW_BRIDGE__ = {
    postJson: async (path, payload) => {
      calls.push({ path, payload });
      if (path === "/multica/workspace/bootstrap") return {
        status: "ok", workspace, user: { id: "user-fixture" },
        capabilities: {}, permissions: {},
      };
      if (path === "/multica/workspace/query") {
        const items = entities[String(payload.resource)] ?? [];
        return { status: "ok", items, total: items.length };
      }
      if (path === "/multica/workspace/command") {
        const receipt = receipts.get(String(payload.commandId));
        if (receipt && receipt.signature !== payload.commandSignature) return { status: "failed", code: "multica_workspace_idempotency_conflict" };
        return { status: "ok", found: !!receipt, result: receipt ? structuredClone(receipt.result) : null };
      }
      if (path === "/multica/workspace/upsert" || path === "/multica/agents/create") {
        const resource = path.endsWith("/create") ? "agents" : String(payload.resource);
        const entity = payload.entity as Record<string, unknown>;
        const previous = entities[resource]?.find((item) => item.id === entity.id);
        if (previous && payload.expectedRevision !== previous.revision) return { status: "failed", code: "multica_workspace_revision_conflict", current_revision: previous.revision };
        const saved = { ...entity, revision: Number(previous?.revision ?? 0) + 1, created_at: previous?.created_at ?? timestamp, updated_at: timestamp };
        entities[resource] = [...(entities[resource] ?? []).filter((item) => item.id !== entity.id), saved];
        const result = { status: "ok", entity: saved };
        if (typeof payload.commandId === "string") receipts.set(payload.commandId, { signature: payload.commandSignature, result: structuredClone(result) });
        return result;
      }
      if (path === "/multica/autopilots/runs") return { status: "ok", runs: [], total: 0 };
      if (path === "/multica/autopilots/cron-preview") return { status: "ok", next_runs: ["2026-09-19T09:00:00Z"] };
      return { status: "failed", code: "capability_unavailable", message: "Fixture capability unavailable" };
    },
    openThread: vi.fn().mockResolvedValue(undefined),
  };
  host = document.createElement("div");
  document.body.append(host);
  const shadow = host.attachShadow({ mode: "open" });
  container = document.createElement("div");
  shadow.append(container);
});

afterEach(async () => {
  await act(async () => runtime.unmount(container));
  host.remove();
  delete window.__CODEX_WORKFLOW_BRIDGE__;
  delete document.documentElement.dataset.theme;
  document.documentElement.classList.remove("dark");
  vi.restoreAllMocks();
});

async function mount(route: WorkflowRoute) {
  await act(async () => runtime.mount(container, options(route)));
  await waitFor(() => expect(container.querySelector(".ccp-workflow-page h1")).not.toBeNull());
  expect(container.textContent).not.toContain("Multica 页面加载失败");
}

describe("actual pinned upstream pages through the local adapter", () => {
  it("shows native child agents alongside the original My Issues page", async () => {
    entities.codex_native_agents = ["Descartes", "Tesla", "Faraday", "Mendel"].map((name) => ({
      id: `native-${name}`, parent_thread_id: "native-parent", agent_nickname: name,
      status: "running", updated_at_ms: 1789720000000, source: "codex_native", read_only: true,
    }));
    const onOpenThread = vi.fn().mockResolvedValue(true);
    await act(async () => runtime.mount(container, { ...options("my-issues"), openThread: onOpenThread }));
    const region = await within(container).findByRole("region", { name: "Codex 原生子任务" });
    for (const name of ["Descartes", "Tesla", "Faraday", "Mendel"]) {
      expect(await within(region).findByRole("button", { name })).toBeTruthy();
    }
    expect(container.querySelector("h1")?.textContent).toContain("我的任务");
    fireEvent.click(within(region).getByRole("button", { name: "Tesla" }));
    await waitFor(() => expect(onOpenThread).toHaveBeenCalledWith("native-Tesla"));
    expect(window.__CODEX_WORKFLOW_BRIDGE__!.openThread).not.toHaveBeenCalled();
  });

  it.each(["loaded", "missing", "loading"] as const)("returns a directly opened %s issue to My Issues", async (state) => {
    const id = "11111111-1111-4111-8111-111111111111";
    if (state === "loaded") entities.issues.push({ id, workspace_id: workspace.id,
      identifier: "TEST-1", title: "Return navigation fixture", status: "todo", priority: "none",
      revision: 1, created_at: timestamp, updated_at: timestamp, created_by_id: "user-fixture" });
    let release: (() => void) | undefined;
    if (state === "loading") {
      const original = window.__CODEX_WORKFLOW_BRIDGE__!.postJson;
      const pending = new Promise<void>((resolve) => { release = resolve; });
      window.__CODEX_WORKFLOW_BRIDGE__!.postJson = async (path, payload) => {
        if (path === "/multica/workspace/query" && payload.resource === "issues") await pending;
        return original(path, payload);
      };
    }
    const onNavigate = vi.fn();
    try {
      await act(async () => runtime.mount(container, { ...options("my-issues"),
        path: `/local-fixture/issues/${id}`, onNavigate }));
      const back = await within(container).findByRole("button", { name: "返回我的任务" });
      if (state === "loaded") await waitFor(() => expect(container.textContent).toContain("Return navigation fixture"));
      expect(within(container).getAllByRole("button", { name: "返回我的任务" })).toHaveLength(1);
      fireEvent.click(back);
      await waitFor(() => expect(container.querySelector("h1")?.textContent).toContain("我的任务"));
      expect(onNavigate).toHaveBeenLastCalledWith("my-issues", "/local-fixture/my-issues");
    } finally {
      await act(async () => release?.());
    }
  });

  it("initializes conversation-starter support before showing forms and clears it on unmount", async () => {
    configStore.getState().setAgentConversationStartersSupported(false);
    await mount("agents");
    expect(configStore.getState().agentConversationStartersSupported).toBe(true);
    await act(async () => runtime.unmount(container));
    expect(configStore.getState().agentConversationStartersSupported).toBe(false);
  });

  describe.each([
    { route: "agents", label: "返回智能体", title: "智能体" },
    { route: "autopilots", label: "返回自动化", title: "自动化" },
  ] as const)("direct $route detail returns", ({ route, label, title }) => {
    it.each(["loaded", "loading", "missing", "error"] as const)("returns from the %s query state", async (state) => {
      const id = `${route}-detail-fixture`;
      if (state === "loaded") entities[route].push(route === "agents"
        ? { id, name: "Direct agent fixture", runtime_id: "host-fixture", owner_id: "user-fixture", revision: 1 }
        : { id, title: "Direct automation fixture", assignee_id: "agent-fixture", revision: 1, status: "active", created_by_id: "user-fixture", created_at: timestamp, updated_at: timestamp });
      const original = window.__CODEX_WORKFLOW_BRIDGE__!.postJson;
      let release!: () => void;
      const pending = new Promise<void>((resolve) => { release = resolve; });
      let queryStarted = false;
      let returned = false;
      window.__CODEX_WORKFLOW_BRIDGE__!.postJson = async (path, payload) => {
        if (path === "/multica/workspace/query" && payload.resource === route && !returned) {
          queryStarted = true;
          if (state === "loading") await pending;
          if (state === "error") return { status: "failed", code: "bridge_timeout" };
        }
        return original(path, payload);
      };
      const onNavigate = vi.fn();
      try {
        await act(async () => runtime.mount(container, { ...options(route), path: `/local-fixture/${route}/${id}`, onNavigate }));
        await waitFor(() => expect(queryStarted).toBe(true));
        await waitFor(() => expect(within(container).queryByRole("status")).toBeNull());
        if (state === "loaded") await within(container).findByRole("heading", { level: 1, name: route === "agents" ? "Direct agent fixture" : "Direct automation fixture" });
        if (state === "missing" || state === "error") {
          await within(container).findByText(route === "autopilots" ? "未找到该自动化" : state === "missing" ? "未找到该智能体" : "无法加载该智能体", {}, { timeout: 5000 });
        }
        expect(onNavigate).not.toHaveBeenCalled();
        expect(container.textContent).not.toContain("Multica 页面加载失败");
        const back = route === "agents" && (state === "missing" || state === "error")
          ? within(container).getByRole("link", { name: "智能体" })
          : within(container).getByRole("button", { name: label });
        returned = true;
        fireEvent.click(back);
        await act(async () => release());
        await waitFor(() => expect(container.querySelector("h1")?.textContent).toBe(title));
        expect(onNavigate).toHaveBeenCalledExactlyOnceWith(route, `/local-fixture/${route}`);
      } finally {
        await act(async () => release());
      }
    });
  });

  it("returns from a forbidden direct agent detail through the upstream header link", async () => {
    const original = window.__CODEX_WORKFLOW_BRIDGE__!.postJson;
    window.__CODEX_WORKFLOW_BRIDGE__!.postJson = async (path, payload) => {
      if (path === "/multica/workspace/query" && payload.resource === "agents") return { status: "failed", code: "permission_denied" };
      return original(path, payload);
    };
    const onNavigate = vi.fn();
    await act(async () => runtime.mount(container, { ...options("agents"), path: "/local-fixture/agents/private-fixture", onNavigate }));
    await within(container).findByText("你没有访问该智能体的权限", {}, { timeout: 5000 });
    expect(onNavigate).not.toHaveBeenCalled();
    window.__CODEX_WORKFLOW_BRIDGE__!.postJson = original;
    fireEvent.click(within(container).getByRole("link", { name: "智能体" }));
    await waitFor(() => expect(container.querySelector("h1")?.textContent).toBe("智能体"));
    expect(onNavigate).toHaveBeenCalledExactlyOnceWith("agents", "/local-fixture/agents");
  });

  describe.each([
    { route: "my-issues", section: "issues", label: "返回我的任务", title: "我的任务" },
    { route: "autopilots", section: "autopilots", label: "返回自动化", title: "自动化" },
    { route: "agents", section: "agents", label: "返回智能体", title: "智能体" },
  ] as const)("direct $section detail bootstrap returns", ({ route, section, label, title }) => {
    it.each(["loading", "error"] as const)("returns during bootstrap %s and retains the collection after recovery", async (state) => {
      const original = window.__CODEX_WORKFLOW_BRIDGE__!.postJson;
      let release!: () => void;
      const pending = new Promise<void>((resolve) => { release = resolve; });
      let failed = state === "error";
      window.__CODEX_WORKFLOW_BRIDGE__!.postJson = async (path, payload) => {
        if (path === "/multica/workspace/bootstrap") {
          if (state === "loading") await pending;
          if (failed) return { status: "failed", code: "bridge_timeout" };
        }
        return original(path, payload);
      };
      const onNavigate = vi.fn();
      try {
        await act(async () => runtime.mount(container, { ...options(route), path: `/local-fixture/${section}/direct-fixture`, onNavigate }));
        const shell = await within(container).findByRole(state === "loading" ? "status" : "alert");
        if (state === "error") expect(shell.getAttribute("data-code")).toBe("bridge_timeout");
        expect(onNavigate).not.toHaveBeenCalled();
        const back = within(container).getAllByRole("button", { name: label });
        expect(back).toHaveLength(1);
        fireEvent.click(back[0]);
        expect(onNavigate).toHaveBeenCalledExactlyOnceWith(route, `/local-fixture/${route}`);
        expect(within(container).queryByRole("button", { name: label })).toBeNull();
        failed = false;
        if (state === "error") fireEvent.click(within(container).getByRole("button", { name: "重试" }));
        await act(async () => release());
        await waitFor(() => expect(container.querySelector("h1")?.textContent).toBe(title));
        expect(onNavigate).toHaveBeenCalledTimes(1);
      } finally {
        await act(async () => release());
      }
    });
  });

  it("renders degraded core bootstrap and retries a sanitized connection failure", async () => {
    const original = window.__CODEX_WORKFLOW_BRIDGE__!.postJson;
    let failed = true;
    const bootstrap = vi.fn();
    window.__CODEX_WORKFLOW_BRIDGE__!.postJson = async (path, payload) => {
      if (path === "/multica/workspace/bootstrap") {
        bootstrap();
        if (failed) return { status: "failed", code: "bridge_timeout", message: "private diagnostic" };
        return { status: "degraded", workspace, user: { id: "user-fixture", kind: "local_control_plane" }, runtime: { available: false }, collections: {} };
      }
      return original(path, payload);
    };
    await act(async () => runtime.mount(container, options("agents")));
    await waitFor(() => expect(container.querySelector('[role="alert"]')?.getAttribute("data-code")).toBe("bridge_timeout"));
    expect(container.textContent).not.toContain("private diagnostic");
    failed = false;
    await act(async () => {
      fireEvent.click(within(container).getByRole("button", { name: "重试" }));
    });
    expect(bootstrap).toHaveBeenCalledTimes(2);
    await waitFor(() => expect(container.querySelector(".ccp-workflow-page h1")).not.toBeNull());
  });

  it("mirrors host theme changes without inserting duplicate styles", async () => {
    document.documentElement.classList.add("dark");
    await mount("agents");
    expect(container.classList.contains("dark")).toBe(true);
    document.documentElement.dataset.theme = "light";
    await waitFor(() => expect(container.classList.contains("dark")).toBe(false));
    document.documentElement.dataset.theme = "dark";
    await waitFor(() => expect(container.classList.contains("dark")).toBe(true));
    await act(async () => runtime.mount(container, options("agents")));
    expect(host.shadowRoot?.querySelectorAll("style")).toHaveLength(0);
    await act(async () => runtime.unmount(container));
    expect(container.classList.contains("dark")).toBe(false);
  });

  it("preserves a real agent form across sync and creates the agent through the bridge", async () => {
    entities.runtimes = [{ id: "host-fixture", kind: "codex_page_host", status: "available", owner_id: "user-fixture", name: "Codex" }];
    await mount("agents");
    const onNavigate = vi.fn();
    await act(async () => runtime.mount(container, { ...options("agents"), onNavigate }));
    await act(async () => runtime.navigate(container, { route: "agents", path: "/local-fixture/agents/new/manual" }));
    const name = await within(container).findByPlaceholderText("例如：深度研究智能体") as HTMLInputElement;
    fireEvent.change(name, { target: { value: "Fixture agent" } });
    await act(async () => runtime.mount(container, options("agents")));
    expect(within(container).getByPlaceholderText("例如：深度研究智能体")).toBe(name);
    expect(name.value).toBe("Fixture agent");
    const create = within(container).getByRole("button", { name: "创建并打开" }) as HTMLButtonElement;
    await waitFor(() => expect(create.disabled).toBe(false));
    fireEvent.click(create);
    await waitFor(() => expect(entities.agents).toHaveLength(1));
    expect(entities.agents[0].name).toBe("Fixture agent");
    await waitFor(() => expect(onNavigate).toHaveBeenLastCalledWith("agents", `/local-fixture/agents/${entities.agents[0].id}`));
    expect(calls.some(({ path }) => path === "/multica/agents/create")).toBe(true);
  });

  it("opens the upstream issue editor and persists a submitted task", async () => {
    await mount("my-issues");
    act(() => useModalStore.getState().open("create-issue"));
    // Let upstream's 50ms dialog autofocus settle before querying the editor.
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, 100)); });
    const title = await within(container).findByRole("textbox", { name: "任务标题" });
    act(() => {
      title.textContent = "Fixture task";
      fireEvent.input(title, { inputType: "insertText", data: "Fixture task" });
    });
    await waitFor(() => expect(within(container).getByRole("button", { name: "创建任务" }).getAttribute("aria-disabled")).not.toBe("true"));
    fireEvent.click(within(container).getByRole("button", { name: "创建任务" }));
    await waitFor(() => expect(entities.issues).toHaveLength(1));
    expect(entities.issues[0].title).toBe("Fixture task");
    expect(document.body.querySelector('[role="dialog"]')).toBeNull();
  });

  it("loads an original autopilot detail and persists pause through its switch", async () => {
    entities.autopilots = [{ id: "auto-fixture", title: "Fixture automation", assignee_id: "agent-fixture", revision: 1, status: "active", created_by_id: "user-fixture", created_at: timestamp, updated_at: timestamp }];
    await mount("autopilots");
    await act(async () => runtime.navigate(container, { route: "autopilots", path: "/local-fixture/autopilots/auto-fixture" }));
    expect(await within(container).findByRole("button", { name: "返回自动化" })).toBeTruthy();
    const toggle = await within(container).findByRole("switch");
    expect(container.textContent).toContain("Fixture automation");
    fireEvent.click(toggle);
    await waitFor(() => expect(calls.some(({ path, payload }) => path === "/multica/workspace/upsert" && payload.resource === "autopilots")).toBe(true));
    expect(entities.autopilots[0].status).toBe("paused");
  });

  it("keeps a return control on the original agent detail", async () => {
    entities.agents = [{ id: "agent-fixture", name: "Fixture agent", runtime_id: "host-fixture", owner_id: "user-fixture", revision: 1 }];
    await mount("agents");
    await act(async () => runtime.navigate(container, { route: "agents", path: "/local-fixture/agents/agent-fixture" }));
    expect(await within(container).findByRole("button", { name: "返回智能体" })).toBeTruthy();
  });

  it("submits the original autopilot form with an agent and schedule", async () => {
    entities.agents = [{ id: "agent-fixture", name: "Schedule agent", runtime_id: "host-fixture", owner_id: "user-fixture", revision: 1 }];
    await mount("autopilots");
    fireEvent.click(within(container).getByRole("button", { name: "新建自动化" }));
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, 100)); });
    const title = await within(container).findByRole("textbox", { name: "自动化名称" });
    act(() => {
      title.textContent = "Fixture scheduled automation";
      fireEvent.input(title, { inputType: "insertText", data: "Fixture scheduled automation" });
    });
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, 100)); });
    const dialog = within(container).getByRole("dialog");
    fireEvent.click(within(dialog).getByText("选择智能体或小队").closest("button")!);
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, 100)); });
    const candidate = await within(container).findByRole("button", { name: "Schedule agent" });
    fireEvent.click(candidate);
    fireEvent.click(within(dialog).getByText("创建自动化", { selector: "button" }));
    await waitFor(() => expect(entities.autopilots).toHaveLength(1));
    await waitFor(() => expect(entities.autopilots[0].triggers).toEqual([expect.objectContaining({ kind: "schedule", enabled: true })]));
    expect(entities.autopilots[0]).toMatchObject({ title: "Fixture scheduled automation", assignee_id: "agent-fixture" });
    expect(calls.some(({ path }) => path === "/multica/autopilots/cron-preview")).toBe(true);
    expect(document.body.querySelector('[role="dialog"]')).toBeNull();
  });

  it.each(["my-issues", "autopilots", "agents"] as const)("mounts %s without unmanaged HTTP or sockets", async (route) => {
    const network = vi.spyOn(globalThis, "fetch").mockRejectedValue(new Error("Unexpected network"));
    const socket = vi.spyOn(globalThis, "WebSocket").mockImplementation(() => { throw new Error("Unexpected socket"); });
    await mount(route);
    expect(calls.some(({ path }) => path === "/multica/workspace/bootstrap")).toBe(true);
    expect(network).not.toHaveBeenCalled();
    expect(socket).not.toHaveBeenCalled();
    expect(container.textContent).toContain("Multica");
  });

  it("opens the full attribution dialog inside the shadow container", async () => {
    await mount("agents");
    fireEvent.click(within(container).getByRole("button", { name: "Multica LICENSE and NOTICE" }));
    await waitFor(() => expect(within(container).getByRole("dialog")).toBeTruthy());
    expect(within(container).getByRole("dialog").textContent).toContain("Apache License");
    expect(document.body.querySelector('[role="dialog"]')).toBeNull();
  });

  it("navigates the original agent creation flow without host callbacks", async () => {
    await mount("agents");
    fireEvent.click(within(container).getAllByRole("button", { name: "新建智能体" })[0]);
    await waitFor(() => expect(container.textContent).toContain("从空白"));
    expect(container.textContent).not.toContain("该页面不属于当前工作流");
  });

  it("notifies the parent on upstream back and preserves the resulting route on sync", async () => {
    await mount("agents");
    const onNavigate = vi.fn();
    await act(async () => runtime.mount(container, { ...options("agents"), onNavigate }));
    fireEvent.click(within(container).getAllByRole("button", { name: "新建智能体" })[0]);
    await waitFor(() => expect(onNavigate).toHaveBeenLastCalledWith("agents", "/local-fixture/agents/new"));
    fireEvent.click(within(container).getByRole("button", { name: "返回" }));
    await waitFor(() => expect(onNavigate).toHaveBeenLastCalledWith("agents", "/local-fixture/agents"));
    await act(async () => runtime.mount(container, { ...options("agents"), path: "/local-fixture/agents", onNavigate }));
    expect(onNavigate).toHaveBeenCalledTimes(2);
  });

  it("preserves the open dialog and DOM root on repeated mount", async () => {
    await mount("agents");
    const surface = container.firstElementChild;
    fireEvent.click(within(container).getByRole("button", { name: "Multica LICENSE and NOTICE" }));
    const dialog = await within(container).findByRole("dialog");
    const bootstrapCalls = calls.filter(({ path }) => path === "/multica/workspace/bootstrap").length;
    await act(async () => {
      runtime.mount(container, options("agents"));
      runtime.mount(container, options("agents"));
    });
    expect(container.firstElementChild).toBe(surface);
    expect(within(container).getByRole("dialog")).toBe(dialog);
    expect(calls.filter(({ path }) => path === "/multica/workspace/bootstrap")).toHaveLength(bootstrapCalls);
  });

  it("updates all three upstream routes through the same mounted root", async () => {
    await mount("agents");
    const surface = container.firstElementChild;
    await act(async () => runtime.navigate(container, { route: "autopilots" }));
    await waitFor(() => expect(container.querySelector("h1")?.textContent).toContain("自动化"));
    await act(async () => runtime.navigate(container, { route: "my-issues" }));
    await waitFor(() => expect(container.querySelector("h1")?.textContent).toContain("我的"));
    expect(container.firstElementChild).toBe(surface);
  });
});
