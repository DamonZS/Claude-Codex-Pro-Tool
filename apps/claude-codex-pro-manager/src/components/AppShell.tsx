import {
  Check,
  ChevronRight,
  Command,
  Laptop,
  Languages,
  MessageCircle,
  Moon,
  PanelLeftClose,
  PanelLeftOpen,
  Rocket,
  Search,
  Sun,
  X,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent, type ReactNode } from "react";

import brandLogo from "../../../../assets/images/claude-codex-pro.png";
import {
  primaryRoute,
  routeBreadcrumb,
  routeCatalog,
  routeDomainTabs,
  routeLabel,
  routes,
  routeSubtitle,
} from "@/lib/routes";
import type { Route } from "@/types";

export type AgentScope = "all" | "codex" | "claude";
export type ThemePreference = "system" | "light" | "dark";
export type ProxyHealth = "healthy" | "attention" | "offline" | "unknown";

type AppShellProps = {
  activeSupplierName: string;
  agentScope: AgentScope;
  busy: boolean;
  children: ReactNode;
  onAgentScopeChange: (scope: AgentScope) => void;
  onInstallClaudeZhPatch: () => void;
  onLaunchClaude: () => void;
  onNavigate: (route: Route) => void;
  onRestartCodex: () => void;
  proxyHealth: ProxyHealth;
  route: Route;
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
  activeSupplierName,
  agentScope,
  busy,
  children,
  onAgentScopeChange,
  onInstallClaudeZhPatch,
  onLaunchClaude,
  onNavigate,
  onRestartCodex,
  proxyHealth,
  route,
}: AppShellProps) {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(readSidebarPreference);
  const [themePreference, setThemePreference] = useState<ThemePreference>(readThemePreference);
  const [systemDark, setSystemDark] = useState(systemPrefersDark);
  const [themeMenuOpen, setThemeMenuOpen] = useState(false);
  const [commandOpen, setCommandOpen] = useState(false);
  const [commandQuery, setCommandQuery] = useState("");
  const [commandIndex, setCommandIndex] = useState(0);
  const commandInputRef = useRef<HTMLInputElement | null>(null);
  const commandPaletteRef = useRef<HTMLElement | null>(null);
  const commandReturnFocusRef = useRef<HTMLElement | null>(null);
  const themeMenuRef = useRef<HTMLDivElement | null>(null);
  const resolvedTheme = themePreference === "system" ? (systemDark ? "dark" : "light") : themePreference;
  const activePrimaryRoute = primaryRoute(route);
  const breadcrumbs = routeBreadcrumb(route);
  const domainTabs = routeDomainTabs(route);

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
    <div className={`ops-shell ${resolvedTheme}${sidebarCollapsed ? " is-sidebar-collapsed" : ""}`} data-theme-preference={themePreference}>
      <aside className="ops-rail" aria-label="一级导航">
        <button className="ops-brand" onClick={() => navigate("overview")} title="CCP 概览" type="button">
          <img alt="" aria-hidden="true" src={brandLogo} />
          <span className="ops-brand-copy">
            <strong>CCP</strong>
            <small>AI 运维控制台</small>
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
          <div className="ops-breadcrumb" aria-label="当前位置">
            {breadcrumbs.map((item, index) => (
              <span key={`${item}-${index}`}>
                {index ? <ChevronRight aria-hidden="true" className="h-3 w-3" /> : null}
                <span>{item}</span>
              </span>
            ))}
          </div>

          <div className="ops-agent-scope" aria-label="Agent 范围" role="group">
            {([
              ["all", "全部"],
              ["codex", "Codex"],
              ["claude", "Claude"],
            ] as const).map(([value, label]) => (
              <button aria-pressed={agentScope === value} className={agentScope === value ? "active" : ""} key={value} onClick={() => onAgentScopeChange(value)} type="button">
                {label}
              </button>
            ))}
          </div>

          <div className="ops-commandbar">
            <button className="ops-command-search" onClick={openCommand} type="button">
              <Search aria-hidden="true" className="h-4 w-4" />
              <span>搜索页面与命令</span>
              <kbd><Command aria-hidden="true" className="h-3 w-3" />K</kbd>
            </button>
            <div className="ops-runtime-chip supplier" title={`当前供应商：${activeSupplierName}`}>
              <span className="ops-runtime-dot" />
              <span>{activeSupplierName}</span>
            </div>
            <div className={`ops-runtime-chip health ${proxyHealth}`} title={healthCopy(proxyHealth)}>
              <span className="ops-runtime-dot" />
              <span>{healthCopy(proxyHealth)}</span>
            </div>
            <button className="ops-icon-command ops-action-command" disabled={busy} onClick={onRestartCodex} title="启动或重启 Codex" type="button">
              <Rocket aria-hidden="true" className="h-4 w-4" />
              <span>启动/重启 Codex</span>
            </button>
            <button className="ops-icon-command ops-action-command" disabled={busy} onClick={onLaunchClaude} title="启动或重启 Claude" type="button">
              <MessageCircle aria-hidden="true" className="h-4 w-4" />
              <span>启动/重启 Claude</span>
            </button>
            <button className="ops-icon-command ops-action-command ops-primary-command" disabled={busy} onClick={onInstallClaudeZhPatch} title="写入 Claude 本机汉化资源" type="button">
              <Languages aria-hidden="true" className="h-4 w-4" />
              <span>Claude 一键汉化</span>
            </button>
          </div>
        </header>

        <section className="ops-screen">
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
