import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  Download,
  ExternalLink,
  FileCode2,
  Info,
  KeyRound,
  Languages,
  LayoutDashboard,
  MessageCircle,
  Moon,
  Network,
  PackageSearch,
  PencilRuler,
  Power,
  RefreshCw,
  Rocket,
  Settings,
  ShieldCheck,
  Sun,
  Trash2,
  Wrench,
  X,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import { invokeCommand } from "@/tauriBridge";

type Status = "ok" | "failed" | "not_implemented" | "not_checked" | string;

type CommandResult<T> = T & {
  status: Status;
  message: string;
};

type PathState = {
  status: string;
  path: string | null;
};

type LaunchStatus = {
  status: string;
  message: string;
  started_at_ms: number;
  debug_port: number | null;
  helper_port: number | null;
  codex_app: string | null;
};

type OverviewResult = CommandResult<{
  codex_app: PathState;
  codex_version: string | null;
  silent_shortcut: PathState;
  management_shortcut: PathState;
  latest_launch: LaunchStatus | null;
  current_version: string;
  update_status: string;
  settings_path: string;
  logs_path: string;
}>;

type ClaudeDesktopResult = CommandResult<{
  processCount: number;
  executablePaths: string[];
  installKind: string;
  cdpStatus: string;
  cdpBlocker: string;
  debugFlagsPresent: boolean;
  debugPorts: number[];
  listeningPorts: number[];
  debugEvidence: string[];
  supportedIntegration: string;
  integrityStatus: string;
  integrityMessage: string;
  executableAudits: Array<Record<string, unknown>>;
}>;

type ClaudeChineseWindowResult = CommandResult<{
  open: boolean;
  label: string;
  defaultUrl: string;
  injectionMode: string;
  cdpStatus: string;
  cdpBlocker: string;
  officialInstallKind: string;
}>;

type BackendSettings = {
  codexAppPath: string;
  codexExtraArgs: string[];
  providerSyncEnabled: boolean;
  providerSyncSavedProviders: string[];
  providerSyncManualProviders: string[];
  providerSyncLastSelectedProvider: string;
  relayProfilesEnabled: boolean;
  enhancementsEnabled: boolean;
  computerUseGuardEnabled: boolean;
  codexAppPluginEntryUnlock: boolean;
  codexAppPluginMarketplaceUnlock: boolean;
  codexAppForcePluginInstall: boolean;
  codexAppModelWhitelistUnlock: boolean;
  codexAppSessionDelete: boolean;
  codexAppMarkdownExport: boolean;
  codexAppProjectMove: boolean;
  codexAppConversationTimeline: boolean;
  codexAppConversationView: boolean;
  codexAppThreadScrollRestore: boolean;
  codexAppZedRemoteOpen: boolean;
  zedRemoteOpenStrategy: string;
  zedRemoteProjectRegistryEnabled: boolean;
  zedRemoteSyncToZedSettings: boolean;
  codexAppUpstreamWorktreeCreate: boolean;
  codexAppNativeMenuPlacement: boolean;
  claudeAppChineseOverlayEnabled: boolean;
  codexAppServiceTierControls: boolean;
  codexAppImageOverlayEnabled: boolean;
  codexAppImageOverlayPath: string;
  codexAppImageOverlayOpacity: number;
  codexGoalsEnabled: boolean;
  launchMode: "patch" | "relay";
  relayBaseUrl: string;
  relayApiKey: string;
  relayProfiles: RelayProfile[];
  relayCommonConfigContents: string;
  relayContextConfigContents: string;
  activeRelayId: string;
  relayTestModel: string;
  cliWrapperEnabled: boolean;
  cliWrapperBaseUrl: string;
  cliWrapperApiKey: string;
  cliWrapperApiKeyEnv: string;
};

type RelayProfile = {
  id: string;
  name: string;
  model: string;
  baseUrl: string;
  upstreamBaseUrl: string;
  apiKey: string;
  protocol: string;
  relayMode: string;
  officialMixApiKey: boolean;
  testModel: string;
  configContents: string;
  authContents: string;
  useCommonConfig: boolean;
  contextSelection: {
    mcpServers: string[];
    skills: string[];
    plugins: string[];
  };
  contextSelectionInitialized: boolean;
  contextWindow: string;
  autoCompactLimit: string;
  modelList: string;
  userAgent: string;
};

type SettingsResult = CommandResult<{
  settings: BackendSettings;
  settings_path: string;
  user_scripts: UserScriptInventory;
}>;

type UserScriptInventory = {
  enabled?: boolean;
  scripts?: Array<{
    key: string;
    name: string;
    source: string;
    enabled: boolean;
    status: string;
    error: string;
    market_id?: string;
    version?: string;
    installed?: boolean;
    source_url?: string;
    homepage?: string;
  }>;
};

type ScriptMarketItem = {
  id: string;
  name: string;
  description: string;
  version: string;
  author: string;
  tags: string[];
  homepage: string;
  script_url: string;
  sha256: string;
  installed: boolean;
  installedVersion: string;
  updateAvailable: boolean;
};

type ScriptMarketResult = CommandResult<{
  market: {
    status: string;
    message: string;
    indexUrl: string;
    updatedAt: string;
    scripts: ScriptMarketItem[];
  };
  user_scripts: UserScriptInventory;
}>;

type PluginInstallKind = "claude_plugin_marketplace" | "claude_desktop_mcp" | "mcp_server" | "skill_bundle" | "resource_link";
type PluginInstallStatus = "notInstalled" | "installed" | "needsReview" | "unsupported";

type PluginCatalogItem = {
  id: string;
  name: string;
  description: string;
  sourceId: string;
  sourceLabel: string;
  sourceUrl: string;
  category: string;
  author: string;
  homepage: string;
  license: string;
  tags: string[];
  installKind: PluginInstallKind;
  installStatus: PluginInstallStatus;
  installCommand: string[];
  configPreview: string;
  risk: string;
  requirements: string[];
};

type PluginCatalogSource = {
  id: string;
  label: string;
  url: string;
  status: string;
  message: string;
  itemCount: number;
};

type PluginHubResult = CommandResult<{
  catalog: {
    updatedAt: string;
    sources: PluginCatalogSource[];
    items: PluginCatalogItem[];
  };
}>;

type PluginInstallPreviewResult = CommandResult<{
  item: PluginCatalogItem;
  canInstall: boolean;
  action: string;
  command: string[];
  configDiff: string;
  message: string;
}>;

type PluginInstallOutcomeResult = CommandResult<{
  item: PluginCatalogItem;
  preview: unknown;
  installed: boolean;
  installMessage?: string;
  stdout: string;
  stderr: string;
  backupPath: string | null;
}>;

type LogsResult = CommandResult<{
  path: string;
  text: string;
  lines: number;
}>;

type WatcherPayload = {
  enabled: boolean;
  disabled_flag: string;
};

type WatcherResult = CommandResult<WatcherPayload>;

type InstallEntrypointsResult = CommandResult<{
  silent_shortcut: {
    installed: boolean;
    path: string | null;
  };
  management_shortcut: {
    installed: boolean;
    path: string | null;
  };
}>;

type Route =
  | "overview"
  | "relay"
  | "context"
  | "pluginHub"
  | "promptOptimizer"
  | "scripts"
  | "maintenance"
  | "logs"
  | "settings";
type Theme = "light" | "dark";
const OPS_THEME_STORAGE_KEY = "claude-codex-pro-ops-theme";

declare global {
  interface Window {
    __CLAUDE_CODEX_PRO_INITIAL_ROUTE?: Route;
  }
}

const routes: Array<{ id: Route; label: string; icon: LucideIcon }> = [
  { id: "overview", label: "概览", icon: LayoutDashboard },
  { id: "relay", label: "供应商", icon: KeyRound },
  { id: "context", label: "工具", icon: Network },
  { id: "pluginHub", label: "插件", icon: PackageSearch },
  { id: "promptOptimizer", label: "提示词", icon: PencilRuler },
  { id: "scripts", label: "脚本", icon: FileCode2 },
  { id: "maintenance", label: "维护", icon: Wrench },
  { id: "logs", label: "日志", icon: Info },
  { id: "settings", label: "设置", icon: Settings },
];

function isRoute(value: unknown): value is Route {
  return routes.some((item) => item.id === value);
}

function statusOk(status?: string | null) {
  return status === "ok" || status === "found" || status === "installed" || status === "running";
}

function compactPath(path?: string | null) {
  if (!path) return "未设置";
  if (path.length <= 58) return path;
  return `${path.slice(0, 24)}...${path.slice(-28)}`;
}

export function App() {
  const [theme, setTheme] = useState<Theme>(() => (localStorage.getItem(OPS_THEME_STORAGE_KEY) === "dark" ? "dark" : "light"));
  const [route, setRoute] = useState<Route>(() => initialRoute());
  const [notice, setNotice] = useState<{ title: string; message: string; status?: Status } | null>(null);
  const [busy, setBusy] = useState(false);
  const [overview, setOverview] = useState<OverviewResult | null>(null);
  const [claudeDesktop, setClaudeDesktop] = useState<ClaudeDesktopResult | null>(null);
  const [claudeChinese, setClaudeChinese] = useState<ClaudeChineseWindowResult | null>(null);
  const [settings, setSettings] = useState<SettingsResult | null>(null);
  const [settingsDraft, setSettingsDraft] = useState<BackendSettings | null>(null);
  const [pluginHub, setPluginHub] = useState<PluginHubResult | null>(null);
  const [pluginPreview, setPluginPreview] = useState<PluginInstallPreviewResult | null>(null);
  const [scriptMarket, setScriptMarket] = useState<ScriptMarketResult | null>(null);
  const [logs, setLogs] = useState<LogsResult | null>(null);
  const [watcher, setWatcher] = useState<WatcherResult | null>(null);
  const isPromptOptimizerStandaloneWindow = window.__CLAUDE_CODEX_PRO_INITIAL_ROUTE === "promptOptimizer";

  const call = <T,>(command: string, args?: Record<string, unknown>) => invokeCommand<T>(command, args);
  const notifyIfNeedsAttention = (next: { title: string; message: string; status?: Status }) => {
    if (!statusOk(next.status)) setNotice(next);
  };

  const run = async <T,>(task: () => Promise<T>, title?: string): Promise<T | null> => {
    setBusy(true);
    try {
      return await task();
    } catch (error) {
      setNotice({ title: title || "调用失败", message: stringifyError(error), status: "failed" });
      return null;
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    if (!notice) return;
    const timeout = window.setTimeout(() => setNotice(null), 5200);
    return () => window.clearTimeout(timeout);
  }, [notice]);

  const refreshOverview = async (silent = false) => {
    const result = await run(() => call<OverviewResult>("load_overview"), "概览");
    if (result) {
      setOverview(result);
      if (!silent) notifyIfNeedsAttention({ title: "概览", message: result.message, status: result.status });
    }
    return result;
  };

  const refreshClaude = async (silent = false) => {
    const desktop = await run(() => call<ClaudeDesktopResult>("load_claude_desktop_status"), "Claude 状态");
    if (desktop) setClaudeDesktop(desktop);
    const wrapped = await run(() => call<ClaudeChineseWindowResult>("load_claude_chinese_window_status"), "Claude 中文窗口");
    if (wrapped) setClaudeChinese(wrapped);
    if (!silent && desktop) notifyIfNeedsAttention({ title: "Claude 状态", message: desktop.message, status: desktop.status });
  };

  const refreshSettings = async (silent = false) => {
    const result = await run(() => call<SettingsResult>("load_settings"), "设置");
    if (result) {
      setSettings(result);
      setSettingsDraft(result.settings);
      if (!silent) notifyIfNeedsAttention({ title: "设置", message: result.message, status: result.status });
    }
    return result;
  };

  const refreshPluginHub = async (silent = false) => {
    const result = await run(() => call<PluginHubResult>("refresh_plugin_hub_catalog"), "插件中心");
    if (result) {
      setPluginHub(result);
      if (!silent) notifyIfNeedsAttention({ title: "插件中心", message: result.message, status: result.status });
    }
    return result;
  };

  const refreshScripts = async (silent = false) => {
    const result = await run(() => call<ScriptMarketResult>("refresh_script_market"), "脚本市场");
    if (result) {
      setScriptMarket(result);
      if (!silent) notifyIfNeedsAttention({ title: "脚本市场", message: result.message, status: result.status });
    }
    return result;
  };

  const refreshLogs = async (silent = false) => {
    const result = await run(() => call<LogsResult>("read_latest_logs", { request: { lines: 240 } }), "日志");
    if (result) {
      setLogs(result);
      if (!silent) notifyIfNeedsAttention({ title: "日志", message: result.message, status: result.status });
    }
    return result;
  };

  const refreshWatcher = async (silent = false) => {
    const result = await run(() => call<WatcherResult>("load_watcher_state"), "Watcher");
    if (result) {
      setWatcher(result);
      if (!silent) notifyIfNeedsAttention({ title: "Watcher", message: result.message, status: result.status });
    }
    return result;
  };

  const openClaudeChinese = async () => {
    const result = await run(() => call<ClaudeChineseWindowResult>("open_claude_chinese_window"), "Claude 中文窗口");
    if (result) {
      setClaudeChinese(result);
      notifyIfNeedsAttention({ title: "Claude 中文窗口", message: result.message, status: result.status });
      await refreshClaude(true);
    }
  };

  const launchClaudeDesktop = async () => {
    const result = await run(() => call<CommandResult<Record<string, unknown>>>("open_claude_desktop"), "启动 Claude");
    if (result) {
      notifyIfNeedsAttention({ title: "启动 Claude", message: result.message, status: result.status });
      await refreshClaude(true);
    }
  };

  const restartCodex = async () => {
    const result = await run(() => call<CommandResult<Record<string, unknown>>>("restart_claude_codex_pro", { request: {} }), "重启 Codex");
    if (result) {
      notifyIfNeedsAttention({ title: "重启 Codex", message: result.message, status: result.status });
      await refreshOverview(true);
    }
  };

  const launchCodex = async () => {
    const result = await run(() => call<CommandResult<Record<string, unknown>>>("launch_claude_codex_pro", { request: {} }), "启动 Codex");
    if (result) {
      notifyIfNeedsAttention({ title: "启动 Codex", message: result.message, status: result.status });
      await refreshOverview(true);
    }
  };

  const previewPlugin = async (id: string) => {
    const result = await run(() => call<PluginInstallPreviewResult>("preview_plugin_hub_install", { request: { id } }), "安装预览");
    if (result) {
      setPluginPreview(result);
      notifyIfNeedsAttention({ title: "安装预览", message: result.message, status: result.status });
    }
    return result;
  };

  const installPlugin = async (id: string) => {
    const preview = pluginPreview?.item.id === id ? pluginPreview : await previewPlugin(id);
    if (!preview) return;
    const details = [
      preview.command?.length ? `命令：${preview.command.join(" ")}` : "",
      preview.configDiff ? `配置：\n${preview.configDiff}` : "",
      preview.message,
    ].filter(Boolean).join("\n\n");
    if (!window.confirm(`确认安装？\n\n${details}`)) return;
    const result = await run(() => call<PluginInstallOutcomeResult>("install_plugin_hub_item", { request: { id } }), "安装插件");
    if (result) {
      const message = result.message || result.installMessage || result.stderr || result.stdout || "插件安装失败，请检查 Claude CLI 登录状态和插件安装预览。";
      notifyIfNeedsAttention({ title: "插件中心", message, status: result.status });
      await refreshPluginHub(true);
    }
  };

  const uninstallPlugin = async (id: string) => {
    if (!window.confirm("移除该安装记录？这不会静默删除第三方工具自身文件。")) return;
    const result = await run(() => call<PluginHubResult>("uninstall_plugin_hub_item", { request: { id } }), "移除插件记录");
    if (result) {
      setPluginHub(result);
      notifyIfNeedsAttention({ title: "插件中心", message: result.message, status: result.status });
    }
  };

  const installMarketScript = async (id: string) => {
    const result = await run(() => call<ScriptMarketResult>("install_market_script", { id }), "安装脚本");
    if (result) {
      setScriptMarket(result);
      notifyIfNeedsAttention({ title: "脚本市场", message: result.message, status: result.status });
    }
  };

  const openExternalUrl = async (url: string) => {
    await run(() => call<CommandResult<Record<string, unknown>>>("open_external_url", { url }), "打开链接");
  };

  const goPluginHub = async () => {
    setRoute("pluginHub");
    await refreshPluginHub(true);
  };

  const goPromptOptimizer = async () => {
    setRoute("promptOptimizer");
    await refreshRoute("promptOptimizer");
  };

  const repairEntrypoints = async () => {
    const result = await run(() => call<CommandResult<Record<string, unknown>>>("repair_shortcuts"), "修复入口");
    if (result) notifyIfNeedsAttention({ title: "修复入口", message: result.message, status: result.status });
    await refreshOverview(true);
  };

  const repairBackend = async () => {
    const result = await run(() => call<SettingsResult>("repair_backend"), "修复后端");
    if (result) {
      setSettings(result);
      setSettingsDraft(result.settings);
      notifyIfNeedsAttention({ title: "修复后端", message: result.message, status: result.status });
    }
  };

  const applyRelayMode = async () => {
    const result = await run(() => call<CommandResult<Record<string, unknown>>>("apply_relay_injection"), "官方混入 API Key");
    if (result) {
      notifyIfNeedsAttention({ title: "官方混入 API Key", message: result.message, status: result.status });
      await refreshSettings(true);
    }
  };

  const applyPureApiMode = async () => {
    const result = await run(() => call<CommandResult<Record<string, unknown>>>("apply_pure_api_injection"), "纯 API");
    if (result) {
      notifyIfNeedsAttention({ title: "纯 API", message: result.message, status: result.status });
      await refreshSettings(true);
    }
  };

  const clearRelayMode = async () => {
    const result = await run(() => call<CommandResult<Record<string, unknown>>>("clear_relay_injection"), "清除 API 模式");
    if (result) {
      notifyIfNeedsAttention({ title: "清除 API 模式", message: result.message, status: result.status });
      await refreshSettings(true);
    }
  };

  const saveSettings = async (next: BackendSettings) => {
    const result = await run(() => call<SettingsResult>("save_settings", { settings: next }), "保存设置");
    if (result) {
      setSettings(result);
      setSettingsDraft(result.settings);
      notifyIfNeedsAttention({ title: "保存设置", message: result.message, status: result.status });
    }
    return result;
  };

  const installEntrypoints = async () => {
    const result = await run(() => call<InstallEntrypointsResult>("install_entrypoints"), "安装入口");
    if (result) notifyIfNeedsAttention({ title: "安装入口", message: result.message, status: result.status });
    await refreshOverview(true);
  };

  const uninstallEntrypoints = async () => {
    if (!window.confirm("卸载入口会移除静默启动和管理工具快捷方式，不会删除配置数据。继续？")) return;
    const result = await run(
      () => call<InstallEntrypointsResult>("uninstall_entrypoints", { options: { removeOwnedData: false } }),
      "卸载入口",
    );
    if (result) notifyIfNeedsAttention({ title: "卸载入口", message: result.message, status: result.status });
    await refreshOverview(true);
  };

  const repairShortcuts = async () => {
    const result = await run(() => call<InstallEntrypointsResult>("repair_shortcuts"), "修复快捷方式");
    if (result) notifyIfNeedsAttention({ title: "修复快捷方式", message: result.message, status: result.status });
    await refreshOverview(true);
  };

  const watcherAction = async (command: "install_watcher" | "uninstall_watcher" | "enable_watcher" | "disable_watcher", title: string) => {
    const result = await run(() => call<WatcherResult>(command), title);
    if (result) {
      setWatcher(result);
      notifyIfNeedsAttention({ title, message: result.message, status: result.status });
    }
  };

  const resetSettings = async () => {
    if (!window.confirm("确认重置管理工具设置？该操作会恢复默认配置。")) return;
    const result = await run(() => call<SettingsResult>("reset_settings"), "重置设置");
    if (result) {
      setSettings(result);
      setSettingsDraft(result.settings);
      notifyIfNeedsAttention({ title: "重置设置", message: result.message, status: result.status });
    }
  };

  const resetImageOverlaySettings = async () => {
    const result = await run(() => call<SettingsResult>("reset_image_overlay_settings"), "重置图片覆盖");
    if (result) {
      setSettings(result);
      setSettingsDraft(result.settings);
      notifyIfNeedsAttention({ title: "重置图片覆盖", message: result.message, status: result.status });
    }
  };

  const refreshRoute = async (target = route) => {
    if (target === "overview") {
      await refreshOverview(true);
      await refreshClaude(true);
    } else if (target === "relay" || target === "context" || target === "settings") {
      await refreshSettings(true);
    } else if (target === "pluginHub") {
      await refreshPluginHub(true);
    } else if (target === "promptOptimizer") {
      await refreshSettings(true);
    } else if (target === "scripts") {
      await refreshSettings(true);
      await refreshScripts(true);
    } else if (target === "logs") {
      await refreshLogs(true);
    } else if (target === "maintenance") {
      await refreshOverview(true);
      await refreshSettings(true);
      await refreshWatcher(true);
    }
  };

  useEffect(() => {
    const navigate = (event: Event) => {
      const route = (event as CustomEvent<{ route?: unknown }>).detail?.route;
      if (!isRoute(route)) return;
      setRoute(route);
      void refreshRoute(route);
    };
    window.addEventListener("claude-codex-pro-navigate", navigate);
    return () => window.removeEventListener("claude-codex-pro-navigate", navigate);
  }, []);

  useEffect(() => {
    void (async () => {
      await refreshOverview(true);
      await refreshClaude(true);
      await refreshSettings(true);
      await refreshPluginHub(true);
      await refreshWatcher(true);
    })();
  }, []);

  useEffect(() => {
    document.documentElement.classList.toggle("dark", theme === "dark");
    document.documentElement.classList.toggle("light", theme === "light");
    localStorage.setItem(OPS_THEME_STORAGE_KEY, theme);
  }, [theme]);

  useEffect(() => {
    document.title = routeDocumentTitle(route);
  }, [route]);

  const actions = useMemo(
    () => ({
      refreshRoute,
      openClaudeChinese,
      launchClaudeDesktop,
      launchCodex,
      restartCodex,
      openExternalUrl,
      goPluginHub,
      goPromptOptimizer,
      previewPlugin,
      installPlugin,
      uninstallPlugin,
      installMarketScript,
      refreshPluginHub,
      refreshScripts,
      repairEntrypoints,
      repairBackend,
      applyRelayMode,
      applyPureApiMode,
      clearRelayMode,
      saveSettings,
      installEntrypoints,
      uninstallEntrypoints,
      repairShortcuts,
      installWatcher: () => watcherAction("install_watcher", "安装 Watcher"),
      uninstallWatcher: () => watcherAction("uninstall_watcher", "移除 Watcher"),
      enableWatcher: () => watcherAction("enable_watcher", "启用 Watcher"),
      disableWatcher: () => watcherAction("disable_watcher", "禁用 Watcher"),
      resetSettings,
      resetImageOverlaySettings,
      refreshLogs,
      refreshWatcher,
      setTheme,
    }),
    [route, pluginPreview],
  );

  if (isPromptOptimizerStandaloneWindow) {
    return (
      <div className={`ops-shell prompt-optimizer-window-shell ${theme}`}>
        <main className="prompt-optimizer-window-workspace">
          <PromptOptimizerScreen actions={actions} />
        </main>
        {notice ? <Notice notice={notice} onClose={() => setNotice(null)} /> : null}
      </div>
    );
  }

  return (
    <div className={`ops-shell ${theme}`}>
      <aside className="ops-rail">
        <div className="ops-brand" title="Claude Codex Pro">CCP</div>
        <nav>
          {routes.map((item) => {
            const Icon = item.icon;
            return (
              <button className={route === item.id ? "active" : ""} key={item.id} onClick={() => { setRoute(item.id); void refreshRoute(item.id); }} title={item.label} type="button">
                <Icon className="h-4 w-4" />
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>
      </aside>
      <main className="ops-workspace">
        <header className="ops-topbar">
          <div>
            <h1>{routeLabel(route)}</h1>
            <p>{routeSubtitle(route)}</p>
          </div>
          <div className="ops-commandbar">
            <Button disabled={busy} onClick={() => actions.setTheme(theme === "dark" ? "light" : "dark")} size="icon" variant="outline">
              {theme === "dark" ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
            </Button>
            <Button disabled={busy} onClick={() => void actions.restartCodex()} variant="outline">
              <Rocket className="h-4 w-4" />
              重启 Codex
            </Button>
            <Button disabled={busy} onClick={() => void actions.launchClaudeDesktop()} variant="outline">
              <MessageCircle className="h-4 w-4" />
              启动 Claude
            </Button>
            <Button className="ops-primary-command" disabled={busy} onClick={() => void actions.openClaudeChinese()}>
              <Languages className="h-4 w-4" />
              Claude 中文窗口
            </Button>
            <Button disabled={busy} onClick={() => void actions.refreshRoute()} size="icon" variant="outline">
              <RefreshCw className="h-4 w-4" />
            </Button>
          </div>
        </header>
        <section className="ops-screen">
          {route === "overview" ? <OverviewScreen actions={actions} claudeChinese={claudeChinese} claudeDesktop={claudeDesktop} overview={overview} pluginHub={pluginHub} /> : null}
          {route === "relay" ? <RelayScreen actions={actions} settings={settings} /> : null}
          {route === "context" ? <ContextScreen settings={settings} /> : null}
          {route === "pluginHub" ? <PluginHubScreen actions={actions} hub={pluginHub} preview={pluginPreview} /> : null}
          {route === "promptOptimizer" ? <PromptOptimizerScreen actions={actions} /> : null}
          {route === "scripts" ? <ScriptsScreen actions={actions} market={scriptMarket} settings={settings} /> : null}
          {route === "maintenance" ? <MaintenanceScreen actions={actions} overview={overview} settings={settings} watcher={watcher} /> : null}
          {route === "logs" ? <LogsScreen actions={actions} logs={logs} /> : null}
          {route === "settings" ? <SettingsScreen actions={actions} claudeChinese={claudeChinese} draft={settingsDraft} onDraftChange={setSettingsDraft} overview={overview} settings={settings} watcher={watcher} /> : null}
        </section>
      </main>
      {notice ? <Notice notice={notice} onClose={() => setNotice(null)} /> : null}
    </div>
  );
}

function OverviewScreen({
  actions,
  overview,
  claudeDesktop,
  claudeChinese,
  pluginHub,
}: {
  actions: ReturnType<typeof createActionsShape>;
  overview: OverviewResult | null;
  claudeDesktop: ClaudeDesktopResult | null;
  claudeChinese: ClaudeChineseWindowResult | null;
  pluginHub: PluginHubResult | null;
}) {
  const pluginCount = pluginHub?.catalog.items.length ?? 0;
  const installedPlugins = pluginHub?.catalog.items.filter((item) => item.installStatus === "installed").length ?? 0;
  return (
    <div className="ops-dashboard">
      <button className="relay-banner" onClick={() => void actions.openExternalUrl("https://api.toporeduce.cn")} type="button">
        <Network className="h-5 w-5" />
        <div>
          <span>官方中转站</span>
          <strong>拓扑熵减API</strong>
          <p>ClaudeCodexPro 官方中转站，主打稳定接入和划算价格，支持 GPT-5.5、GPT-5.4、Claude Opus 4.8、Claude Opus 4.7、gpt-image-2 等模型与图像能力。</p>
        </div>
        <ExternalLink className="h-4 w-4" />
      </button>
      <div className="ops-matrix">
        <StatusTile icon={Activity} label="Codex 版本" status={overview?.codex_version ? "ok" : "not_checked"} value={overview?.codex_version ?? "未检测"} />
        <StatusTile icon={Power} label="最近启动" status={overview?.latest_launch?.status ?? "not_checked"} value={overview?.latest_launch?.status ?? "无记录"} />
        <StatusTile icon={Languages} label="中文窗口" status={claudeChinese?.open ? "ok" : "not_checked"} value={claudeChinese?.open ? "已打开" : "未打开"} />
        <StatusTile icon={MessageCircle} label="官方 Claude" status={claudeDesktop?.status ?? "not_checked"} value={`${claudeDesktop?.installKind ?? "unknown"} / ${claudeDesktop?.cdpStatus ?? "unknown"}`} />
      </div>
      <div className="ops-columns">
        <Panel title="核心动作" detail="Codex 与 Claude 的启动入口已分离。">
          <ActionButton icon={Rocket} label="启动 Codex" onClick={() => void actions.launchCodex()} />
          <ActionButton icon={RefreshCw} label="重启 Codex" onClick={() => void actions.restartCodex()} />
          <ActionButton icon={MessageCircle} label="启动官方 Claude" onClick={() => void actions.launchClaudeDesktop()} />
          <ActionButton icon={Languages} label="打开 Claude 中文窗口" onClick={() => void actions.openClaudeChinese()} />
        </Panel>
        <Panel title="Claude 诊断" detail="官方 MSIX 只读诊断，中文化在包装窗口完成。">
          <InfoRow label="安装类型" value={claudeDesktop?.installKind ?? "未检测"} />
          <InfoRow label="CDP 状态" value={claudeDesktop?.cdpStatus ?? "未检测"} />
          <InfoRow label="阻断原因" value={claudeDesktop?.cdpBlocker || "无"} />
          <InfoRow label="包装窗口" value={`${claudeChinese?.injectionMode ?? "wrapped_webview"} · ${claudeChinese?.defaultUrl ?? "https://claude.ai/new"}`} />
        </Panel>
        <Panel title="插件中心" detail={`${pluginCount} 个条目，${installedPlugins} 个已安装记录。`}>
          <ActionButton icon={PackageSearch} label="刷新插件目录" onClick={() => void actions.refreshPluginHub()} />
          <ActionButton icon={PackageSearch} label="打开插件中心" onClick={() => void actions.goPluginHub()} />
          <ActionButton icon={ExternalLink} label="官方插件仓库" onClick={() => void actions.openExternalUrl("https://github.com/anthropics/claude-plugins-official")} />
          <InfoRow label="官方市场" value={pluginHub?.catalog.sources.find((source) => source.id === "official")?.message ?? "未加载"} />
          <InfoRow label="社区资源" value={pluginHub?.catalog.sources.find((source) => source.id === "awesome")?.message ?? "未加载"} />
        </Panel>
        <Panel title="提示词工坊" detail="集成 linshenkx/prompt-optimizer，应用内路由打开控制页。">
          <ActionButton icon={PencilRuler} label="打开提示词优化器" onClick={() => void actions.goPromptOptimizer()} />
          <ActionButton icon={ExternalLink} label="查看开源仓库" onClick={() => void actions.openExternalUrl("https://github.com/linshenkx/prompt-optimizer")} />
          <InfoRow label="集成方式" value="外部 WebView / 本地 Docker / MCP" />
          <InfoRow label="许可证" value="AGPL-3.0-only" />
        </Panel>
      </div>
    </div>
  );
}

function RelayScreen({ actions, settings }: { actions: ReturnType<typeof createActionsShape>; settings: SettingsResult | null }) {
  const profiles = settings?.settings.relayProfiles ?? [];
  const active = profiles.find((profile) => profile.id === settings?.settings.activeRelayId) ?? profiles[0];
  return (
    <div className="ops-two-column">
      <div className="ops-wide-column">
        <Panel title="供应商配置" detail="选择写入方式后会更新 Codex 配置；真实 Key 不会写入日志或提示。">
          <div className="info-grid">
            <InfoRow label="当前供应商" value={active?.name || active?.id || "未配置"} />
            <InfoRow label="模式" value={active?.relayMode || "official"} />
            <InfoRow label="模型" value={active?.model || settings?.settings.relayTestModel || "默认"} />
            <InfoRow label="Base URL" value={active?.baseUrl || settings?.settings.relayBaseUrl || "官方登录"} />
            <InfoRow label="配置路径" value={settings?.settings_path ?? "未加载"} />
          </div>
          <div className="action-row">
            <Button onClick={() => void actions.applyRelayMode()}>
              <KeyRound className="h-4 w-4" />
              官方混入 API Key
            </Button>
            <Button onClick={() => void actions.applyPureApiMode()} variant="outline">
              <Network className="h-4 w-4" />
              纯 API
            </Button>
            <Button onClick={() => void actions.clearRelayMode()} variant="outline">
              <Trash2 className="h-4 w-4" />
              清除 API 模式
            </Button>
          </div>
        </Panel>
        <Panel title="供应商列表" detail={`${profiles.length} 个配置；当前页面保留关键操作，复杂编辑在设置文件中维护。`}>
          <div className="ops-status-list">
            {profiles.length ? profiles.map((profile) => (
              <StatusRow
                key={profile.id}
                label={profile.name || profile.id}
                status={profile.id === settings?.settings.activeRelayId ? "running" : "not_checked"}
                value={`${profile.relayMode || "official"} · ${profile.model || profile.testModel || "默认模型"}`}
              />
            )) : <Empty text="暂无供应商配置。" />}
          </div>
        </Panel>
      </div>
      <div className="stack">
        <Panel title="写入模式" detail="按使用场景选择，不混淆 Claude Desktop 插件安装。">
          <div className="ops-status-list">
            <StatusRow label="官方混入 API Key" status={active?.officialMixApiKey ? "running" : "not_checked"} value="保留官方账号能力，把模型请求转到自定义兼容 API。" />
            <StatusRow label="纯 API" status={active?.relayMode === "pure_api" ? "running" : "not_checked"} value="写入 custom provider，并将 auth 状态切换到当前供应商。" />
            <StatusRow label="清除 API 模式" status="not_checked" value="移除中转 API 配置，回到官方 ChatGPT 登录态。" />
          </div>
        </Panel>
        <Panel title="当前配置摘录" detail="只展示路径和非敏感字段。">
          <div className="info-grid compact">
            <InfoRow label="Provider Sync" value={settings?.settings.providerSyncEnabled ? "开启" : "关闭"} />
            <InfoRow label="供应商开关" value={settings?.settings.relayProfilesEnabled ? "开启" : "关闭"} />
            <InfoRow label="协议" value={active?.protocol || "responses"} />
            <InfoRow label="测试模型" value={active?.testModel || settings?.settings.relayTestModel || "默认"} />
          </div>
        </Panel>
      </div>
    </div>
  );
}

function ContextScreen({ settings }: { settings: SettingsResult | null }) {
  const common = settings?.settings.relayContextConfigContents || settings?.settings.relayCommonConfigContents || "";
  return (
    <Panel title="工具与插件配置" detail="MCP、Skills、Plugins 仍保存在统一 TOML 配置中。">
      <div className="ops-note">
        <ShieldCheck className="h-4 w-4" />
        <span>插件中心安装社区 MCP 时只保存配置草案，不会静默执行第三方脚本。</span>
      </div>
      <pre className="ops-code">{common.trim() || "暂无通用 MCP / Skills / Plugins 配置。"}</pre>
    </Panel>
  );
}

function PluginHubScreen({
  actions,
  hub,
  preview,
}: {
  actions: ReturnType<typeof createActionsShape>;
  hub: PluginHubResult | null;
  preview: PluginInstallPreviewResult | null;
}) {
  const [filter, setFilter] = useState<"all" | "official" | "mcp" | "skill" | "installed" | "review">("all");
  const [selectedId, setSelectedId] = useState("");
  const items = hub?.catalog.items ?? [];
  const visible = items.filter((item) => {
    if (filter === "official") return item.sourceId === "official";
    if (filter === "mcp") return item.installKind === "mcp_server" || item.installKind === "claude_desktop_mcp";
    if (filter === "skill") return item.installKind === "skill_bundle";
    if (filter === "installed") return item.installStatus === "installed";
    if (filter === "review") return item.installStatus === "needsReview";
    return true;
  });
  const selected = items.find((item) => item.id === selectedId) ?? visible[0] ?? null;
  const selectedPreview = preview?.item.id === selected?.id ? preview : null;
  const installButtonLabel = selected?.installKind === "claude_desktop_mcp" || selected?.installKind === "mcp_server"
    ? "安装到 Claude Desktop"
    : "安装";
  return (
    <div className="plugin-layout">
      <Panel title="Claude 插件中心" detail="官方插件、MCP Registry 与 awesome-claude-code 社区资源。">
        <div className="filter-row">
          {[
            ["all", "全部"],
            ["official", "官方插件"],
            ["mcp", "MCP"],
            ["skill", "Skills"],
            ["installed", "已安装"],
            ["review", "需审查"],
          ].map(([id, label]) => (
            <button className={filter === id ? "active" : ""} key={id} onClick={() => setFilter(id as typeof filter)} type="button">
              {label}
            </button>
          ))}
          <Button onClick={() => void actions.refreshPluginHub()} size="sm" variant="outline">
            <RefreshCw className="h-4 w-4" />
            刷新
          </Button>
        </div>
        <div className="source-strip">
          {(hub?.catalog.sources ?? []).map((source) => (
            <div className={`source-pill ${source.status}`} key={source.id}>
              <strong>{source.label}</strong>
              <span>{source.itemCount} 项 · {source.message}</span>
            </div>
          ))}
        </div>
        <div className="plugin-list">
          {visible.length ? visible.slice(0, 220).map((item) => (
            <button className={selected?.id === item.id ? "active" : ""} key={item.id} onClick={() => setSelectedId(item.id)} type="button">
              <div>
                <strong>{item.name}</strong>
                <p>{item.description || item.homepage}</p>
              </div>
              <span className={`status-chip ${item.installStatus}`}>{pluginStatusLabel(item.installStatus)}</span>
            </button>
          )) : <Empty text="暂无目录数据，点击刷新。" />}
        </div>
      </Panel>
      <Panel title={selected?.name ?? "插件详情"} detail={selected ? selected.sourceLabel : "选择条目后查看安装预览。"}>
        {selected ? (
          <div className="detail-stack">
            <p>{selected.description || "暂无描述。"}</p>
            <div className="info-grid compact">
              <InfoRow label="类型" value={pluginKindLabel(selected.installKind)} />
              <InfoRow label="状态" value={pluginStatusLabel(selected.installStatus)} />
              <InfoRow label="分类" value={selected.category || "-"} />
              <InfoRow label="作者" value={selected.author || "-"} />
              <InfoRow label="许可证" value={selected.license || "-"} />
            </div>
            <div className="tag-row">
              {selected.requirements.map((item) => <span key={item}>{item}</span>)}
              {selected.tags.map((item) => <span key={item}>{item}</span>)}
            </div>
            <div className="risk-box">{selected.risk}</div>
            {selectedPreview ? (
              <div className="preview-box">
                <strong>安装预览</strong>
                <span>{selectedPreview.message}</span>
                {selectedPreview.command.length ? <pre>{selectedPreview.command.join(" ")}</pre> : null}
                {selectedPreview.configDiff ? <pre>{selectedPreview.configDiff}</pre> : null}
              </div>
            ) : null}
            <div className="action-row">
              <Button onClick={() => void actions.previewPlugin(selected.id)} variant="outline">
                <ShieldCheck className="h-4 w-4" />
                预览安装
              </Button>
              {selected.installStatus === "installed" ? (
                <Button onClick={() => void actions.uninstallPlugin(selected.id)} variant="outline">
                  <Trash2 className="h-4 w-4" />
                  移除记录
                </Button>
              ) : (
                <Button disabled={selected.installStatus === "unsupported"} onClick={() => void actions.installPlugin(selected.id)}>
                  <Download className="h-4 w-4" />
                  <span className="desktop-install-label">{installButtonLabel}</span>
                </Button>
              )}
              {selected.homepage ? (
                <Button onClick={() => void actions.openExternalUrl(selected.homepage)} variant="outline">
                  <ExternalLink className="h-4 w-4" />
                  来源
                </Button>
              ) : null}
            </div>
          </div>
        ) : <Empty text="还没有选择插件。" />}
      </Panel>
    </div>
  );
}

function PromptOptimizerScreen({ actions }: { actions: ReturnType<typeof createActionsShape> }) {
  return (
    <div className="stack">
      <Panel title="提示词优化器" detail="基于 linshenkx/prompt-optimizer 的独立集成入口。">
        <div className="prompt-optimizer-hero">
          <div>
            <span>Prompt Optimizer</span>
            <strong>打磨系统提示词、用户提示词和可复用模板</strong>
            <p>当前窗口是本地控制入口，在线优化器使用系统浏览器打开，避开远程站点在嵌入式 WebView 中白屏的兼容问题，同时保留清晰的 AGPL 许可边界。</p>
          </div>
          <div className="action-row">
            <Button onClick={() => void actions.openExternalUrl("https://prompt.always200.com")}>
              <PencilRuler className="h-4 w-4" />
              浏览器打开在线版
            </Button>
          </div>
        </div>
      </Panel>
      <div className="ops-columns">
        <Panel title="接入方式" detail="第一版先保证可用：本地控制页不白屏，实际在线版交给系统浏览器。">
          <InfoRow label="在线版" value="https://prompt.always200.com" />
          <InfoRow label="窗口模式" value="内部控制页 + 系统浏览器" />
          <InfoRow label="本地 Web" value="docker run -p 8081:80 linshen/prompt-optimizer" />
          <InfoRow label="MCP 地址" value="http://localhost:8081/mcp" />
          <InfoRow label="源码仓库" value="github.com/linshenkx/prompt-optimizer" />
        </Panel>
        <Panel title="适合场景" detail="把提示词优化放到 Codex/Claude 运维流旁边。">
          <div className="prompt-usecase-list">
            <span>系统提示词优化</span>
            <span>用户提示词改写</span>
            <span>多轮迭代比较</span>
            <span>MCP 方式给 Claude 使用</span>
          </div>
        </Panel>
      </div>
    </div>
  );
}

function ScriptsScreen({ actions, market, settings }: { actions: ReturnType<typeof createActionsShape>; market: ScriptMarketResult | null; settings: SettingsResult | null }) {
  const scripts = market?.market.scripts ?? [];
  const localScripts = settings?.user_scripts.scripts ?? [];
  return (
    <div className="stack">
      <Panel title="脚本市场" detail={`${scripts.length} 个远程脚本，${localScripts.length} 个本地脚本。`}>
        <div className="action-row">
          <Button onClick={() => void actions.refreshScripts()}>
            <RefreshCw className="h-4 w-4" />
            刷新脚本市场
          </Button>
          <Button onClick={() => void actions.openExternalUrl("https://github.com/DamonZS/Claude-Codex-Pro-ToolScriptMarket")} variant="outline">
            <ExternalLink className="h-4 w-4" />
            投稿仓库
          </Button>
        </div>
      </Panel>
      <div className="card-grid">
        {scripts.length ? scripts.map((script) => (
          <div className="market-card" key={script.id}>
            <strong>{script.name}</strong>
            <p>{script.description || script.homepage}</p>
            <div className="tag-row">
              <span>{script.version}</span>
              {script.tags.map((tag) => <span key={tag}>{tag}</span>)}
            </div>
            <div className="action-row">
              <Button disabled={script.installed && !script.updateAvailable} onClick={() => void actions.installMarketScript(script.id)} size="sm">
                {script.installed ? "已安装" : "安装"}
              </Button>
              {script.homepage ? <Button onClick={() => void actions.openExternalUrl(script.homepage)} size="sm" variant="outline">来源</Button> : null}
            </div>
          </div>
        )) : <Empty text="暂无脚本目录数据。" />}
      </div>
    </div>
  );
}

function MaintenanceScreen({
  actions,
  overview,
  settings,
  watcher,
}: {
  actions: ReturnType<typeof createActionsShape>;
  overview: OverviewResult | null;
  settings: SettingsResult | null;
  watcher: WatcherResult | null;
}) {
  return (
    <div className="ops-two-column">
      <div className="ops-wide-column">
        <Panel title="入口维护" detail="修复快捷方式、后端入口和启动状态。">
          <div className="ops-status-list">
            <StatusRow label="Codex 应用" status={overview?.codex_app.status ?? "not_checked"} value={compactPath(overview?.codex_app.path)} />
            <StatusRow label="静默启动入口" status={overview?.silent_shortcut.status ?? "not_checked"} value={compactPath(overview?.silent_shortcut.path)} />
            <StatusRow label="管理工具入口" status={overview?.management_shortcut.status ?? "not_checked"} value={compactPath(overview?.management_shortcut.path)} />
            <StatusRow label="Watcher 自动接管" status={watcher?.enabled ? "running" : "disabled"} value={watcher?.disabled_flag ? compactPath(watcher.disabled_flag) : "未加载"} />
          </div>
          <div className="action-row">
            <Button onClick={() => void actions.repairEntrypoints()}>
              <Wrench className="h-4 w-4" />
              修复入口
            </Button>
            <Button onClick={() => void actions.repairBackend()} variant="outline">
              <ShieldCheck className="h-4 w-4" />
              修复后端
            </Button>
            <Button onClick={() => void actions.refreshRoute("maintenance")} variant="outline">
              <RefreshCw className="h-4 w-4" />
              重新检查
            </Button>
          </div>
        </Panel>
        <Panel title="启动控制" detail="Codex 与 Claude 的启动入口分开，避免误操作。">
          <div className="card-grid">
            <div className="ops-setting-card">
              <Rocket className="h-5 w-5" />
              <strong>Codex</strong>
              <p>启动或重启 Claude Codex Pro 接管的 Codex。</p>
              <div className="action-row">
                <Button onClick={() => void actions.launchCodex()} size="sm">启动 Codex</Button>
                <Button onClick={() => void actions.restartCodex()} size="sm" variant="outline">重启 Codex</Button>
              </div>
            </div>
            <div className="ops-setting-card">
              <MessageCircle className="h-5 w-5" />
              <strong>Claude</strong>
              <p>启动官方 Claude 桌面端，不承诺注入官方 MSIX 窗口。</p>
              <Button onClick={() => void actions.launchClaudeDesktop()} size="sm" variant="outline">启动 Claude</Button>
            </div>
            <div className="ops-setting-card">
              <Languages className="h-5 w-5" />
              <strong>Claude 中文窗口</strong>
              <p>打开包装 WebView，加载 Claude 网页并注入中文覆盖。</p>
              <Button onClick={() => void actions.openClaudeChinese()} size="sm" variant="outline">打开 Claude 中文窗口</Button>
            </div>
          </div>
        </Panel>
        <Panel title="入口安装" detail="安装入口、卸载入口和修复快捷方式都在这里。">
          <div className="action-row">
            <Button onClick={() => void actions.installEntrypoints()}>
              <Download className="h-4 w-4" />
              安装入口
            </Button>
            <Button onClick={() => void actions.repairShortcuts()} variant="outline">
              <Wrench className="h-4 w-4" />
              修复快捷方式
            </Button>
            <Button onClick={() => void actions.uninstallEntrypoints()} variant="outline">
              <Trash2 className="h-4 w-4" />
              卸载入口
            </Button>
          </div>
        </Panel>
      </div>
      <div className="stack">
        <Panel title="Watcher 自动接管" detail="用于保持静默启动入口和接管状态。">
          <div className="ops-status-list">
            <StatusRow label="状态" status={watcher?.enabled ? "running" : "disabled"} value={watcher?.enabled ? "已启用" : "未启用"} />
            <StatusRow label="禁用标记" status={watcher?.disabled_flag ? "found" : "not_checked"} value={watcher?.disabled_flag ? compactPath(watcher.disabled_flag) : "未加载"} />
          </div>
          <div className="action-row">
            <Button onClick={() => void actions.installWatcher()} size="sm" variant="outline">安装 Watcher</Button>
            <Button onClick={() => void actions.enableWatcher()} size="sm" variant="outline">启用</Button>
            <Button onClick={() => void actions.disableWatcher()} size="sm" variant="outline">禁用</Button>
            <Button onClick={() => void actions.uninstallWatcher()} size="sm" variant="outline">移除</Button>
          </div>
        </Panel>
        <Panel title="关键路径" detail="维护排障时优先看这些路径。">
          <div className="info-grid compact">
            <InfoRow label="Codex App" value={compactPath(overview?.codex_app.path)} />
            <InfoRow label="设置文件" value={compactPath(settings?.settings_path)} />
            <InfoRow label="日志文件" value={compactPath(overview?.logs_path)} />
            <InfoRow label="当前版本" value={overview?.current_version ?? "未加载"} />
          </div>
        </Panel>
        <Panel title="最近启动" detail={overview?.latest_launch?.message ?? "暂无启动记录。"}>
          <div className="info-grid compact">
            <InfoRow label="状态" value={overview?.latest_launch?.status ?? "无记录"} />
            <InfoRow label="Debug 端口" value={String(overview?.latest_launch?.debug_port ?? "-")} />
            <InfoRow label="Helper 端口" value={String(overview?.latest_launch?.helper_port ?? "-")} />
            <InfoRow label="Codex App" value={compactPath(overview?.latest_launch?.codex_app)} />
          </div>
        </Panel>
        <Panel title="安全边界" detail="维护操作不会静默改写官方 Claude 安装包。">
          <div className="ops-danger-zone">
            <AlertTriangle className="h-4 w-4" />
            <span>卸载入口只移除本工具创建的入口；修复后端只更新本工具管理的命令包装和配置。</span>
          </div>
        </Panel>
      </div>
    </div>
  );
}

function LogsScreen({ actions, logs }: { actions: ReturnType<typeof createActionsShape>; logs: LogsResult | null }) {
  return (
    <Panel title="运行日志" detail={logs?.path ?? "读取最近 240 行诊断日志。"}>
      <div className="action-row">
        <Button onClick={() => void actions.refreshLogs()}>
          <RefreshCw className="h-4 w-4" />
          刷新日志
        </Button>
      </div>
      <pre className="ops-code tall">{logs?.text || "暂无日志。"}</pre>
    </Panel>
  );
}

function SettingsScreen({
  actions,
  claudeChinese,
  draft,
  onDraftChange,
  overview,
  settings,
  watcher,
}: {
  actions: ReturnType<typeof createActionsShape>;
  claudeChinese: ClaudeChineseWindowResult | null;
  draft: BackendSettings | null;
  onDraftChange: (settings: BackendSettings) => void;
  overview: OverviewResult | null;
  settings: SettingsResult | null;
  watcher: WatcherResult | null;
}) {
  const s = draft ?? settings?.settings ?? null;
  const updateDraft = <K extends keyof BackendSettings>(key: K, value: BackendSettings[K]) => {
    if (!s) return;
    onDraftChange({ ...s, [key]: value });
  };
  const saveDraft = async () => {
    if (!s) return;
    await actions.saveSettings(s);
  };
  const enhancementRows = [
    ["供应商同步", "providerSyncEnabled"],
    ["供应商配置", "relayProfilesEnabled"],
    ["增强总开关", "enhancementsEnabled"],
    ["Computer Use Guard", "computerUseGuardEnabled"],
    ["插件入口解锁", "codexAppPluginEntryUnlock"],
    ["插件市场解锁", "codexAppPluginMarketplaceUnlock"],
    ["特殊插件强制安装", "codexAppForcePluginInstall"],
    ["模型白名单解锁", "codexAppModelWhitelistUnlock"],
    ["会话删除", "codexAppSessionDelete"],
    ["Markdown 导出", "codexAppMarkdownExport"],
    ["会话项目移动", "codexAppProjectMove"],
    ["对话 Timeline", "codexAppConversationTimeline"],
    ["对话阅读视图", "codexAppConversationView"],
    ["切换对话保留位置", "codexAppThreadScrollRestore"],
    ["Zed Remote open", "codexAppZedRemoteOpen"],
    ["Zed 项目记录", "zedRemoteProjectRegistryEnabled"],
    ["同步 Zed settings", "zedRemoteSyncToZedSettings"],
    ["Upstream worktree", "codexAppUpstreamWorktreeCreate"],
    ["原生菜单栏位置", "codexAppNativeMenuPlacement"],
    ["Claude 中文覆盖", "claudeAppChineseOverlayEnabled"],
    ["Fast 按钮", "codexAppServiceTierControls"],
    ["图片覆盖", "codexAppImageOverlayEnabled"],
    ["Codex Goals", "codexGoalsEnabled"],
    ["CLI Wrapper", "cliWrapperEnabled"],
  ] as const;
  return (
    <div className="ops-two-column">
      <div className="ops-wide-column">
        <Panel title="设置文件位置" detail={settings?.settings_path ?? "未读取到设置文件。"}>
          <div className="info-grid compact">
            <InfoRow label="设置文件" value={compactPath(settings?.settings_path)} />
            <InfoRow label="Codex App" value={compactPath(s?.codexAppPath || overview?.codex_app.path)} />
            <InfoRow label="启动模式" value={s?.launchMode ?? "patch"} />
            <InfoRow label="Watcher" value={watcher?.enabled ? "已启用" : "未启用"} />
            <InfoRow label="供应商数量" value={`${s?.relayProfiles.length ?? 0} 个`} />
            <InfoRow label="当前供应商" value={s?.activeRelayId ?? "default"} />
          </div>
          <div className="action-row">
            <Button onClick={() => void actions.refreshRoute("settings")} variant="outline">
              <RefreshCw className="h-4 w-4" />
              刷新设置
            </Button>
            <Button onClick={() => void actions.repairBackend()} variant="outline">
              <ShieldCheck className="h-4 w-4" />
              修复后端
            </Button>
            <Button disabled={!s} onClick={() => void saveDraft()}>
              <CheckCircle2 className="h-4 w-4" />
              保存设置
            </Button>
          </div>
        </Panel>
        <Panel title="Codex 增强矩阵" detail="可直接开关，点击保存设置后写入配置。">
          <div className="ops-setting-grid">
            {enhancementRows.map(([label, key]) => {
              const enabled = Boolean(s?.[key]);
              return (
                <div className={`ops-setting-card ${enabled ? "enabled" : ""}`} key={label}>
                  <strong>{label}</strong>
                  <span>{enabled ? "开启" : "关闭"}</span>
                  <ToggleSwitch checked={enabled} disabled={!s} onChange={(value) => updateDraft(key, value)} />
                </div>
              );
            })}
          </div>
          <div className="action-row">
            <Button disabled={!s} onClick={() => void saveDraft()}>保存增强矩阵</Button>
          </div>
        </Panel>
        <Panel title="Codex 启动参数" detail="每行一个参数，保存后下次启动 Codex 生效。">
          <textarea
            className="ops-textarea"
            disabled={!s}
            onChange={(event) => updateDraft("codexExtraArgs", event.currentTarget.value.split(/\r?\n/).map((line) => line.trim()).filter(Boolean))}
            placeholder="--force_high_performance_gpu"
            value={s?.codexExtraArgs.join("\n") ?? ""}
          />
          <Button disabled={!s} onClick={() => void saveDraft()} variant="outline">保存启动参数</Button>
        </Panel>
      </div>
      <div className="stack">
        <Panel title="Claude 中文包装窗口" detail="中文化目标是包装 WebView，不改官方 MSIX。">
          <div className="info-grid compact">
            <InfoRow label="窗口状态" value={claudeChinese?.open ? "已打开" : "未打开"} />
            <InfoRow label="入口 URL" value={claudeChinese?.defaultUrl ?? "https://claude.ai/new"} />
            <InfoRow label="注入模式" value={claudeChinese?.injectionMode ?? "wrapped_webview"} />
            <InfoRow label="官方 Claude" value={claudeChinese?.officialInstallKind ?? "未检测"} />
            <InfoRow label="CDP 状态" value={claudeChinese?.cdpStatus ?? "未检测"} />
          </div>
          <div className="action-row">
            <Button onClick={() => void actions.openClaudeChinese()}>
              <Languages className="h-4 w-4" />
              打开 Claude 中文窗口
            </Button>
            <Button onClick={() => void actions.launchClaudeDesktop()} variant="outline">
              <MessageCircle className="h-4 w-4" />
              启动 Claude
            </Button>
          </div>
        </Panel>
        <Panel title="CLI Wrapper" detail="命令行包装器用于把本地 Codex CLI 请求接入当前配置。">
          <div className="ops-toggle-line">
            <span>启用 CLI Wrapper</span>
            <ToggleSwitch checked={Boolean(s?.cliWrapperEnabled)} disabled={!s} onChange={(value) => updateDraft("cliWrapperEnabled", value)} />
          </div>
          <label className="ops-form-field">
            <span>Base URL</span>
            <input disabled={!s} onChange={(event) => updateDraft("cliWrapperBaseUrl", event.currentTarget.value)} placeholder="https://api.example.com/v1" value={s?.cliWrapperBaseUrl ?? ""} />
          </label>
          <label className="ops-form-field">
            <span>API Key 环境变量</span>
            <input disabled={!s} onChange={(event) => updateDraft("cliWrapperApiKeyEnv", event.currentTarget.value)} placeholder="OPENAI_API_KEY" value={s?.cliWrapperApiKeyEnv ?? ""} />
          </label>
          <label className="ops-form-field">
            <span>API Key</span>
            <input disabled={!s} onChange={(event) => updateDraft("cliWrapperApiKey", event.currentTarget.value)} placeholder={s?.cliWrapperApiKey ? "已配置，输入新值覆盖" : "未设置"} type="password" value={s?.cliWrapperApiKey ?? ""} />
          </label>
          <Button disabled={!s} onClick={() => void saveDraft()} variant="outline">保存 CLI Wrapper</Button>
        </Panel>
        <Panel title="图片覆盖" detail="应用 Logo / 背景覆盖相关设置，保存后下次注入生效。">
          <div className="ops-toggle-line">
            <span>启用图片覆盖</span>
            <ToggleSwitch checked={Boolean(s?.codexAppImageOverlayEnabled)} disabled={!s} onChange={(value) => updateDraft("codexAppImageOverlayEnabled", value)} />
          </div>
          <label className="ops-form-field">
            <span>图片路径</span>
            <input disabled={!s} onChange={(event) => updateDraft("codexAppImageOverlayPath", event.currentTarget.value)} placeholder="C:\\Users\\Damon\\Pictures\\拓扑.jpg" value={s?.codexAppImageOverlayPath ?? ""} />
          </label>
          <label className="ops-form-field">
            <span>透明度：{s?.codexAppImageOverlayOpacity ?? 0}%</span>
            <input disabled={!s} max={100} min={0} onChange={(event) => updateDraft("codexAppImageOverlayOpacity", Number(event.currentTarget.value))} type="range" value={s?.codexAppImageOverlayOpacity ?? 0} />
          </label>
          <div className="action-row">
            <Button disabled={!s} onClick={() => void saveDraft()} variant="outline">保存图片覆盖</Button>
            <Button onClick={() => void actions.resetImageOverlaySettings()} variant="outline">
              <RefreshCw className="h-4 w-4" />
              重置图片覆盖
            </Button>
          </div>
        </Panel>
        <Panel title="安全边界" detail="这些操作只改本工具配置，不静默改写官方 Claude 包。">
          <div className="ops-danger-zone">
            <ShieldCheck className="h-4 w-4" />
            <span>如需清空配置，可重置设置；第三方接口和 token 不会显示明文。</span>
          </div>
          <Button onClick={() => void actions.resetSettings()} variant="outline">
            <Trash2 className="h-4 w-4" />
            重置设置
          </Button>
        </Panel>
      </div>
    </div>
  );
}

function Panel({ title, detail, children }: { title: string; detail?: string; children: React.ReactNode }) {
  return (
    <section className="ops-panel">
      <header>
        <div>
          <h2>{title}</h2>
          {detail ? <p>{detail}</p> : null}
        </div>
      </header>
      <div className="ops-panel-body">{children}</div>
    </section>
  );
}

function StatusTile({ icon: Icon, label, value, status }: { icon: LucideIcon; label: string; value: string; status: string }) {
  return (
    <div className={`status-tile ${statusOk(status) ? "ok" : "warn"}`}>
      <Icon className="h-4 w-4" />
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function ActionButton({ icon: Icon, label, onClick }: { icon: LucideIcon; label: string; onClick: () => void }) {
  return (
    <button className="action-button" onClick={onClick} type="button">
      <Icon className="h-4 w-4" />
      <span>{label}</span>
    </button>
  );
}

function ToggleSwitch({
  checked,
  disabled,
  onChange,
}: {
  checked: boolean;
  disabled?: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <button
      aria-pressed={checked}
      className={`toggle-switch ${checked ? "checked" : ""}`}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      type="button"
    >
      <span className="toggle-switch-thumb" />
    </button>
  );
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="info-row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function StatusRow({ label, value, status }: { label: string; value: string; status: string }) {
  return (
    <div className={`ops-status-row ${statusOk(status) ? "ok" : "warn"}`}>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function Empty({ text }: { text: string }) {
  return <div className="empty-state">{text}</div>;
}

function Notice({ notice, onClose }: { notice: { title: string; message: string; status?: Status }; onClose: () => void }) {
  const ok = statusOk(notice.status);
  return (
    <div className="toast-wrap" role="status" aria-live={ok ? "polite" : "assertive"}>
      <div className={ok ? "toast-card" : "toast-card failed"}>
        <div className="toast-progress" />
        <div className="toast-icon">{ok ? <CheckCircle2 className="h-5 w-5" /> : <AlertTriangle className="h-5 w-5" />}</div>
        <div className="toast-body">
          <h2>{notice.title}</h2>
          <p>{notice.message}</p>
        </div>
        <button className="toast-close" onClick={onClose} type="button" aria-label="关闭提示">
          <X className="h-4 w-4" />
        </button>
      </div>
    </div>
  );
}

function pluginKindLabel(kind: PluginInstallKind) {
  if (kind === "claude_desktop_mcp") return "Claude Desktop MCP";
  if (kind === "claude_plugin_marketplace") return "Claude Code 插件";
  const labels: Partial<Record<PluginInstallKind, string>> = {
    claude_plugin_marketplace: "Claude 插件",
    mcp_server: "MCP 服务器",
    skill_bundle: "Skill Bundle",
    resource_link: "资源链接",
  };
  return labels[kind] ?? kind;
}

function pluginStatusLabel(status: PluginInstallStatus) {
  const labels: Record<PluginInstallStatus, string> = {
    notInstalled: "未安装",
    installed: "已安装",
    needsReview: "需审查",
    unsupported: "仅浏览",
  };
  return labels[status] ?? status;
}

function routeLabel(route: Route) {
  return routes.find((item) => item.id === route)?.label ?? "概览";
}

function initialRoute(): Route {
  const injectedRoute = window.__CLAUDE_CODEX_PRO_INITIAL_ROUTE;
  if (routes.some((item) => item.id === injectedRoute)) return injectedRoute as Route;
  try {
    const view = new URLSearchParams(window.location.search).get("view");
    if (routes.some((item) => item.id === view)) return view as Route;
  } catch {
    // Fall back to overview when running outside a normal browser URL.
  }
  return "overview";
}

function routeSubtitle(route: Route) {
  const subtitles: Record<Route, string> = {
    overview: "运行状态、启动动作和 Claude 中文窗口诊断。",
    relay: "供应商与模型接入摘要。",
    context: "MCP、Skills、Plugins 的配置入口。",
    pluginHub: "开源插件、MCP 与技能目录。",
    promptOptimizer: "提示词优化、测试和 MCP 接入。",
    scripts: "Codex 前端用户脚本市场。",
    maintenance: "快捷方式、后端入口和启动修复。",
    logs: "诊断日志与运行信息。",
    settings: "全局开关和配置摘要。",
  };
  return subtitles[route];
}

function routeDocumentTitle(route: Route) {
  if (window.__CLAUDE_CODEX_PRO_INITIAL_ROUTE === "promptOptimizer") return "提示词优化器";
  if (window.__CLAUDE_CODEX_PRO_INITIAL_ROUTE === "pluginHub") return "Claude 插件中心";
  return route === "overview" ? "Claude Codex Pro 管理工具" : `${routeLabel(route)} - Claude Codex Pro 管理工具`;
}

function stringifyError(error: unknown) {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return JSON.stringify(error);
}

function createActionsShape() {
  return {
    refreshRoute: async (_route?: Route) => {},
    openClaudeChinese: async () => {},
    launchClaudeDesktop: async () => {},
    launchCodex: async () => {},
    restartCodex: async () => {},
    openExternalUrl: async (_url: string) => {},
    goPluginHub: async () => {},
    goPromptOptimizer: async () => {},
    previewPlugin: async (_id: string) => null as PluginInstallPreviewResult | null,
    installPlugin: async (_id: string) => {},
    uninstallPlugin: async (_id: string) => {},
    installMarketScript: async (_id: string) => {},
    refreshPluginHub: async () => null as PluginHubResult | null,
    refreshScripts: async () => null as ScriptMarketResult | null,
    repairEntrypoints: async () => {},
    repairBackend: async () => {},
    applyRelayMode: async () => {},
    applyPureApiMode: async () => {},
    clearRelayMode: async () => {},
    saveSettings: async (_settings: BackendSettings) => null as SettingsResult | null,
    installEntrypoints: async () => {},
    uninstallEntrypoints: async () => {},
    repairShortcuts: async () => {},
    installWatcher: async () => {},
    uninstallWatcher: async () => {},
    enableWatcher: async () => {},
    disableWatcher: async () => {},
    resetSettings: async () => {},
    resetImageOverlaySettings: async () => {},
    refreshLogs: async () => null as LogsResult | null,
    refreshWatcher: async () => null as WatcherResult | null,
    setTheme: (_theme: Theme | ((theme: Theme) => Theme)) => {},
  };
}
