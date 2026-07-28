import {
  ArrowDownToLine,
  Check,
  ChevronDown,
  ChevronRight,
  CircleCheck,
  Command,
  Laptop,
  Languages,
  LoaderCircle,
  MessageCircle,
  Moon,
  PanelLeftClose,
  PanelLeftOpen,
  Rocket,
  Search,
  Sun,
  TriangleAlert,
  X,
} from "lucide-react";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type ReactNode,
} from "react";

import {
  primaryRoute,
  routeBreadcrumb,
  routeCatalog,
  routeDomainTabs,
  routeLabel,
  routes,
  routeSubtitle,
} from "@/lib/routes";
import { formatDownloadBytes, updateProgressLabel } from "@/lib/update";
import type { Route, SupplierTargetApp, UpdateResult } from "@/types";

export type AgentScope = "codex" | "claude";
export type ThemePreference = "system" | "light" | "dark";
export type ProxyHealth = "healthy" | "attention" | "offline" | "unknown";
export type ShellSupplierOption = {
  id: string;
  name: string;
  targetApp: SupplierTargetApp;
};

type AppShellProps = {
  activeSupplierId: string | null;
  activeSupplierName: string;
  agentScope: AgentScope;
  busy: boolean;
  children: ReactNode;
  codexThemeBackground: string | null;
  onAgentScopeChange: (scope: AgentScope) => void;
  onInstallClaudeZhPatch: () => void;
  onInstallUpdate: () => void;
  onLaunchClaude: () => void;
  onNavigate: (route: Route) => void;
  onRestartCodex: () => void;
  onSelectSupplier: (profileId: string) => void;
  proxyHealth: ProxyHealth;
  route: Route;
  supplierOptions: ShellSupplierOption[];
  updateInfo: UpdateResult | null;
};

const THEME_STORAGE_KEY = "ccp-manager-theme";
const SIDEBAR_STORAGE_KEY = "ccp-manager-sidebar-collapsed";

function readThemePreference(): ThemePreference {
  try {
    const value = window.localStorage.getItem(THEME_STORAGE_KEY);
    if (value === "light" || value === "dark" || value === "system") return value;
  } catch {
    // Local storage can be unavailable in hardened WebView contexts.
  }
  return "system";
}

function readSidebarPreference() {
  try {
    return window.localStorage.getItem(SIDEBAR_STORAGE_KEY) === "true";
  } catch {
    return false;
  }
}

function systemPrefersDark() {
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? true;
}

function healthCopy(health: ProxyHealth) {
  if (health === "healthy") return "代理在线";
  if (health === "attention") return "代理待修复";
  if (health === "offline") return "代理离线";
  return "代理待检查";
}

export function AppShell({
  activeSupplierId,
  activeSupplierName,
  agentScope,
  busy,
  children,
  codexThemeBackground,
  onAgentScopeChange,
  onInstallClaudeZhPatch,
  onInstallUpdate,
  onLaunchClaude,
  onNavigate,
  onRestartCodex,
  onSelectSupplier,
  proxyHealth,
  route,
  supplierOptions,
  updateInfo,
}: AppShellProps) {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(readSidebarPreference);
  const [themePreference, setThemePreference] = useState<ThemePreference>(readThemePreference);
  const [systemDark, setSystemDark] = useState(systemPrefersDark);
  const [themeMenuOpen, setThemeMenuOpen] = useState(false);
  const [supplierMenuOpen, setSupplierMenuOpen] = useState(false);
  const [commandOpen, setCommandOpen] = useState(false);
  const [commandQuery, setCommandQuery] = useState("");
  const [commandIndex, setCommandIndex] = useState(0);
  const commandInputRef = useRef<HTMLInputElement | null>(null);
  const commandPaletteRef = useRef<HTMLElement | null>(null);
  const commandReturnFocusRef = useRef<HTMLElement | null>(null);
  const themeMenuRef = useRef<HTMLDivElement | null>(null);
  const supplierMenuRef = useRef<HTMLDivElement | null>(null);
  const resolvedTheme = themePreference === "system" ? (systemDark ? "dark" : "light") : themePreference;
  const activePrimaryRoute = primaryRoute(route);
  const breadcrumbs = routeBreadcrumb(route);
  const domainTabs = routeDomainTabs(route);
  const updateAvailable = updateInfo?.updateAvailable === true;
  const updatePhase = updateInfo?.phase ?? "ready";
  const updateRunning = updateInfo?.status === "running" && updatePhase !== "checking";
  const updateComplete = updatePhase === "complete" || updateInfo?.launched === true;
  const updateFailed = updatePhase === "failed";
  const updatePercent = Math.max(0, Math.min(100, updateInfo?.progress ?? 0));
  const currentVersion = updateInfo?.currentVersion?.trim() || "未知";
  const latestVersion = updateInfo?.latestVersion?.trim() || "未知";
  const updateStatus = updateRunning
    ? updateProgressLabel(updatePhase, updatePercent)
    : updateComplete
      ? "安装程序已启动"
      : updateFailed
        ? "更新失败，点击重试"
        : "有可用更新";
  const supportedThemeBackground = codexThemeBackground && [
    "data:image/png;base64,",
    "data:image/jpeg;base64,",
    "data:image/webp;base64,",
  ].some((prefix) => codexThemeBackground.startsWith(prefix))
    ? codexThemeBackground
    : null;

  const commandItems = useMemo(() => {
    const query = commandQuery.trim().toLocaleLowerCase("zh-CN");
    if (!query) return routeCatalog;
    return routeCatalog.filter((item) =>
      [item.label, item.description, ...item.keywords].some((value) => value.toLocaleLowerCase("zh-CN").includes(query)),
    );
  }, [commandQuery]);

  const openCommand = useCallback(() => {
    commandReturnFocusRef.current = document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null;
    setCommandOpen(true);
  }, []);

  const closeCommand = useCallback(() => {
    setCommandOpen(false);
    window.setTimeout(() => commandReturnFocusRef.current?.focus(), 0);
  }, []);

  useEffect(() => {
    const media = window.matchMedia?.("(prefers-color-scheme: dark)");
    if (!media) return;
    const update = () => setSystemDark(media.matches);
    update();
    media.addEventListener?.("change", update);
    return () => media.removeEventListener?.("change", update);
  }, []);

  useEffect(() => {
    const root = document.documentElement;
    root.dataset.theme = resolvedTheme;
    root.classList.toggle("dark", resolvedTheme === "dark");
    root.classList.toggle("light", resolvedTheme === "light");
    root.style.colorScheme = resolvedTheme;
  }, [resolvedTheme]);

  useEffect(() => {
    try {
      window.localStorage.setItem(THEME_STORAGE_KEY, themePreference);
    } catch {
      // Theme still applies for this session when persistence is unavailable.
    }
  }, [themePreference]);

  useEffect(() => {
    try {
      window.localStorage.setItem(SIDEBAR_STORAGE_KEY, String(sidebarCollapsed));
    } catch {
      // Sidebar remains usable without persistence.
    }
  }, [sidebarCollapsed]);

  useEffect(() => {
    const handleShortcut = (event: globalThis.KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLocaleLowerCase() === "k") {
        event.preventDefault();
        if (!commandOpen) openCommand();
      } else if (event.key === "Escape") {
        if (commandOpen) closeCommand();
        setThemeMenuOpen(false);
        setSupplierMenuOpen(false);
      }
    };
    window.addEventListener("keydown", handleShortcut);
    return () => window.removeEventListener("keydown", handleShortcut);
  }, [closeCommand, commandOpen, openCommand]);

  useEffect(() => {
    if (!commandOpen) return;
    setCommandQuery("");
    setCommandIndex(0);
    window.setTimeout(() => commandInputRef.current?.focus(), 0);
  }, [commandOpen]);

  useEffect(() => {
    setCommandIndex(0);
  }, [commandQuery]);

  useEffect(() => {
    if (!themeMenuOpen) return;
    const closeOnOutsideClick = (event: MouseEvent) => {
      if (!themeMenuRef.current?.contains(event.target as Node)) setThemeMenuOpen(false);
    };
    document.addEventListener("mousedown", closeOnOutsideClick);
    return () => document.removeEventListener("mousedown", closeOnOutsideClick);
  }, [themeMenuOpen]);

  useEffect(() => {
    if (!supplierMenuOpen) return;
    const closeOnOutsideClick = (event: MouseEvent) => {
      if (!supplierMenuRef.current?.contains(event.target as Node)) setSupplierMenuOpen(false);
    };
    document.addEventListener("mousedown", closeOnOutsideClick);
    return () => document.removeEventListener("mousedown", closeOnOutsideClick);
  }, [supplierMenuOpen]);

  const navigate = (nextRoute: Route) => {
    if (commandOpen) closeCommand();
    onNavigate(nextRoute);
  };

  const handleCommandDialogKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key !== "Tab") return;
    const focusable = Array.from(
      commandPaletteRef.current?.querySelectorAll<HTMLElement>(
        'button:not([disabled]), input:not([disabled]), [href], [tabindex]:not([tabindex="-1"])',
      ) ?? [],
    );
    if (!focusable.length) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };

  const handleCommandKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setCommandIndex((index) => Math.min(index + 1, Math.max(0, commandItems.length - 1)));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setCommandIndex((index) => Math.max(0, index - 1));
    } else if (event.key === "Enter" && commandItems[commandIndex]) {
      event.preventDefault();
      navigate(commandItems[commandIndex].id);
    }
  };

  return (
    <div
      className={`ops-shell ${resolvedTheme}${sidebarCollapsed ? " is-sidebar-collapsed" : ""}${supportedThemeBackground ? " has-custom-background" : ""}`}
      data-theme-preference={themePreference}
    >
      {supportedThemeBackground ? (
        <img
          alt=""
          aria-hidden="true"
          className="ops-shell-background"
          draggable={false}
          src={supportedThemeBackground}
        />
      ) : null}
      <aside className="ops-rail" aria-label="一级导航">
        <button className="ops-brand" onClick={() => navigate("overview")} title="CCP 概览" type="button">
          <span aria-hidden="true" className="ops-brand-mark">CCP</span>
          <span className="ops-brand-copy">
            <strong>Control Plane</strong>
            <small>Local AI Operations</small>
          </span>
        </button>

        <nav>
          {routes.map((item) => {
            const Icon = item.icon;
            const active = activePrimaryRoute === item.id;
            return (
              <button
                aria-current={active ? "page" : undefined}
                className={active ? "active" : ""}
                key={item.id}
                onClick={() => navigate(item.id)}
                title={sidebarCollapsed ? item.label : undefined}
                type="button"
              >
                <Icon aria-hidden="true" className="h-4 w-4" />
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>

        <div className="ops-rail-footer">
          <div className="ops-theme-control" ref={themeMenuRef}>
            <button
              aria-expanded={themeMenuOpen}
              aria-haspopup="menu"
              className="ops-rail-utility"
              onClick={() => setThemeMenuOpen((open) => !open)}
              title="外观主题"
              type="button"
            >
              {themePreference === "light" ? <Sun aria-hidden="true" className="h-4 w-4" /> : themePreference === "dark" ? <Moon aria-hidden="true" className="h-4 w-4" /> : <Laptop aria-hidden="true" className="h-4 w-4" />}
              <span>{themePreference === "system" ? "跟随系统" : themePreference === "dark" ? "深色外观" : "浅色外观"}</span>
            </button>
            {themeMenuOpen ? (
              <div className="ops-theme-menu" role="menu">
                {([
                  ["system", "跟随系统", Laptop],
                  ["light", "浅色", Sun],
                  ["dark", "深色", Moon],
                ] as const).map(([value, label, Icon]) => (
                  <button key={value} onClick={() => { setThemePreference(value); setThemeMenuOpen(false); }} role="menuitem" type="button">
                    <Icon aria-hidden="true" className="h-4 w-4" />
                    <span>{label}</span>
                    {themePreference === value ? <Check aria-hidden="true" className="h-4 w-4" /> : null}
                  </button>
                ))}
              </div>
            ) : null}
          </div>
          <button
            aria-label={sidebarCollapsed ? "展开侧栏" : "折叠侧栏"}
            className="ops-rail-utility"
            onClick={() => setSidebarCollapsed((collapsed) => !collapsed)}
            title={sidebarCollapsed ? "展开侧栏" : "折叠侧栏"}
            type="button"
          >
            {sidebarCollapsed ? <PanelLeftOpen aria-hidden="true" className="h-4 w-4" /> : <PanelLeftClose aria-hidden="true" className="h-4 w-4" />}
            <span>{sidebarCollapsed ? "展开侧栏" : "折叠侧栏"}</span>
          </button>
        </div>
      </aside>

      <main className="ops-workspace">
        <header className="ops-topbar" data-tauri-drag-region>
          {route === "overview" ? (
            <div className="ops-overview-heading">
              <strong>运维概览</strong>
              <small>供应商、路由与 Agent 运行态</small>
            </div>
          ) : (
            <div className="ops-breadcrumb" aria-label="当前位置">
              {breadcrumbs.map((item, index) => (
                <span key={`${item}-${index}`}>
                  {index ? <ChevronRight aria-hidden="true" className="h-3 w-3" /> : null}
                  <span>{item}</span>
                </span>
              ))}
            </div>
          )}

          <div className="ops-commandbar">
            <div className="ops-command-context">
              <div className="ops-agent-scope" aria-label="Agent 范围" role="group">
                {([
                  ["codex", "Codex"],
                  ["claude", "Claude"],
                ] as const).map(([value, label]) => (
                  <button aria-pressed={agentScope === value} className={agentScope === value ? "active" : ""} key={value} onClick={() => onAgentScopeChange(value)} type="button">
                    {label}
                  </button>
                ))}
              </div>
              <button className="ops-command-search" onClick={openCommand} type="button">
                <Search aria-hidden="true" className="h-4 w-4" />
                <span>搜索页面与命令</span>
                <kbd><Command aria-hidden="true" className="h-3 w-3" />K</kbd>
              </button>
              <div className="ops-supplier-control" ref={supplierMenuRef}>
                <button
                  aria-expanded={supplierMenuOpen}
                  aria-haspopup="menu"
                  className={`ops-runtime-chip supplier ${proxyHealth}`}
                  disabled={busy || supplierOptions.length === 0}
                  onClick={() => setSupplierMenuOpen((open) => !open)}
                  title={`当前供应商：${activeSupplierName}；${healthCopy(proxyHealth)}`}
                  type="button"
                >
                  <span aria-hidden="true" className="ops-runtime-dot" />
                  <span className="ops-supplier-name">{activeSupplierName}</span>
                  <span className="ops-supplier-current">· 当前</span>
                  <ChevronDown aria-hidden="true" />
                </button>
                {supplierMenuOpen ? (
                  <div className="ops-supplier-menu" role="menu" aria-label="切换当前供应商">
                    <header>
                      <strong>当前供应商</strong>
                      <small>{agentScope === "claude" ? "Claude" : "Codex"}</small>
                    </header>
                    <div>
                      {supplierOptions.map((option) => {
                        const selected = option.id === activeSupplierId;
                        return (
                          <button
                            aria-checked={selected}
                            key={option.id}
                            onClick={() => {
                              setSupplierMenuOpen(false);
                              if (!selected) onSelectSupplier(option.id);
                            }}
                            role="menuitemradio"
                            type="button"
                          >
                            <span>
                              <strong>{option.name}</strong>
                              <small>{option.targetApp === "claude-desktop" ? "Claude Desktop" : option.targetApp === "claude" ? "Claude" : "Codex"}</small>
                            </span>
                            {selected ? <Check aria-hidden="true" /> : null}
                          </button>
                        );
                      })}
                    </div>
                    <button className="ops-supplier-manage" onClick={() => { setSupplierMenuOpen(false); navigate("supplier"); }} type="button">
                      管理供应商
                      <ChevronRight aria-hidden="true" />
                    </button>
                  </div>
                ) : null}
              </div>
            </div>
            <div className="ops-command-actions">
              {updateAvailable ? (
              <div className={`ops-update-control${updateRunning ? " is-running" : ""}${updateComplete ? " is-complete" : ""}${updateFailed ? " is-failed" : ""}`}>
                <button
                  aria-busy={updateRunning}
                  aria-describedby="ops-update-popover"
                  aria-label={`${updateStatus}，${latestVersion}`}
                  className="ops-update-trigger"
                  disabled={busy || updateRunning}
                  onClick={onInstallUpdate}
                  type="button"
                >
                  {updateRunning ? (
                    <LoaderCircle aria-hidden="true" className="spin" strokeWidth={3} />
                  ) : updateComplete ? (
                    <CircleCheck aria-hidden="true" strokeWidth={3} />
                  ) : updateFailed ? (
                    <TriangleAlert aria-hidden="true" strokeWidth={3} />
                  ) : (
                    <ArrowDownToLine aria-hidden="true" strokeWidth={3} />
                  )}
                </button>
                <div className="ops-update-popover" id="ops-update-popover" role="status" aria-live="polite">
                  <header>
                    <span>
                      <strong>发现 CCP 更新</strong>
                      <small>{currentVersion} → {latestVersion}</small>
                    </span>
                    <em>{updateRunning ? `${Math.round(updatePercent)}%` : updateComplete ? "DONE" : "NEW"}</em>
                  </header>
                  <p>{updateInfo.releaseSummary?.trim() || "新版本已就绪，可直接下载并启动安装程序。"}</p>
                  <div className="ops-update-meta">
                    <span>{updateStatus}</span>
                    {updateRunning ? (
                      <span>{formatDownloadBytes(updateInfo.downloadedBytes)} / {formatDownloadBytes(updateInfo.totalBytes)}</span>
                    ) : null}
                  </div>
                  {updateRunning ? (
                    <div
                      aria-label="更新下载进度"
                      aria-valuemax={100}
                      aria-valuemin={0}
                      aria-valuenow={Math.round(updatePercent)}
                      className="ops-update-progress"
                      role="progressbar"
                    >
                      <span style={{ width: `${updatePercent}%` }} />
                    </div>
                  ) : null}
                </div>
              </div>
              ) : null}
              <button className="ops-icon-command ops-action-command" disabled={busy} onClick={onRestartCodex} title="启动或重启 Codex" type="button">
                <Rocket aria-hidden="true" className="h-4 w-4" />
                <span>启动/重启 Codex</span>
              </button>
              <button className="ops-icon-command ops-action-command" disabled={busy} onClick={onLaunchClaude} title="启动或重启 Claude" type="button">
                <MessageCircle aria-hidden="true" className="h-4 w-4" />
                <span>启动/重启 Claude</span>
              </button>
              <button className="ops-icon-command ops-action-command ops-primary-command claude-zh-success" disabled={busy} onClick={onInstallClaudeZhPatch} title="写入 Claude 本机汉化资源" type="button">
                <Languages aria-hidden="true" className="h-4 w-4" />
                <span>Claude 一键汉化</span>
              </button>
            </div>
          </div>
        </header>

        <section className="ops-screen" data-route={route}>
          {route !== "prompts" && route !== "overview" ? (
            <div className="ops-page-heading">
              <div>
                <h1>{routeLabel(route)}</h1>
                <p>{routeSubtitle(route)}</p>
              </div>
              {domainTabs.length ? (
                <div className="ops-domain-tabs" aria-label={`${routeLabel(activePrimaryRoute)}视图`}>
                  {domainTabs.map((tab) => (
                    <button aria-pressed={route === tab.id} className={route === tab.id ? "active" : ""} key={tab.id} onClick={() => navigate(tab.id)} type="button">
                      {tab.label}
                    </button>
                  ))}
                </div>
              ) : null}
            </div>
          ) : null}
          <div className="ops-page-content">{children}</div>
        </section>
      </main>

      {commandOpen ? (
        <div className="ops-command-overlay" onMouseDown={(event) => { if (event.currentTarget === event.target) closeCommand(); }} role="presentation">
          <section
            aria-label="搜索页面与命令"
            aria-modal="true"
            className="ops-command-palette"
            onKeyDown={handleCommandDialogKeyDown}
            ref={commandPaletteRef}
            role="dialog"
          >
            <header>
              <Search aria-hidden="true" className="h-4 w-4" />
              <input
                aria-activedescendant={commandItems[commandIndex] ? `ops-command-option-${commandItems[commandIndex].id}` : undefined}
                aria-autocomplete="list"
                aria-controls="ops-command-results"
                aria-expanded="true"
                aria-label="搜索页面与命令"
                onChange={(event) => setCommandQuery(event.currentTarget.value)}
                onKeyDown={handleCommandKeyDown}
                placeholder="输入页面、功能或 Agent"
                ref={commandInputRef}
                role="combobox"
                value={commandQuery}
              />
              <button aria-label="关闭命令搜索" onClick={closeCommand} title="关闭" type="button"><X aria-hidden="true" className="h-4 w-4" /></button>
            </header>
            <div className="ops-command-results" id="ops-command-results" role="listbox">
              {commandItems.length ? commandItems.map((item, index) => {
                const Icon = item.icon;
                return (
                  <button
                    aria-selected={commandIndex === index}
                    className={commandIndex === index ? "active" : ""}
                    id={`ops-command-option-${item.id}`}
                    key={item.id}
                    onClick={() => navigate(item.id)}
                    onMouseEnter={() => setCommandIndex(index)}
                    role="option"
                    type="button"
                  >
                    <Icon aria-hidden="true" className="h-4 w-4" />
                    <span><strong>{item.label}</strong><small>{item.description}</small></span>
                    <kbd>Enter</kbd>
                  </button>
                );
              }) : <p className="ops-command-empty">没有匹配的页面或命令。</p>}
            </div>
          </section>
        </div>
      ) : null}
    </div>
  );
}
