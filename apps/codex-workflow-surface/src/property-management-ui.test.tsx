import { act, fireEvent, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { runtime } from "./main";
import { ApiClient } from "@multica/core/api";

const workspace = { id: "property-workspace", slug: "property-fixture", name: "Fixture" };
const options = { route: "my-issues" as const, workspaceId: workspace.id, workspaceSlug: workspace.slug };
const catalogPath = "/property-fixture/my-issues/properties";
type Row = Record<string, unknown>;
let container: HTMLDivElement;
let host: HTMLDivElement;
let entities: Record<string, Row[]>;
let permission: boolean | undefined;
let failQuery: boolean;
let failWrite: boolean;
let writes: Row[];

beforeEach(() => {
  localStorage.clear();
  permission = true;
  failQuery = failWrite = false;
  writes = [];
  entities = { properties: [], issues: [], issue_views: [], issue_view_preferences: [],
    issue_statuses: ["backlog", "todo", "in_progress", "done"].map((key, position) => ({ id: key, key, category: key, name: key, position, revision: 1 })) };
  const receipts = new Map<string, { signature: unknown; result: Row }>();
  window.__CODEX_WORKFLOW_BRIDGE__ = { postJson: async (path, payload) => {
    if (path === "/multica/workspace/bootstrap") return { status: "ok", workspace,
      user: { id: "local-user", kind: "local_control_plane" }, permissions: { managePropertyCatalog: permission } };
    if (path === "/multica/executions/list") return { status: "ok", items: [], total: 0 };
    if (path === "/multica/workspace/query") {
      if (failQuery && payload.resource === "properties") return { status: "failed", code: "bridge_timeout" };
      const all = entities[String(payload.resource)] ?? [];
      return { status: "ok", items: all.slice(Number(payload.offset), Number(payload.offset) + Number(payload.limit)), total: all.length };
    }
    if (path === "/multica/workspace/command") {
      const receipt = receipts.get(String(payload.commandId));
      if (receipt && receipt.signature !== payload.commandSignature) return { status: "failed", code: "multica_workspace_idempotency_conflict" };
      return { status: "ok", found: !!receipt, result: receipt ? structuredClone(receipt.result) : null };
    }
    if (path === "/multica/workspace/upsert") {
      if (failWrite) return { status: "failed", code: "multica_workspace_revision_conflict" };
      const resource = String(payload.resource), input = payload.entity as Row;
      const previous = entities[resource]?.find((entry) => entry.id === input.id);
      if (payload.expectedRevision !== Number(previous?.revision ?? 0)) return { status: "failed", code: "multica_workspace_revision_conflict" };
      const saved = { ...input, revision: Number(previous?.revision ?? 0) + 1, created_at_ms: 1789689600000, updated_at_ms: 1789689600000 };
      entities[resource] = [...(entities[resource] ?? []).filter((entry) => entry.id !== input.id), saved];
      writes.push(payload);
      const result = { status: "ok", entity: saved };
      receipts.set(String(payload.commandId), { signature: payload.commandSignature, result: structuredClone(result) });
      return result;
    }
    return { status: "failed", code: "capability_unavailable" };
  } };
  host = document.createElement("div");
  document.body.append(host);
  container = document.createElement("div");
  host.attachShadow({ mode: "open" }).append(container);
});

afterEach(async () => {
  await act(async () => runtime.unmount(container));
  host.remove();
  delete window.__CODEX_WORKFLOW_BRIDGE__;
  vi.restoreAllMocks();
});

async function mount(path?: string) {
  await act(async () => runtime.mount(container, { ...options, path }));
  await within(container).findByRole("button", { name: path === catalogPath ? "返回我的任务" : "管理属性" });
  await within(container).findByRole("heading", { name: path === catalogPath ? "属性" : "我的任务" });
}
async function remount(path: string) {
  await act(async () => runtime.unmount(container));
  await mount(path);
}
async function openCatalog() {
  await mount();
  fireEvent.click(within(container).getByRole("button", { name: "管理属性" }));
  await within(container).findByRole("heading", { name: "属性" });
}
async function propertyAction(name: string, action: string) {
  fireEvent.click(await within(container).findByRole("button", { name: `${name} 的操作` }));
  fireEvent.click(await within(container).findByRole("menuitem", { name: action }));
}

describe("original property catalog UI", () => {
  it("creates blank-ID select options, edits, archives, reloads and restores without changing member identity", async () => {
    await openCatalog();
    expect((await new ApiClient("").listMembers(workspace.id))[0].role).toBe("member");
    fireEvent.click(within(container).getByRole("button", { name: "新建属性" }));
    const dialog = await within(container).findByRole("dialog", { name: "新建属性" });
    fireEvent.change(within(dialog).getByLabelText("名称"), { target: { value: "CCP验收 属性" } });
    fireEvent.change(within(dialog).getByPlaceholderText("选项名称"), { target: { value: "小" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "保存属性" }));
    await waitFor(() => expect(entities.properties).toHaveLength(1));
    const first = entities.properties[0];
    const firstOption = (first.config as { options: Row[] }).options[0];
    expect(firstOption).toMatchObject({ name: "小" });
    expect(firstOption.id).toEqual(expect.stringMatching(/\S+/));
    await propertyAction("CCP验收 属性", "编辑");
    const edit = await within(container).findByRole("dialog", { name: "编辑属性" });
    fireEvent.change(within(edit).getByLabelText("名称"), { target: { value: "CCP验收 属性已编辑" } });
    fireEvent.click(within(edit).getByRole("button", { name: "添加选项" }));
    fireEvent.change(within(edit).getAllByPlaceholderText("选项名称")[1], { target: { value: "大" } });
    fireEvent.click(within(edit).getByRole("button", { name: "保存属性" }));
    await waitFor(() => expect(entities.properties[0].revision).toBe(2));
    expect((entities.properties[0].config as { options: Row[] }).options).toEqual([firstOption, expect.objectContaining({ name: "大", id: expect.any(String) })]);
    await propertyAction("CCP验收 属性已编辑", "归档");
    fireEvent.click(await within(container).findByRole("button", { name: "归档属性" }));
    await waitFor(() => expect(entities.properties[0].archived).toBe(true));
    await remount(catalogPath);
    expect(within(container).queryByText("CCP验收 属性已编辑")).toBeNull();
    fireEvent.click(within(container).getByRole("switch", { name: "显示已归档" }));
    await propertyAction("CCP验收 属性已编辑", "恢复");
    await waitFor(() => expect(entities.properties[0].archived).toBe(false));
    expect(entities.properties[0].id).toBe(first.id);
    expect(writes.map((write) => write.expectedRevision)).toEqual([0, 1, 2, 3]);
    fireEvent.click(within(container).getByRole("button", { name: "返回我的任务" }));
    await within(container).findByRole("heading", { name: "我的任务", level: 1 });
  });

  it.each([false, undefined])("keeps the catalog read-only for capability %s", async (enabled) => {
    permission = enabled;
    await mount(catalogPath);
    await within(container).findByRole("heading", { name: "属性" });
    expect(within(container).queryByRole("button", { name: "新建属性" })).toBeNull();
    expect(container.textContent).toContain("当前本地工作区未开放属性管理");
    expect(writes).toEqual([]);
  });

  it.each(["loading", "error"])("returns from a direct property deep link during bootstrap %s", async (state) => {
    const original = window.__CODEX_WORKFLOW_BRIDGE__!.postJson;
    let release!: () => void;
    const pending = new Promise<void>((resolve) => { release = resolve; });
    window.__CODEX_WORKFLOW_BRIDGE__!.postJson = async (path, payload) => {
      if (path.endsWith("/bootstrap")) {
        if (state === "loading") await pending;
        else return { status: "failed", code: "bridge_timeout" };
      }
      return original(path, payload);
    };
    const onNavigate = vi.fn();
    try {
      await act(async () => runtime.mount(container, { ...options, path: catalogPath, onNavigate }));
      await within(container).findByRole(state === "loading" ? "status" : "alert");
      fireEvent.click(within(container).getByRole("button", { name: "返回我的任务" }));
      expect(onNavigate).toHaveBeenCalledExactlyOnceWith("my-issues", "/property-fixture/my-issues");
    } finally {
      await act(async () => release());
    }
  });

  it("shows catalog read failure and retries instead of reporting an empty catalog", async () => {
    failQuery = true;
    await mount(catalogPath);
    await within(container).findByText("属性加载失败", {}, { timeout: 8000 });
    expect(within(container).queryByText("还没有属性")).toBeNull();
    failQuery = false;
    fireEvent.click(within(container).getByRole("button", { name: "重试" }));
    await within(container).findByText("还没有属性");
  });

  it("keeps the editor open and leaves storage unchanged when a real mutation fails", async () => {
    await mount(catalogPath);
    fireEvent.click(await within(container).findByRole("button", { name: "新建属性" }));
    const dialog = await within(container).findByRole("dialog", { name: "新建属性" });
    fireEvent.change(within(dialog).getByLabelText("名称"), { target: { value: "CCP验收 失败" } });
    fireEvent.change(within(dialog).getByPlaceholderText("选项名称"), { target: { value: "选项" } });
    failWrite = true;
    fireEvent.click(within(dialog).getByRole("button", { name: "保存属性" }));
    await within(container).findByText(/multica_workspace_revision_conflict/);
    expect(within(container).getByRole("dialog", { name: "新建属性" })).toBe(dialog);
    expect(entities.properties).toEqual([]);
  });

  it("sets, reloads, edits and clears a value through original Issue controls without losing other values", async () => {
    entities.properties = [{ id: "estimate", name: "CCP验收 数字", type: "number", revision: 1 }, { id: "other", name: "Keep", type: "text", revision: 1 }];
    entities.issues = [{ id: "issue-fixture", title: "CCP验收 任务", identifier: "CCP-1", status: "backlog", priority: "none", creator_type: "member", creator_id: "local-user", assignee_type: null, assignee_id: null, properties: { other: "preserved" }, revision: 1 }];
    const detail = async () => {
      await act(async () => runtime.mount(container, { ...options, path: "/property-fixture/issues/issue-fixture" }));
      await within(container).findByText("CCP验收 任务");
    };
    const submit = async (value: string) => {
      const input = await within(container).findByPlaceholderText("0");
      fireEvent.change(input, { target: { value } });
      fireEvent.submit(input.closest("form")!);
    };
    await detail();
    fireEvent.click(await within(container).findByText("添加字段"));
    fireEvent.click(await within(container).findByRole("button", { name: "CCP验收 数字" }));
    await submit("13");
    await waitFor(() => expect(entities.issues[0].properties).toEqual({ other: "preserved", estimate: 13 }));
    await act(async () => runtime.unmount(container));
    await detail();
    fireEvent.click(await within(container).findByRole("button", { name: "13" }));
    await submit("21");
    await waitFor(() => expect(entities.issues[0].properties).toEqual({ other: "preserved", estimate: 21 }));
    fireEvent.click(await within(container).findByRole("button", { name: "21" }));
    await submit("");
    await waitFor(() => expect(entities.issues[0].properties).toEqual({ other: "preserved" }));
  });
});

describe("original view preference controls", () => {
  it("persists builtin and saved-view IDs from real visibility controls across remount", async () => {
    entities.issue_views = [{ id: "saved-view", workspace_id: workspace.id, owner_id: "local-user", name: "CCP验收 视图", scope_type: "my", visibility: "private", scope_variant: "assigned", query: {}, display: {}, revision: 1 }];
    await mount();
    const manage = async () => {
      fireEvent.click(within(container).getAllByRole("button", { name: "视图" })[0]);
      fireEvent.click(await within(container).findByRole("menuitem", { name: "管理视图" }));
      return within(await within(container).findByRole("dialog", { name: "管理视图" }));
    };
    let dialog = await manage();
    const toggle = (label: string) => within(dialog.getByText(label).parentElement!).getByRole("switch");
    expect(toggle("全部").getAttribute("aria-disabled")).toBe("true");
    fireEvent.click(toggle("已分配"));
    await waitFor(() => expect(entities.issue_view_preferences[0]?.prefs).toMatchObject({ hidden: ["builtin:assigned"] }));
    fireEvent.click(toggle("CCP验收 视图"));
    await waitFor(() => expect(entities.issue_view_preferences[0]?.prefs).toMatchObject({ hidden: ["builtin:assigned", "view:saved-view"] }));
    expect((entities.issue_view_preferences[0].prefs as { order: string[] }).order).toContain("builtin:all");
    expect((entities.issue_view_preferences[0].prefs as { order: string[] }).order).toContain("view:saved-view");
    expect(entities.issue_view_preferences[0].user_id).toBe("local-user");
    const handles = dialog.getAllByRole("button", { name: "拖拽排序" });
    handles.forEach((handle, index) => vi.spyOn(handle.parentElement!, "getBoundingClientRect").mockReturnValue({ x: 0, y: index * 40, top: index * 40, left: 0, bottom: index * 40 + 40, right: 400, width: 400, height: 40, toJSON: () => ({}) }));
    const startY = (handles.length - 1) * 40 + 20;
    fireEvent.pointerDown(handles.at(-1)!, { clientX: 10, clientY: startY, button: 0, isPrimary: true });
    await act(async () => { fireEvent.pointerMove(handles[1], { clientX: 10, clientY: startY - 10 }); });
    await act(async () => { fireEvent.pointerMove(handles[1], { clientX: 10, clientY: 60 }); });
    await act(async () => { fireEvent.pointerUp(handles[1]); });
    await waitFor(() => expect((entities.issue_view_preferences[0].prefs as { order: string[] }).order[1]).toBe("view:saved-view"));
    await remount("/property-fixture/my-issues");
    dialog = await manage();
    expect(toggle("已分配").getAttribute("aria-checked")).toBe("false");
    expect(toggle("CCP验收 视图").getAttribute("aria-checked")).toBe("false");
    expect(dialog.getAllByRole("button", { name: "拖拽排序" })[1].parentElement?.textContent).toContain("CCP验收 视图");
  });
});
