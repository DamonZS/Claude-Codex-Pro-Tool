/* CCP host for Multica 9fce92f. Attribution and modifications: UPSTREAM.md. */
import "./schema-runtime";
import { Component, useState, useSyncExternalStore, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { QueryClientProvider } from "@tanstack/react-query";
import { ApiClient, setApiInstance } from "@multica/core/api";
import { createAuthStore, registerAuthStore } from "@multica/core/auth";
import { configStore } from "@multica/core/config";
import { createChatStore, registerChatStore } from "@multica/core/chat";
import { useModalStore } from "@multica/core/modals";
import { createQueryClient } from "@multica/core/query-client";
import { setCurrentWorkspace } from "@multica/core/platform/workspace-storage";
import { defaultStorage } from "@multica/core/platform/storage";
import { I18nProvider } from "@multica/core/i18n/provider";
import type { LocaleResources } from "@multica/core/i18n/types";
import { WorkspaceSlugProvider } from "@multica/core/paths/hooks";
import { workspaceListOptions } from "@multica/core/workspace/queries";
import { WSContext } from "@multica/core/realtime/provider";
import type { WSEventType } from "@multica/core/types";
import { MyIssuesPage } from "@multica/views/my-issues/components/my-issues-page";
import { PropertiesTab } from "@multica/views/settings/components/properties-tab";
import { AutopilotsPage } from "@multica/views/autopilots/components/autopilots-page";
import { AutopilotDetailPage } from "@multica/views/autopilots/components/autopilot-detail-page";
import { AgentsPage } from "@multica/views/agents/components/agents-page";
import { AgentDetailPage } from "@multica/views/agents/components/agent-detail-page";
import { ChooseCreateMethodPage } from "@multica/views/agents/create/choose-create-method-page";
import { ManualCreateAgentPage } from "@multica/views/agents/create/manual-create-agent-page";
import { AiCreateAgentPage } from "@multica/views/agents/create/ai-create-agent-page";
import { AiBuilderSessionPage } from "@multica/views/agents/create/ai-builder-session-page";
import { IssueDetailRoute } from "@multica/views/issues/components/issue-detail-route";
import { ModalRegistry } from "@multica/views/modals/registry";
import { NavigationProvider, useNavigation, type NavigationAdapter } from "@multica/views/navigation";
import { ArrowLeft } from "lucide-react";
import { Button } from "@multica/ui/components/ui/button";
import { MulticaIcon } from "@multica/ui/components/common/multica-icon";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@multica/ui/components/ui/dialog";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@multica/ui/components/ui/tooltip";
import { Toaster } from "sonner";
import { MulticaApiAdapter } from "./multica-api-adapter";
import { requireWorkflowBridge } from "./runtime-bridge";
import { ExecutionBoardRefresh, ExecutionDetail } from "./native-subtasks";
import { isExecutionIssueId } from "./execution-issue";
import { connectWorkflowHost, disconnectWorkflowHost } from "./upstream-host";
import { createBuilderHostSync, type BuilderHostSync } from "./builder-host-sync";
import license from "../vendor/multica/LICENSE?raw";
import notice from "../vendor/multica/NOTICE?raw";
import "./styles.css";

export type WorkflowRoute = "my-issues" | "autopilots" | "agents";
export interface WorkflowSurfaceMount {
  route: WorkflowRoute;
  workspaceSlug: string;
  workspaceId: string;
  path?: string;
  openThread?: (id: string) => Promise<unknown>;
  onNavigate?: (route: WorkflowRoute, path: string) => void;
}
export interface WorkflowSurfaceRuntime {
  mount(container: HTMLElement, options: WorkflowSurfaceMount): void;
  navigate(container: HTMLElement, options: Pick<WorkflowSurfaceMount, "route" | "path">): void;
  invalidate(container: HTMLElement): void;
  event(container: HTMLElement, event: { type: WSEventType; payload: unknown; actorId?: string; actorType?: string }): void;
  unmount(container: HTMLElement): void;
  dispose(): void;
}

const resources: Record<string, LocaleResources> = {};
for (const [path, value] of Object.entries(import.meta.glob<{ default: Record<string, unknown> }>(
  "../vendor/multica/packages/views/locales/{en,zh-Hans}/*.json", { eager: true },
))) {
  const [locale, file] = path.split("/").slice(-2);
  (resources[locale] ??= {})[file.replace(/\.json$/, "")] = value.default;
}

class SurfaceBoundary extends Component<{ children: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  render() {
    return this.state.failed
      ? <div role="alert">Multica 页面加载失败。<button onClick={() => this.setState({ failed: false })}>重试</button></div>
      : this.props.children;
  }
}

type EventHandler = (payload: unknown, actorId?: string, actorType?: string) => void;
type Session = {
  root: Root;
  options: WorkflowSurfaceMount;
  client: ReturnType<typeof createQueryClient>;
  history: string[];
  listeners: Set<() => void>;
  events: Map<WSEventType, Set<EventHandler>>;
  reconnect: Set<() => void>;
  version: number;
  ready: boolean;
  managePropertyCatalog: boolean;
  error: string | null;
  disposed: boolean;
  builderSync?: BuilderHostSync;
  stopTheme(): void;
  bootstrap(): Promise<void>;
};
const sessions = new Map<HTMLElement, Session>();
// Upstream registers workspace rehydration callbacks for the store's lifetime.
let chatStore: ReturnType<typeof createChatStore> | undefined;
const routePath = (options: WorkflowSurfaceMount) => options.path ?? `/${encodeURIComponent(options.workspaceSlug)}/${options.route}`;
const notify = (session: Session) => { session.version++; session.listeners.forEach((listener) => listener()); };

function syncTheme(container: HTMLElement): () => void {
  const doc = container.ownerDocument;
  const root = container.getRootNode();
  const host = root instanceof ShadowRoot ? root.host : container.parentElement;
  const media = window.matchMedia("(prefers-color-scheme: dark)");
  const update = () => {
    const explicit = doc.documentElement.dataset.theme ?? doc.body.dataset.theme;
    const scheme = host ? getComputedStyle(host).colorScheme : "";
    const dark = explicit === "dark" || (explicit !== "light" && (
      doc.documentElement.classList.contains("dark") || doc.body.classList.contains("dark") ||
      scheme === "dark" || (scheme !== "light" && media.matches)
    ));
    container.classList.toggle("dark", dark);
  };
  const observer = new MutationObserver(update);
  for (const target of [doc.documentElement, doc.body, host]) {
    if (target) observer.observe(target, { attributes: true, attributeFilter: ["class", "data-theme", "style"] });
  }
  media.addEventListener("change", update);
  update();
  return () => { observer.disconnect(); media.removeEventListener("change", update); container.classList.remove("dark"); };
}

export function localPath(path: string, slug: string): string {
  const prefix = `/${encodeURIComponent(slug)}/`;
  if (!path.startsWith(prefix) || path.includes("\\")) throw new Error("capability_unavailable: route outside workflow workspace");
  const url = new URL(path, "https://workflow.invalid");
  if (!url.pathname.startsWith(prefix)) throw new Error("capability_unavailable: invalid workflow route");
  return url.pathname + url.search + url.hash;
}

function navigateSession(session: Session, path: string, replace = false) {
  const next = localPath(path, session.options.workspaceSlug);
  if (next === session.history.at(-1)) return;
  if (replace) session.history[session.history.length - 1] = next;
  else session.history.push(next);
  const section = next.split("/")[2]?.split(/[?#]/)[0];
  const route = section === "agents" || section === "autopilots" ? section : "my-issues";
  session.options = { ...session.options, route, path: next };
  notify(session);
  session.options.onNavigate?.(route, next);
}

function WorkflowBackButton({ slug, section, label }: { slug: string; section: "my-issues" | "autopilots" | "agents"; label: string }) {
  const navigation = useNavigation();
  return <Tooltip>
    <TooltipTrigger render={<Button variant="ghost" size="icon-sm" aria-label={label}
      onClick={() => navigation.replace(`/${encodeURIComponent(slug)}/${section}`)} />}>
      <ArrowLeft className="size-4" />
    </TooltipTrigger>
    <TooltipContent>{label}</TooltipContent>
  </Tooltip>;
}

function Page({ path, slug, workspaceId, openThread, managePropertyCatalog }: { path: string; slug: string; workspaceId: string; openThread?: (id: string) => Promise<unknown>; managePropertyCatalog: boolean }) {
  const navigation = useNavigation();
  const segments = new URL(path, "https://workflow.invalid").pathname.slice(`/${encodeURIComponent(slug)}/`.length).split("/").map(decodeURIComponent);
  const [section, id, method, sessionId] = segments;
  if (section === "my-issues" && id === "properties") return <div className="p-6">
    <div className="mb-4"><WorkflowBackButton slug={slug} section="my-issues" label="返回我的任务" /></div>
    <PropertiesTab localCanManage={managePropertyCatalog} managementHint={managePropertyCatalog ? "当前本地工作区已启用属性管理。" : "当前本地工作区未开放属性管理。"} />
  </div>;
  if (section === "my-issues" || (section === "issues" && !id)) return <>
    <div className="flex shrink-0 justify-end px-6 py-1"><Button variant="ghost" size="sm" onClick={() => navigation.push(`/${encodeURIComponent(slug)}/my-issues/properties`)}>管理属性</Button></div>
    <ExecutionBoardRefresh workspaceId={workspaceId} /><MyIssuesPage />
  </>;
  if (section === "issues" && id && isExecutionIssueId(id)) return <>
    <ExecutionBoardRefresh workspaceId={workspaceId} />
    <ExecutionDetail key={id} id={id} workspaceId={workspaceId} onOpenThread={openThread} leadingAction={<WorkflowBackButton slug={slug} section="my-issues" label="返回我的任务" />} />
  </>;
  if (section === "issues" && id) return <IssueDetailRoute routeId={id} leadingAction={<WorkflowBackButton slug={slug} section="my-issues" label="返回我的任务" />} />;
  if (section === "autopilots") return id ? <AutopilotDetailPage autopilotId={id} leadingAction={<WorkflowBackButton slug={slug} section="autopilots" label="返回自动化" />} /> : <AutopilotsPage />;
  if (section === "agents") {
    if (!id) return <AgentsPage />;
    if (id !== "new") return <AgentDetailPage agentId={id} leadingAction={<WorkflowBackButton slug={slug} section="agents" label="返回智能体" />} />;
    if (method === "manual") return <ManualCreateAgentPage />;
    if (method === "ai") return sessionId ? <AiBuilderSessionPage sessionId={sessionId} /> : <AiCreateAgentPage />;
    return <ChooseCreateMethodPage />;
  }
  return <div role="alert" data-code="capability_unavailable">该页面不属于当前工作流。</div>;
}

function Attribution() {
  const [open, setOpen] = useState(false);
  return <>
    <footer className="ccp-workflow-attribution">
      <button onClick={() => setOpen(true)} aria-label="Multica LICENSE and NOTICE">
        <MulticaIcon noSpin /> Multica · Copyright 2025-2026 Multica, Inc.
      </button>
    </footer>
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent className="ccp-workflow-license">
        <DialogHeader><DialogTitle>Multica · LICENSE / NOTICE</DialogTitle></DialogHeader>
        <pre>{notice}{"\n"}{license}</pre>
      </DialogContent>
    </Dialog>
  </>;
}

function Surface({ session }: { session: Session }) {
  useSyncExternalStore(
    (listener) => { session.listeners.add(listener); return () => { session.listeners.delete(listener); }; },
    () => session.version,
  );
  const path = session.history.at(-1)!;
  const url = new URL(path, "https://workflow.invalid");
  const [, , section, detailId] = url.pathname.split("/");
  const returnRoute = detailId ? ({
    "my-issues": { section: "my-issues", label: "返回我的任务" },
    issues: { section: "my-issues", label: "返回我的任务" },
    autopilots: { section: "autopilots", label: "返回自动化" },
    agents: { section: "agents", label: "返回智能体" },
  } as const)[section as "my-issues" | "issues" | "autopilots" | "agents"] : undefined;
  const navigation: NavigationAdapter = {
    pathname: url.pathname, searchParams: url.searchParams, hash: url.hash,
    push: (next) => navigateSession(session, next),
    replace: (next) => navigateSession(session, next, true),
    back: () => {
      if (session.history.length > 1) {
        const previous = session.history[session.history.length - 2];
        session.history.pop();
        session.history[session.history.length - 1] = "";
        navigateSession(session, previous, true);
      }
    },
    canGoBack: () => session.history.length > 1,
    getShareableUrl: (next) => `#ccp-workflow=${encodeURIComponent(localPath(next, session.options.workspaceSlug))}`,
  };
  return <div className="ccp-workflow-surface">
    <I18nProvider locale="zh-Hans" resources={resources}>
      <QueryClientProvider client={session.client}>
        <WorkspaceSlugProvider slug={session.options.workspaceSlug}>
          <NavigationProvider value={navigation}>
            <TooltipProvider>
              <WSContext.Provider value={{
                subscribe: (type, handler) => {
                  const handlers = session.events.get(type) ?? new Set<EventHandler>();
                  handlers.add(handler); session.events.set(type, handlers);
                  return () => { handlers.delete(handler); };
                },
                onReconnect: (callback) => { session.reconnect.add(callback); return () => { session.reconnect.delete(callback); }; },
              }}>
                <div className="ccp-workflow-page">
                  {(!session.ready || session.error) && returnRoute &&
                    <WorkflowBackButton slug={session.options.workspaceSlug} {...returnRoute} />}
                  {session.error ? <div role="alert" data-code={session.error}>工作流连接失败（{session.error}）。<button onClick={() => void session.bootstrap()}>重试</button></div>
                    : !session.ready ? <div role="status">正在加载…</div>
                    : <SurfaceBoundary key={url.pathname}><Page path={path} slug={session.options.workspaceSlug} workspaceId={session.options.workspaceId} openThread={session.options.openThread} managePropertyCatalog={session.managePropertyCatalog} /><ModalRegistry /></SurfaceBoundary>}
                </div>
                <Attribution />
                <Toaster />
              </WSContext.Provider>
            </TooltipProvider>
          </NavigationProvider>
        </WorkspaceSlugProvider>
      </QueryClientProvider>
    </I18nProvider>
  </div>;
}

export const runtime: WorkflowSurfaceRuntime = {
  mount(container, options) {
    const existing = sessions.get(container);
    if (existing && existing.options.workspaceId === options.workspaceId && existing.options.workspaceSlug === options.workspaceSlug) {
      const changed = existing.options.route !== options.route || (options.path !== undefined && existing.options.path !== options.path);
      existing.options = { ...existing.options, ...options };
      if (changed) navigateSession(existing, routePath(options), true);
      return;
    }
    // Upstream auth/API/workspace stores are singletons; the spec has one host.
    for (const mounted of sessions.keys()) runtime.unmount(mounted);
    const bridge = requireWorkflowBridge();
    const initialPath = localPath(routePath(options), options.workspaceSlug);
    const session: Session = {
      root: createRoot(container), options: { ...options }, client: createQueryClient(),
      history: [initialPath], listeners: new Set(), events: new Map(), reconnect: new Set(),
      version: 0, ready: false, managePropertyCatalog: false, error: null, disposed: false, bootstrap: async () => {},
      stopTheme: syncTheme(container),
    };
    sessions.set(container, session);
    session.builderSync = createBuilderHostSync(session.client, options.workspaceId);
    const adapter = new MulticaApiAdapter({
      postJson: async (path, payload) => {
        const result = await bridge.postJson(path, payload);
        if (path === "/multica/builder") session.builderSync?.observeReply(payload, result);
        return result;
      },
      openThread: async (id) => {
        const open = session.options.openThread ?? bridge.openThread;
        if (!open) throw new Error("capability_unavailable: native thread navigation");
        return open(id);
      },
    });
    connectWorkflowHost((path, init) => adapter.transport(path, init), container);
    const api = new ApiClient("");
    setApiInstance(api);
    const auth = createAuthStore({ api, storage: defaultStorage, cookieAuth: true });
    registerAuthStore(auth);
    registerChatStore(chatStore ??= createChatStore({ storage: defaultStorage }));
    setCurrentWorkspace(options.workspaceSlug, options.workspaceId);
    configStore.getState().setAgentConversationStartersSupported(false);
    session.bootstrap = async () => {
      if (session.disposed) return;
      session.error = null;
      notify(session);
      try {
        const [user, workspaces, config] = await Promise.all([api.getMe(), session.client.fetchQuery({ ...workspaceListOptions(), retry: false }), api.getConfig()]);
        if (session.disposed) return;
        if (!workspaces.some((ws) => ws.id === options.workspaceId && ws.slug === options.workspaceSlug)) throw new Error("workspace_mismatch");
        auth.getState().setUser(user);
        configStore.getState().setAgentConversationStartersSupported(config.agent_conversation_starters_supported);
        session.managePropertyCatalog = config.feature_flags?.local_property_catalog_management === true;
        session.ready = true;
      } catch (error) {
        const code = error instanceof Error ? error.message : "";
        if (!session.disposed) session.error = ["workspace_mismatch", "bridge_timeout", "request_aborted", "invalid_bridge_response", "invalid_identifier", "runtime_unavailable"].includes(code) ? code : "runtime_unavailable";
      }
      if (!session.disposed) notify(session);
    };
    session.root.render(<Surface session={session} />);
    void session.bootstrap();
  },
  navigate(container, options) {
    const session = sessions.get(container);
    if (session) navigateSession(session, routePath({ ...session.options, ...options, path: options.path }));
  },
  invalidate(container) {
    const session = sessions.get(container);
    if (session) { session.builderSync?.reconnect(); void session.client.invalidateQueries(); session.reconnect.forEach((callback) => callback()); }
  },
  event(container, event) {
    const session = sessions.get(container);
    if (session) { session.events.get(event.type)?.forEach((handler) => handler(event.payload, event.actorId, event.actorType)); void session.client.invalidateQueries(); }
  },
  unmount(container) {
    const session = sessions.get(container);
    if (!session) return;
    session.disposed = true;
    session.builderSync?.dispose();
    session.stopTheme();
    session.root.unmount();
    useModalStore.getState().close();
    session.client.clear();
    session.listeners.clear();
    session.events.clear();
    session.reconnect.clear();
    sessions.delete(container);
    setCurrentWorkspace(null, null);
    configStore.getState().setAgentConversationStartersSupported(false);
    disconnectWorkflowHost();
  },
  dispose() {
    for (const container of sessions.keys()) runtime.unmount(container);
  },
};

window.__CODEX_WORKFLOW_SURFACE__?.dispose?.();
Object.defineProperty(window, "__CODEX_WORKFLOW_SURFACE__", { configurable: true, value: runtime });

declare global {
  interface Window {
    __CODEX_WORKFLOW_SURFACE__?: WorkflowSurfaceRuntime;
    __CODEX_WORKFLOW_STYLES__?: string;
  }
}
