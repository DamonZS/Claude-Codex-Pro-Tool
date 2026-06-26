import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  Download,
  FileDown,
  FileUp,
  ExternalLink,
  FileCode2,
  Info,
  KeyRound,
  Languages,
  LayoutDashboard,
  MessageCircle,
  MessageSquare,
  Network,
  PackageSearch,
  PencilRuler,
  Power,
  RefreshCw,
  Rocket,
  Settings,
  ShieldCheck,
  Copy,
  Plus,
  Save,
  Trash2,
  Wrench,
  X,
  type LucideIcon,
} from "lucide-react";
import { type Dispatch, type SetStateAction, useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import { open } from "@tauri-apps/plugin-dialog";
import { invokeCommand } from "@/tauriBridge";

const PONYTAIL_REPOSITORY_URL = "https://github.com/DietrichGebert/ponytail";

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

type ClaudeZhPatchStatus = {
  status: string;
  message: string;
  installRoot: string | null;
  appRoot: string | null;
  installKind: string;
  localeConfigPath: string;
  backupDir: string;
  resourcesPresent: boolean;
  frontendI18nPresent: boolean;
  statsigI18nPresent: boolean;
  chunkPatchPresent: boolean;
  languageWhitelistPatched: boolean;
  localeConfigured: boolean;
  writable: boolean;
};

type ClaudeZhPatchResult = CommandResult<{
  status: ClaudeZhPatchStatus;
  changedFiles: string[];
  backupDir: string;
  logsPath: string;
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
  memoryAssistEnabled: boolean;
  memoryAssistInjectEnabled: boolean;
  memoryAssistAutoSuggestEnabled: boolean;
  memoryAssistMaxInjectedItems: number;
  memoryAssistWorkspaceMode: string;
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

type SupplierSaveResult = {
  settings: BackendSettings;
  profile: RelayProfile;
};

type SettingsResult = CommandResult<{
  settings: BackendSettings;
  settings_path: string;
  user_scripts: UserScriptInventory;
}>;

type RelayProfileModelsResult = CommandResult<{
  models: string[];
  endpoint: string;
}>;

type SupplierPreset = {
  id: string;
  name: string;
  category: "official" | "cn_official" | "aggregator" | "third_party";
  baseUrl: string;
  protocol: "responses" | "chatCompletions";
  model: string;
  modelList?: string[];
  websiteUrl?: string;
  apiKeyUrl?: string;
};

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

type CodexPluginMarketplaceStatus = {
  codexHome: string;
  marketplaceRoot: string | null;
  configRegistered: boolean;
  needsRepair: boolean;
  message: string;
};

type CodexPluginMarketplaceStatusResult = CommandResult<{
  marketplace: CodexPluginMarketplaceStatus;
}>;

type CodexPluginMarketplaceRepairResult = CommandResult<{
  repair: {
    codexHome: string;
    marketplaceRoot: string | null;
    initialized: boolean;
    configured: boolean;
    configRegistered: boolean;
    needsRepair: boolean;
    message: string;
  };
  marketplace: CodexPluginMarketplaceStatus;
}>;

type LocalSession = {
  id: string;
  title: string;
  cwd: string;
  modelProvider: string;
  archived: boolean;
  updatedAtMs: number | null;
  rolloutPath: string;
  dbPath: string;
};

type LocalSessionsResult = CommandResult<{
  dbPath: string;
  dbPaths: string[];
  sessions: LocalSession[];
}>;

type MemoryItem = {
  id: string;
  text: string;
  workspace: string;
  category: string;
  tags: string[];
  source: string;
  sourceSessionId: string;
  createdAt: number;
  updatedAt: number;
  lastAccessedAt: number;
  accessCount: number;
};

type MemoryCandidate = {
  id: string;
  text: string;
  workspace: string;
  category: string;
  tags: string[];
  source: string;
  reason: string;
  sourceSessionId: string;
  status: string;
  createdAt: number;
  updatedAt: number;
};

type MemoryStatusResult = CommandResult<{
  memory: {
    status: string;
    dbPath: string;
    totalItems: number;
    pendingCandidates: number;
    workspaces: Array<{ workspace: string; itemCount: number; pendingCount: number }>;
    latestBackupPath: string | null;
  };
}>;

type MemoryItemsResult = CommandResult<{ items: MemoryItem[] }>;
type MemoryCandidatesResult = CommandResult<{ candidates: MemoryCandidate[] }>;
type MemoryItemResult = CommandResult<{ item: MemoryItem }>;
type MemoryCandidateResult = CommandResult<{ candidate: MemoryCandidate }>;
type MemoryQueryResult = CommandResult<{
  memory: {
    query: string;
    workspace: string;
    results: Array<{
      item: MemoryItem;
      score: number;
      matchedKeywords: string[];
    }>;
  };
}>;
type MemoryExport = {
  schemaVersion: string;
  exportedAt: number;
  items: MemoryItem[];
  candidates: MemoryCandidate[];
};
type MemoryExportResult = CommandResult<{ data: MemoryExport }>;
type MemorySelfCheckResult = CommandResult<{
  report: {
    status: string;
    repaired: boolean;
    backupPath: string | null;
    checks: Array<{ name: string; status: string; message: string }>;
  };
}>;

type ProviderSyncResult = CommandResult<{
  syncStatus?: string;
  targetProvider?: string;
  changedSessionFiles?: number;
  skippedLockedRolloutFiles?: string[];
  sqliteRowsUpdated?: number;
  sqliteProviderRowsUpdated?: number;
  sqliteUserEventRowsUpdated?: number;
  sqliteCwdRowsUpdated?: number;
  updatedWorkspaceRoots?: string[];
  encryptedContentWarning?: string;
  backupDir?: string;
  syncMessage?: string;
}>;

type DeleteLocalSessionResult = CommandResult<{
  session_id?: string;
  sessionId?: string;
  undo_token?: string | null;
  undoToken?: string | null;
  backup_path?: string | null;
  backupPath?: string | null;
}>;

type PluginInstallKind =
  | "claude_plugin_marketplace"
  | "claude_desktop_mcp"
  | "claude_desktop_org_plugin"
  | "claude_code_plugin"
  | "codex_plugin"
  | "copilot_plugin"
  | "managed_skill_bundle"
  | "mcp_server"
  | "skill_bundle"
  | "resource_link";
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

type PluginInstallOutcomePayload = Omit<PluginInstallOutcomeResult, "status" | "message">;

type CodexHookTrustResult = CommandResult<{
  preview: {
    configPath: string;
    hooks: Array<{
      key: string;
      eventName: string;
      matcher: string | null;
      command: string;
      statusMessage: string | null;
      currentHash: string;
      trusted: boolean;
      sourcePath: string;
    }>;
    message: string;
  };
}>;

type McpbPackageResult = CommandResult<{
  package: {
    mcpbPath: string;
    manifestPath: string;
    opened: boolean;
    message: string;
  };
}>;

type ClaudeDesktopOrgPluginStatusResult = CommandResult<{
  orgPluginStatus: {
    supported: boolean;
    orgPluginsDir: string;
    configLibraryDir: string;
    profileMetaPath: string;
    ponytailPluginDir: string;
    ponytailInstalled: boolean;
    writable: boolean;
    message: string;
  };
}>;

type ClaudeDesktopOrgPluginInstallResult = CommandResult<{
  outcome: {
    installed: boolean;
    orgPluginsDir: string;
    pluginDir: string;
    manifestPath: string;
    pluginJsonPath: string;
    copiedSkills: string[];
    backupPath: string | null;
    message: string;
  };
  orgPluginStatus: ClaudeDesktopOrgPluginStatusResult["orgPluginStatus"];
}>;

type ClaudeDesktopMarketplaceStatusResult = CommandResult<{
  marketplaceStatus: {
    supported: boolean;
    marketplace: string;
    plugin: string;
    deepLink: string;
    canAutoWrite: boolean;
    message: string;
  };
}>;

type ClaudeDesktopMarketplaceOpenResult = CommandResult<{
  outcome: {
    opened: boolean;
    deepLink: string;
    message: string;
  };
  marketplaceStatus: ClaudeDesktopMarketplaceStatusResult["marketplaceStatus"];
}>;

type ClaudeDesktopDevModeStatusResult = CommandResult<{
  devModeStatus: {
    supported: boolean;
    configured: boolean;
    normalConfigPath: string;
    threepConfigPath: string;
    configLibraryDir: string;
    profileMetaPath: string;
    appliedId: string | null;
    message: string;
  };
}>;

type ClaudeDesktopDevModeConfigureResult = CommandResult<{
  outcome: {
    configured: boolean;
    normalConfigPath: string;
    threepConfigPath: string;
    profileMetaPath: string;
    backupPaths: string[];
    message: string;
  };
  devModeStatus: ClaudeDesktopDevModeStatusResult["devModeStatus"];
}>;

type ClaudeDesktopProviderPreviewResult = CommandResult<{
  preview: {
    profileId: string;
    profileName: string;
    normalConfigPath: string;
    threepConfigPath: string;
    profilePath: string;
    metaPath: string;
    writeTargets: string[];
    configDiff: string;
    redactedProfile: string;
  };
}>;

type ClaudeDesktopProviderApplyResult = CommandResult<{
  outcome: {
    configured: boolean;
    normalConfigPath: string;
    threepConfigPath: string;
    profilePath: string;
    metaPath: string;
    backupPaths: string[];
    message: string;
  };
  devModeStatus: ClaudeDesktopDevModeStatusResult["devModeStatus"];
}>;

type ClaudeDesktopLocalBundleResult = CommandResult<{
  outcome: {
    devMode: ClaudeDesktopDevModeConfigureResult["outcome"];
    codexMcp: PluginInstallOutcomePayload;
    ponytailMcp: PluginInstallOutcomePayload;
    organizationPlugin: ClaudeDesktopOrgPluginInstallResult["outcome"];
    message: string;
  };
  devModeStatus: ClaudeDesktopDevModeStatusResult["devModeStatus"];
  orgPluginStatus: ClaudeDesktopOrgPluginStatusResult["orgPluginStatus"];
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
  | "supplier"
  | "tools"
  | "sessions"
  | "scripts"
  | "logs"
  | "settings";
type LegacyRoute = "promptOptimizer" | "relay";
const MEMORY_ALL_WORKSPACES = "__all__";
const MEMORY_GLOBAL_WORKSPACE = "global";
const PROMPT_OPTIMIZER_URL = "https://prompt.always200.com";

declare global {
  interface Window {
    __CLAUDE_CODEX_PRO_INITIAL_ROUTE?: Route | LegacyRoute;
  }
}

const routes: Array<{ id: Route; label: string; icon: LucideIcon }> = [
  { id: "overview", label: "概览", icon: LayoutDashboard },
  { id: "supplier", label: "供应商", icon: Network },
  { id: "tools", label: "工具与插件", icon: PackageSearch },
  { id: "sessions", label: "会话管理", icon: MessageSquare },
  { id: "scripts", label: "脚本", icon: FileCode2 },
  { id: "logs", label: "日志", icon: Info },
  { id: "settings", label: "设置", icon: Settings },
];

function isRoute(value: unknown): value is Route {
  return routes.some((item) => item.id === value);
}

function statusOk(status?: string | null) {
  return status === "ok" || status === "accepted" || status === "found" || status === "installed" || status === "running";
}

function statusFailed(status?: string | null) {
  return status === "failed" || status === "not_implemented";
}

function compactPath(path?: string | null) {
  if (!path) return "未设置";
  if (path.length <= 58) return path;
  return `${path.slice(0, 24)}...${path.slice(-28)}`;
}

function zhPatchNoticeMessage(result: ClaudeZhPatchResult) {
  const status = result.status;
  const patchStatus = result.status.status;
  const installKind = result.status.installKind || "unknown";
  const writable = result.status.writable ? "可写" : "不可写/需管理员授权";
  if (!statusFailed(status)) {
    return `${result.message}\n资源状态：${patchStatus} / ${installKind} / ${writable}\n已自动启动/重启 Claude Desktop，请验证界面语言。`;
  }
  const logPath = result.logsPath || "~\\.claude-codex-pro\\claude-codex-pro.log";
  return `${result.message}\n资源状态：${patchStatus} / ${installKind} / ${writable}\n诊断日志：${logPath}`;
}

export function App() {
  const [route, setRoute] = useState<Route>(() => initialRoute());
  const [notice, setNotice] = useState<{ title: string; message: string; status?: Status } | null>(null);
  const [busyCount, setBusyCount] = useState(0);
  const busy = busyCount > 0;
  const [overview, setOverview] = useState<OverviewResult | null>(null);
  const [claudeDesktop, setClaudeDesktop] = useState<ClaudeDesktopResult | null>(null);
  const [claudeChinese, setClaudeChinese] = useState<ClaudeChineseWindowResult | null>(null);
  const [claudeZhPatch, setClaudeZhPatch] = useState<ClaudeZhPatchResult | null>(null);
  const [settings, setSettings] = useState<SettingsResult | null>(null);
  const [settingsDraft, setSettingsDraft] = useState<BackendSettings | null>(null);
  const [pluginHub, setPluginHub] = useState<PluginHubResult | null>(null);
  const [pluginPreview, setPluginPreview] = useState<PluginInstallPreviewResult | null>(null);
  const [claudeDesktopProviderPreview, setClaudeDesktopProviderPreview] = useState<ClaudeDesktopProviderPreviewResult | null>(null);
  const [claudeDesktopProviderApply, setClaudeDesktopProviderApply] = useState<ClaudeDesktopProviderApplyResult | null>(null);
  const [claudeDesktopProviderDraft, setClaudeDesktopProviderDraft] = useState({
    name: "拓扑熵减API",
    baseUrl: "https://api.toporeduce.cn",
    apiKey: "",
    modelList: "claude-sonnet-4-6\nclaude-opus-4-8 [1m]\nclaude-haiku-4-5",
  });
  const [codexHookTrust, setCodexHookTrust] = useState<CodexHookTrustResult | null>(null);
  const [codexPluginMarketplace, setCodexPluginMarketplace] = useState<CodexPluginMarketplaceStatusResult | null>(null);
  const [claudeDesktopOrgPlugin, setClaudeDesktopOrgPlugin] = useState<ClaudeDesktopOrgPluginStatusResult | null>(null);
  const [claudeDesktopMarketplace, setClaudeDesktopMarketplace] = useState<ClaudeDesktopMarketplaceStatusResult | null>(null);
  const [claudeDesktopDevMode, setClaudeDesktopDevMode] = useState<ClaudeDesktopDevModeStatusResult | null>(null);
  const [claudeDevModeBusy, setClaudeDevModeBusy] = useState(false);
  const [scriptMarket, setScriptMarket] = useState<ScriptMarketResult | null>(null);
  const [localSessions, setLocalSessions] = useState<LocalSessionsResult | null>(null);
  const [memoryAssist, setMemoryAssist] = useState<MemoryStatusResult | null>(null);
  const [memoryItems, setMemoryItems] = useState<MemoryItemsResult | null>(null);
  const [memoryCandidates, setMemoryCandidates] = useState<MemoryCandidatesResult | null>(null);
  const [memorySelfCheck, setMemorySelfCheck] = useState<MemorySelfCheckResult | null>(null);
  const [memorySearch, setMemorySearch] = useState<MemoryQueryResult | null>(null);
  const [memoryExport, setMemoryExport] = useState<MemoryExportResult | null>(null);
  const [providerSync, setProviderSync] = useState<ProviderSyncResult | null>(null);
  const [logs, setLogs] = useState<LogsResult | null>(null);
  const [watcher, setWatcher] = useState<WatcherResult | null>(null);

  const call = <T,>(command: string, args?: Record<string, unknown>) => invokeCommand<T>(command, args);
  const notifyIfNeedsAttention = (next: { title: string; message: string; status?: Status }) => {
    if (!statusOk(next.status)) setNotice(next);
  };

  const run = async <T,>(
    task: () => Promise<T>,
    title?: string,
    options: { trackBusy?: boolean; notify?: boolean } = {},
  ): Promise<T | null> => {
    const trackBusy = options.trackBusy !== false;
    const notify = options.notify !== false;
    if (trackBusy) setBusyCount((count) => count + 1);
    try {
      return await task();
    } catch (error) {
      if (notify) setNotice({ title: title || "调用失败", message: stringifyError(error), status: "failed" });
      return null;
    } finally {
      if (trackBusy) setBusyCount((count) => Math.max(0, count - 1));
    }
  };

  useEffect(() => {
    if (!notice) return;
    if (notice.status === "running") return;
    const timeout = window.setTimeout(() => setNotice(null), 5200);
    return () => window.clearTimeout(timeout);
  }, [notice]);

  const refreshOverview = async (silent = false) => {
    const result = await run(() => call<OverviewResult>("load_overview"), "概览", { trackBusy: !silent, notify: !silent });
    if (result) {
      setOverview(result);
      if (!silent) notifyIfNeedsAttention({ title: "概览", message: result.message, status: result.status });
    }
    return result;
  };

  const refreshClaude = async (silent = false) => {
    const [desktop, wrapped, zhPatch] = await Promise.all([
      run(() => call<ClaudeDesktopResult>("load_claude_desktop_status"), "Claude Desktop", { trackBusy: !silent, notify: !silent }),
      run(() => call<ClaudeChineseWindowResult>("load_claude_chinese_window_status"), "Claude 一键汉化", { trackBusy: !silent, notify: !silent }),
      run(() => call<ClaudeZhPatchResult>("load_claude_zh_patch_status"), "Claude 本机汉化", { trackBusy: !silent, notify: !silent }),
    ]);
    if (desktop) setClaudeDesktop(desktop);
    if (wrapped) setClaudeChinese(wrapped);
    if (zhPatch) setClaudeZhPatch(zhPatch);
    if (!silent && desktop) notifyIfNeedsAttention({ title: "Claude Desktop", message: desktop.message, status: desktop.status });
  };

  const refreshSettings = async (silent = false) => {
    const result = await run(() => call<SettingsResult>("load_settings"), "设置", { trackBusy: !silent, notify: !silent });
    if (result) {
      setSettings(result);
      setSettingsDraft(result.settings);
      if (!silent) notifyIfNeedsAttention({ title: "设置", message: result.message, status: result.status });
    }
    return result;
  };

  const refreshPluginHub = async (silent = false) => {
    const result = await run(() => call<PluginHubResult>("refresh_plugin_hub_catalog"), "插件中心", { trackBusy: !silent, notify: !silent });
    if (result) {
      setPluginHub(result);
      if (!silent) notifyIfNeedsAttention({ title: "插件中心", message: result.message, status: result.status });
    }
    return result;
  };

  const refreshClaudeDesktopOrgPlugin = async (silent = false) => {
    const result = await run(() => call<ClaudeDesktopOrgPluginStatusResult>("load_claude_desktop_org_plugin_status"), "Claude Desktop 组织插件", { trackBusy: !silent, notify: !silent });
    if (result) {
      setClaudeDesktopOrgPlugin(result);
      if (!silent) notifyIfNeedsAttention({ title: "Claude Desktop 组织插件", message: result.message, status: result.status });
    }
    return result;
  };

  const refreshClaudeDesktopMarketplace = async (silent = false) => {
    const result = await run(() => call<ClaudeDesktopMarketplaceStatusResult>("load_claude_desktop_marketplace_status"), "Claude Desktop 插件仓库", { trackBusy: !silent, notify: !silent });
    if (result) {
      setClaudeDesktopMarketplace(result);
      if (!silent) notifyIfNeedsAttention({ title: "Claude Desktop 插件仓库", message: result.message, status: result.status });
    }
    return result;
  };

  const refreshClaudeDesktopDevMode = async (silent = false) => {
    const result = await run(() => call<ClaudeDesktopDevModeStatusResult>("load_claude_desktop_dev_mode_status"), "Claude Desktop 开发模式", { trackBusy: !silent, notify: !silent });
    if (result) {
      setClaudeDesktopDevMode(result);
      if (!silent) notifyIfNeedsAttention({ title: "Claude Desktop 开发模式", message: result.message, status: result.status });
    }
    return result;
  };

  const refreshScripts = async (silent = false) => {
    const result = await run(() => call<ScriptMarketResult>("refresh_script_market"), "脚本市场", { trackBusy: !silent, notify: !silent });
    if (result) {
      setScriptMarket(result);
      if (!silent) notifyIfNeedsAttention({ title: "脚本市场", message: result.message, status: result.status });
    }
    return result;
  };

  const refreshCodexPluginMarketplace = async (silent = false) => {
    const result = await run(() => call<CodexPluginMarketplaceStatusResult>("load_codex_plugin_marketplace_status"), "Codex OpenAI 插件仓库", { trackBusy: !silent, notify: !silent });
    if (result) {
      setCodexPluginMarketplace(result);
      if (!silent) notifyIfNeedsAttention({ title: "Codex OpenAI 插件仓库", message: result.message, status: result.status });
    }
    return result;
  };

  const refreshLocalSessions = async (silent = false) => {
    const result = await run(() => call<LocalSessionsResult>("list_local_sessions"), "Codex 会话管理", { trackBusy: !silent, notify: !silent });
    if (result) {
      setLocalSessions(result);
      if (!silent) notifyIfNeedsAttention({ title: "Codex 会话管理", message: result.message, status: result.status });
    }
    return result;
  };

  const refreshMemoryAssistStatus = async (silent = false) => {
    const status = await run(() => call<MemoryStatusResult>("load_memory_assist_status"), "盘古记忆", { trackBusy: !silent, notify: !silent });
    if (status) {
      setMemoryAssist(status);
      if (!silent) notifyIfNeedsAttention({ title: "盘古记忆", message: status.message, status: status.status });
    }
    return status;
  };

  const refreshMemoryAssist = async (silent = false) => {
    const [status, items, candidates] = await Promise.all([
      refreshMemoryAssistStatus(silent),
      run(() => call<MemoryItemsResult>("list_memory_assist_items", { request: { workspace: MEMORY_ALL_WORKSPACES, includeGlobal: true, limit: 80 } }), "记忆列表", { trackBusy: !silent, notify: !silent }),
      run(() => call<MemoryCandidatesResult>("list_memory_assist_candidates", { request: { workspace: MEMORY_ALL_WORKSPACES, includeGlobal: true } }), "待确认记忆", { trackBusy: !silent, notify: !silent }),
    ]);
    if (items) setMemoryItems(items);
    if (candidates) setMemoryCandidates(candidates);
    return status;
  };

  const refreshLogs = async (silent = false) => {
    const result = await run(() => call<LogsResult>("read_latest_logs", { request: { lines: 240 } }), "日志", { trackBusy: !silent, notify: !silent });
    if (result) {
      setLogs(result);
      if (!silent) notifyIfNeedsAttention({ title: "日志", message: result.message, status: result.status });
    }
    return result;
  };

  const refreshWatcher = async (silent = false) => {
    const result = await run(() => call<WatcherResult>("load_watcher_state"), "Watcher", { trackBusy: !silent, notify: !silent });
    if (result) {
      setWatcher(result);
      if (!silent) notifyIfNeedsAttention({ title: "Watcher", message: result.message, status: result.status });
    }
    return result;
  };

  const writeUiEvent = async (event: string, detail: Record<string, unknown> = {}) => {
    try {
      await call<CommandResult<Record<string, unknown>>>("write_diagnostic_event", { event, detail });
    } catch {
      // Diagnostic logging must never block the user action it is observing.
    }
  };

  const installClaudeZhPatch = async () => {
    setNotice({
      title: "Claude 一键汉化",
      message: "正在请求管理员授权并写入 Claude 本机汉化资源，请在弹出的 UAC 授权框中选择允许。",
      status: "running",
    });
    await waitForPaint();
    void writeUiEvent("claude_zh_patch.install.click");
    const autoResult = await run(() => call<ClaudeZhPatchResult>("install_claude_zh_patch"), "Claude 一键汉化");
    if (autoResult) {
      setClaudeZhPatch(autoResult);
      setNotice({ title: "Claude 一键汉化", message: zhPatchNoticeMessage(autoResult), status: autoResult.status });
      await refreshClaude(true);
    }
  };

  const installClaudeZhPatchFromDirectory = async () => {
    const selected = await open({ directory: true, multiple: false, title: "选择可写的 Claude Desktop 安装目录" });
    const installRoot = Array.isArray(selected) ? selected[0] : selected;
    if (!installRoot) return;
    setNotice({
      title: "Claude 手动汉化",
      message: "正在写入所选 Claude 安装目录，需要时会弹出管理员授权...",
      status: "running",
    });
    await waitForPaint();
    void writeUiEvent("claude_zh_patch.manual_install.click");

    const result = await run(
      () => call<ClaudeZhPatchResult>("install_claude_zh_patch_at_install_root", { installRoot }),
      "Claude 手动汉化"
    );
    if (result) {
      setClaudeZhPatch(result);
      setNotice({ title: "Claude 手动汉化", message: zhPatchNoticeMessage(result), status: result.status });
      await refreshClaude(true);
    }
  };

  const openClaudeChinese = async () => {
    const result = await run(() => call<ClaudeChineseWindowResult>("open_claude_chinese_window"), "Claude 一键汉化");
    if (result) {
      setClaudeChinese(result);
      notifyIfNeedsAttention({ title: "Claude 一键汉化", message: result.message, status: result.status });
      await refreshClaude(true);
    }
  };

  const restoreClaudeZhPatch = async () => {
    if (!window.confirm("确认恢复 Claude 官方文件？这会用汉化前的备份覆盖已修改文件。")) return;
    setNotice({
      title: "恢复 Claude 官方文件",
      message: "正在恢复 Claude 官方文件，需要时会弹出管理员授权...",
      status: "running",
    });
    await waitForPaint();
    void writeUiEvent("claude_zh_patch.restore.click");
    const result = await run(() => call<ClaudeZhPatchResult>("restore_claude_zh_patch"), "恢复 Claude 官方文件");
    if (result) {
      setClaudeZhPatch(result);
      setNotice({ title: "恢复 Claude 官方文件", message: result.message, status: result.status });
      await refreshClaude(true);
    }
  };

  const launchClaudeDesktop = async () => {
    const result = await run(() => call<CommandResult<Record<string, unknown>>>("open_claude_desktop"), "启动/重启Claude");
    if (result) {
      notifyIfNeedsAttention({ title: "启动/重启Claude", message: result.message, status: result.status });
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
    const result = await run(() => call<CommandResult<Record<string, unknown>>>("launch_claude_codex_pro", { request: {} }), "启动/重启Codex");
    if (result) {
      notifyIfNeedsAttention({ title: "启动/重启Codex", message: result.message, status: result.status });
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
    if (!preview.canInstall) {
      setNotice({ title: "插件中心", message: preview.message, status: "needs_review" });
      return;
    }
    const details = [
      preview.command?.length ? `命令：${preview.command.join(" ")}` : "",
      preview.configDiff ? `配置：\n${preview.configDiff}` : "",
      preview.message,
    ].filter(Boolean).join("\n\n");
    if (!window.confirm(`确认安装？\n\n${details}`)) return;
    const result = await run(() => call<PluginInstallOutcomeResult>("install_plugin_hub_item", { request: { id } }), "安装插件");
    if (result) {
      const defaultFailure = preview.item.installKind === "claude_plugin_marketplace" || preview.item.installKind === "claude_code_plugin"
        ? "插件安装失败，请检查 Claude CLI 状态和安装预览。"
        : "插件安装失败，请检查安装预览、文件权限和本地依赖。";
      const message = result.message || result.installMessage || result.stderr || result.stdout || defaultFailure;
      notifyIfNeedsAttention({ title: "插件中心", message, status: result.status });
      await refreshPluginHub(true);
    }
  };

  const uninstallPlugin = async (id: string) => {
    if (!window.confirm("卸载该条目？会撤销本工具写入的 Claude Desktop MCP 配置和托管 Skills；外部 CLI 插件只移除安装记录。")) return;
    const result = await run(() => call<PluginHubResult>("uninstall_plugin_hub_item", { request: { id } }), "卸载插件");
    if (result) {
      setPluginHub(result);
      notifyIfNeedsAttention({ title: "插件中心", message: result.message, status: result.status });
    }
  };

  const previewPonytailCodexHooks = async () => {
    const result = await run(() => call<CodexHookTrustResult>("preview_ponytail_codex_hooks"), "Ponytail Codex Hooks");
    if (result) {
      setCodexHookTrust(result);
      notifyIfNeedsAttention({ title: "Ponytail Codex Hooks", message: result.message, status: result.status });
    }
    return result;
  };

  const trustPonytailCodexHooks = async () => {
    const preview = codexHookTrust ?? await previewPonytailCodexHooks();
    if (!preview) return;
    const pending = preview.preview.hooks.filter((hook) => !hook.trusted);
    if (!pending.length) {
      setNotice({ title: "Ponytail Codex Hooks", message: "No untrusted Ponytail hooks were found.", status: "ok" });
      return;
    }
    const details = pending.map((hook) => `${hook.eventName}: ${hook.command}`).join("\n\n");
    if (!window.confirm(`Trust these Ponytail Codex hooks?\n\n${details}`)) return;
    const result = await run(() => call<CodexHookTrustResult>("trust_ponytail_codex_hooks"), "Trust Ponytail Hooks");
    if (result) {
      setCodexHookTrust(result);
      notifyIfNeedsAttention({ title: "Ponytail Codex Hooks", message: result.message, status: result.status });
    }
  };

  const generatePonytailMcpbInstaller = async () => {
    const result = await run(() => call<McpbPackageResult>("generate_ponytail_mcpb_installer"), "Ponytail MCPB");
    if (result) {
      notifyIfNeedsAttention({ title: "Ponytail MCPB", message: result.message || result.package.message, status: result.status });
    }
  };

  const installPonytailClaudeDesktopOrgPlugin = async () => {
    await installPlugin("ponytail:claude-desktop-org-plugin");
    await refreshClaudeDesktopOrgPlugin(true);
  };

  const installPonytailClaudeDesktopLocalBundle = async () => {
    if (!window.confirm("确认写入 Claude Desktop 本地开发模式插件包？将配置开发模式、写入 Codex/Ponytail MCP，并复制 Ponytail skills 到组织插件目录；不会调用 Claude CLI 登录。")) return;
    const result = await run(() => call<ClaudeDesktopLocalBundleResult>("install_ponytail_claude_desktop_local_bundle"), "Claude Desktop 本地插件包");
    if (result) {
      setClaudeDesktopDevMode({
        status: result.status,
        message: result.message,
        devModeStatus: result.devModeStatus,
      });
      setClaudeDesktopOrgPlugin({
        status: result.status,
        message: result.message,
        orgPluginStatus: result.orgPluginStatus,
      });
      notifyIfNeedsAttention({ title: "Claude Desktop 本地插件包", message: result.message || result.outcome.message, status: result.status });
      await refreshPluginHub(true);
    }
  };

  const openClaudeDesktopOrgPluginsDir = async () => {
    const result = await run(() => call<ClaudeDesktopOrgPluginStatusResult>("open_claude_desktop_org_plugins_dir"), "Claude Desktop 组织插件目录");
    if (result) {
      setClaudeDesktopOrgPlugin(result);
      notifyIfNeedsAttention({ title: "Claude Desktop 组织插件目录", message: result.message, status: result.status });
    }
  };

  const openPonytailClaudeDesktopMarketplaceSetup = async () => {
    const result = await run(() => call<ClaudeDesktopMarketplaceOpenResult>("open_ponytail_claude_desktop_marketplace_setup"), "Claude Desktop 插件仓库");
    if (result) {
      setClaudeDesktopMarketplace({
        status: result.status,
        message: result.message,
        marketplaceStatus: result.marketplaceStatus,
      });
      notifyIfNeedsAttention({ title: "Claude Desktop 插件仓库", message: result.message || result.outcome.message, status: result.status });
    }
  };

  const configureClaudeDesktopDevMode = async () => {
    const request = claudeDesktopProviderDraft.baseUrl.trim()
      ? { request: claudeDesktopProviderDraft }
      : undefined;
    setClaudeDevModeBusy(true);
    setNotice({ title: "Claude 一键开发模式", message: "正在写入 Claude Desktop 开发配置...", status: "running" });
    try {
      const result = await run(() => call<ClaudeDesktopDevModeConfigureResult>("configure_claude_desktop_dev_mode", request), "Claude Desktop 开发模式");
      if (result) {
        setClaudeDesktopDevMode({
          status: result.status,
          message: result.message,
          devModeStatus: result.devModeStatus,
        });
        setNotice({ title: "Claude 一键开发模式", message: result.message || result.outcome.message, status: result.status });
        await refreshClaudeDesktopDevMode(true);
        await refreshClaudeDesktopOrgPlugin(true);
      }
    } finally {
      setClaudeDevModeBusy(false);
    }
  };

  const installMarketScript = async (id: string) => {
    const result = await run(() => call<ScriptMarketResult>("install_market_script", { id }), "安装脚本");
    if (result) {
      setScriptMarket(result);
      notifyIfNeedsAttention({ title: "脚本市场", message: result.message, status: result.status });
    }
  };

  const repairCodexPluginMarketplace = async () => {
    const result = await run(() => call<CodexPluginMarketplaceRepairResult>("repair_codex_plugin_marketplace"), "下载并注册 Codex 插件仓库");
    if (result) {
      setCodexPluginMarketplace({
        status: result.status,
        message: result.message,
        marketplace: result.marketplace,
      });
      notifyIfNeedsAttention({ title: "Codex OpenAI 插件仓库", message: result.message || result.repair.message, status: result.status });
      await refreshPluginHub(true);
    }
  };

  const openExternalUrl = async (url: string) => {
    await run(() => call<CommandResult<Record<string, unknown>>>("open_external_url", { url }), "打开链接");
  };

  const goPluginHub = async () => {
    setRoute("tools");
    await refreshRoute("tools");
  };

  const goPromptOptimizer = async () => {
    await openExternalUrl(PROMPT_OPTIMIZER_URL);
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

  const repairHistorySessions = async () => {
    const result = await run(() => call<ProviderSyncResult>("sync_providers_now"), "历史会话修复");
    if (result) {
      setProviderSync(result);
      notifyIfNeedsAttention({ title: "历史会话修复", message: result.message, status: result.status });
      await refreshLocalSessions(true);
      await refreshSettings(true);
    }
  };

  const deleteLocalSession = async (session: LocalSession) => {
    const title = session.title || session.id;
    if (!window.confirm(`确认删除 Codex 本地会话？\n\n${title}\n${session.id}`)) return;
    const result = await run(
      () => call<DeleteLocalSessionResult>("delete_local_session", { request: { sessionId: session.id, title: session.title, dbPath: session.dbPath } }),
      "删除 Codex 会话",
    );
    if (result) {
      notifyIfNeedsAttention({ title: "删除 Codex 会话", message: result.message, status: result.status });
      await refreshLocalSessions(true);
    }
  };

  const learnMemoryAssistItem = async (text: string, category = "manual") => {
    const result = await run(
      () => call<MemoryItemResult>("learn_memory_assist_item", { request: { text, category, workspace: MEMORY_GLOBAL_WORKSPACE, source: "manager" } }),
      "保存记忆",
    );
    if (result) {
      notifyIfNeedsAttention({ title: "盘古记忆", message: result.message, status: result.status });
      await refreshMemoryAssist(true);
    }
    return result?.status === "ok";
  };

  const searchMemoryAssist = async (query: string) => {
    const result = await run(
      () => call<MemoryQueryResult>("query_memory_assist", { request: { query, workspace: MEMORY_ALL_WORKSPACES, includeGlobal: true, limit: 12 } }),
      "搜索记忆",
    );
    if (result) {
      setMemorySearch(result);
      notifyIfNeedsAttention({ title: "记忆搜索", message: result.message, status: result.status });
    }
  };

  const deleteMemoryAssistItem = async (id: string) => {
    if (!window.confirm("确认删除这条长期记忆？")) return;
    const result = await run(() => call<MemoryItemResult>("delete_memory_assist_item", { request: { id } }), "删除记忆");
    if (result) {
      notifyIfNeedsAttention({ title: "盘古记忆", message: result.message, status: result.status });
      await refreshMemoryAssist(true);
    }
  };

  const approveMemoryAssistCandidate = async (id: string) => {
    const result = await run(() => call<MemoryItemResult>("approve_memory_assist_candidate", { request: { id } }), "确认待确认记忆");
    if (result) {
      notifyIfNeedsAttention({ title: "盘古记忆", message: result.message, status: result.status });
      await refreshMemoryAssist(true);
    }
  };

  const rejectMemoryAssistCandidate = async (id: string) => {
    const result = await run(() => call<MemoryCandidateResult>("reject_memory_assist_candidate", { request: { id } }), "忽略待确认记忆");
    if (result) {
      notifyIfNeedsAttention({ title: "盘古记忆", message: result.message, status: result.status });
      await refreshMemoryAssist(true);
    }
  };

  const exportMemoryAssist = async () => {
    const result = await run(() => call<MemoryExportResult>("export_memory_assist"), "导出记忆");
    if (result) {
      setMemoryExport(result);
      notifyIfNeedsAttention({ title: "记忆导出", message: result.message, status: result.status });
    }
  };

  const importMemoryAssist = async (jsonText: string, replaceExisting: boolean) => {
    let data: MemoryExport;
    try {
      data = JSON.parse(jsonText) as MemoryExport;
    } catch (error) {
      setNotice({ title: "记忆导入", message: `JSON 解析失败：${stringifyError(error)}`, status: "failed" });
      return;
    }
    if (!data || data.schemaVersion !== "memory-assist/v1" || !Array.isArray(data.items) || !Array.isArray(data.candidates)) {
      setNotice({ title: "记忆导入", message: "导入内容不是 memory-assist/v1 导出包。", status: "failed" });
      return;
    }
    const action = replaceExisting ? "替换现有记忆库" : "合并到现有记忆库";
    if (!window.confirm(`确认导入记忆数据？\n\n${action}\n长期记忆：${data.items.length} 条\n待确认：${data.candidates.length} 条`)) return;
    const result = await run(
      () => call<MemoryStatusResult>("import_memory_assist", { request: { data, replaceExisting } }),
      "导入记忆",
    );
    if (result) {
      setMemoryAssist(result);
      notifyIfNeedsAttention({ title: "记忆导入", message: result.message, status: result.status });
      await refreshMemoryAssist(true);
    }
  };

  const runMemoryAssistSelfcheck = async () => {
    const result = await run(() => call<MemorySelfCheckResult>("run_memory_assist_selfcheck", { request: { repair: true } }), "盘古记忆自检");
    if (result) {
      setMemorySelfCheck(result);
      notifyIfNeedsAttention({ title: "盘古记忆自检", message: result.message, status: result.status });
      await refreshMemoryAssist(true);
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

  const switchCodexRelayProfile = async (profileId: string, sourceSettings?: BackendSettings) => {
    const current = sourceSettings ?? settings?.settings;
    if (!current) {
      setNotice({ title: "供应商切换", message: "设置尚未加载，无法切换 Codex 供应商。", status: "failed" });
      return;
    }
    const targetProfile = current.relayProfiles.find((profile) => profile.id === profileId);
    if (targetProfile && !targetProfile.apiKey.trim()) {
      setNotice({ title: "供应商切换", message: "该供应商缺少 API Key。记录已可保存，请补入 Key 后再切换写入。", status: "failed" });
      return;
    }
    const previousActiveRelayId = current.activeRelayId;
    const next = { ...current, activeRelayId: profileId, relayProfilesEnabled: true };
    const result = await run(
      () => call<SettingsResult & { relay?: unknown }>("switch_relay_profile", { request: { settings: next, previousActiveRelayId } }),
      "切换 Codex 供应商",
    );
    if (result) {
      setSettings(result);
      setSettingsDraft(result.settings);
      notifyIfNeedsAttention({ title: "切换 Codex 供应商", message: result.message, status: result.status });
      await refreshSettings(true);
    }
  };

  const fetchRelayProfileModels = async (profile: RelayProfile) => {
    const result = await run(() => call<RelayProfileModelsResult>("fetch_relay_profile_models", { profile }), "获取供应商模型");
    if (result) {
      notifyIfNeedsAttention({ title: "获取供应商模型", message: result.message, status: result.status });
    }
    return result;
  };

  const previewClaudeDesktopProvider = async (request: typeof claudeDesktopProviderDraft) => {
    const result = await run(
      () => call<ClaudeDesktopProviderPreviewResult>("preview_claude_desktop_provider", { request }),
      "预览 Claude Desktop 供应商",
    );
    if (result) {
      setClaudeDesktopProviderPreview(result);
      notifyIfNeedsAttention({ title: "Claude Desktop 供应商预览", message: result.message, status: result.status });
    }
  };

  const applyClaudeDesktopProvider = async (request: typeof claudeDesktopProviderDraft) => {
    if (!request.apiKey.trim()) {
      setNotice({ title: "Claude Desktop 供应商", message: "API Key 为空，未写入配置。", status: "failed" });
      return;
    }
    const result = await run(
      () => call<ClaudeDesktopProviderApplyResult>("apply_claude_desktop_provider", { request }),
      "写入 Claude Desktop 供应商",
    );
    if (result) {
      setClaudeDesktopProviderApply(result);
      setClaudeDesktopDevMode({
        status: result.status,
        message: result.message,
        devModeStatus: result.devModeStatus,
      });
      notifyIfNeedsAttention({ title: "Claude Desktop 供应商", message: result.message, status: result.status });
      await refreshClaudeDesktopDevMode(true);
    }
  };

  const restoreClaudeDesktopProviderOfficial = async () => {
    if (!window.confirm("确认将 Claude Desktop 切回官方部署模式？操作前会备份现有配置。")) return;
    const result = await run(
      () => call<ClaudeDesktopProviderApplyResult>("restore_claude_desktop_provider_official"),
      "恢复 Claude Desktop 官方模式",
    );
    if (result) {
      setClaudeDesktopProviderApply(result);
      setClaudeDesktopDevMode({
        status: result.status,
        message: result.message,
        devModeStatus: result.devModeStatus,
      });
      notifyIfNeedsAttention({ title: "Claude Desktop 官方模式", message: result.message, status: result.status });
      await refreshClaudeDesktopDevMode(true);
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
      await Promise.all([refreshOverview(true), refreshClaude(true), refreshMemoryAssistStatus(true)]);
    } else if (target === "settings") {
      await refreshSettings(true);
    } else if (target === "supplier") {
      await Promise.all([refreshSettings(true), refreshClaudeDesktopDevMode(true)]);
    } else if (target === "tools") {
      await Promise.all([
        refreshPluginHub(true),
        refreshCodexPluginMarketplace(true),
        refreshClaudeDesktopOrgPlugin(true),
        refreshClaudeDesktopMarketplace(true),
        refreshClaudeDesktopDevMode(true),
        refreshSettings(true),
        refreshOverview(true),
        refreshClaude(true),
        refreshWatcher(true),
      ]);
    } else if (target === "sessions") {
      await Promise.all([
        refreshLocalSessions(true),
        refreshMemoryAssist(true),
        refreshSettings(true),
        refreshOverview(true),
        refreshClaude(true),
      ]);
    } else if (target === "scripts") {
      await Promise.all([refreshSettings(true), refreshScripts(true)]);
    } else if (target === "logs") {
      await refreshLogs(true);
    }
  };

  useEffect(() => {
    const navigate = (event: Event) => {
      const route = normalizeRoute((event as CustomEvent<{ route?: unknown }>).detail?.route);
      if (!isRoute(route)) return;
      setRoute(route);
    };
    window.addEventListener("claude-codex-pro-navigate", navigate);
    return () => window.removeEventListener("claude-codex-pro-navigate", navigate);
  }, []);

  useEffect(() => {
    void refreshRoute(route);
  }, [route]);

  useEffect(() => {
    document.documentElement.classList.add("dark");
    document.documentElement.classList.remove("light");
  }, []);

  useEffect(() => {
    document.title = routeDocumentTitle(route);
  }, [route]);

  const actions = {
      refreshRoute,
      showNotice: setNotice,
      openClaudeChinese,
      installClaudeZhPatch,
      installClaudeZhPatchFromDirectory,
      restoreClaudeZhPatch,
      launchClaudeDesktop,
      launchCodex,
      restartCodex,
      openExternalUrl,
      goPluginHub,
      goPromptOptimizer,
      previewPlugin,
      installPlugin,
      uninstallPlugin,
      previewPonytailCodexHooks,
      trustPonytailCodexHooks,
      generatePonytailMcpbInstaller,
      installPonytailClaudeDesktopOrgPlugin,
      installPonytailClaudeDesktopLocalBundle,
      openClaudeDesktopOrgPluginsDir,
      openPonytailClaudeDesktopMarketplaceSetup,
      configureClaudeDesktopDevMode,
      installMarketScript,
      refreshCodexPluginMarketplace,
      repairCodexPluginMarketplace,
      refreshPluginHub,
      refreshClaudeDesktopOrgPlugin,
      refreshClaudeDesktopMarketplace,
      refreshClaudeDesktopDevMode,
      refreshScripts,
      repairEntrypoints,
      repairBackend,
      repairHistorySessions,
      refreshLocalSessions,
      deleteLocalSession,
      refreshMemoryAssist,
      learnMemoryAssistItem,
      searchMemoryAssist,
      deleteMemoryAssistItem,
      approveMemoryAssistCandidate,
      rejectMemoryAssistCandidate,
      runMemoryAssistSelfcheck,
      exportMemoryAssist,
      importMemoryAssist,
      applyRelayMode,
      applyPureApiMode,
      clearRelayMode,
      switchCodexRelayProfile,
      fetchRelayProfileModels,
      previewClaudeDesktopProvider,
      applyClaudeDesktopProvider,
      restoreClaudeDesktopProviderOfficial,
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
  };

  return (
    <div className="ops-shell dark">
      <aside className="ops-rail">
        <div className="ops-brand" title="Claude Codex Pro">
          <span>CCP</span>
        </div>
        <nav>
          {routes.map((item) => {
            const Icon = item.icon;
            return (
              <button
                className={route === item.id ? "active" : ""}
                key={item.id}
                onClick={() => {
                  setRoute(item.id);
                }}
                title={item.label}
                type="button"
              >
                <Icon className="h-4 w-4" />
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>
      </aside>
      <main className="ops-workspace">
        <header className="ops-topbar">
          <div className="ops-topbar-copy">
            <h1>{routeLabel(route)}</h1>
            <p>{routeSubtitle(route)}</p>
          </div>
          {route !== "overview" ? (
            <div className="ops-topbar-pill">
              <span>后端链接</span>
              <strong>
                {overview?.latest_launch?.helper_port
                  ? `127.0.0.1:${overview.latest_launch.helper_port}`
                  : overview?.latest_launch?.debug_port
                    ? `127.0.0.1:${overview.latest_launch.debug_port}`
                    : "Bridge ready"}
              </strong>
            </div>
          ) : null}
          <div className="ops-commandbar">
            <Button aria-label="启动/重启Codex" disabled={busy} onClick={() => void actions.restartCodex()} variant="outline">
              <Rocket className="h-4 w-4" />
              <span className="desktop-command-label">启动/重启Codex</span>
              <span aria-hidden="true" className="mobile-command-label">Codex</span>
            </Button>
            <Button aria-label="启动/重启Claude" disabled={busy} onClick={() => void actions.launchClaudeDesktop()} variant="outline">
              <MessageCircle className="h-4 w-4" />
              <span className="desktop-command-label">启动/重启Claude</span>
              <span aria-hidden="true" className="mobile-command-label">Claude</span>
            </Button>
            <Button aria-label="Claude 一键汉化" className="ops-primary-command" disabled={busy} onClick={() => void actions.installClaudeZhPatch()}>
              <Languages className="h-4 w-4" />
              <span className="desktop-command-label">Claude 一键汉化</span>
              <span aria-hidden="true" className="mobile-command-label">汉化</span>
            </Button>
            <Button disabled={busy} onClick={() => void actions.refreshRoute()} size="icon" variant="outline">
              <RefreshCw className="h-4 w-4" />
            </Button>
          </div>
        </header>
        <section className="ops-screen">
          {route === "overview" ? <OverviewScreen actions={actions} claudeChinese={claudeChinese} claudeDesktop={claudeDesktop} claudeDesktopDevMode={claudeDesktopDevMode} claudeDevModeBusy={claudeDevModeBusy} memoryAssist={memoryAssist} overview={overview} /> : null}
          {route === "supplier" ? (
            <SupplierScreen
              actions={actions}
              claudeDesktopDevMode={claudeDesktopDevMode}
              claudeDesktopProviderApply={claudeDesktopProviderApply}
              claudeDesktopProviderDraft={claudeDesktopProviderDraft}
              claudeDesktopProviderPreview={claudeDesktopProviderPreview}
              onClaudeDesktopProviderDraftChange={setClaudeDesktopProviderDraft}
              settings={settings}
            />
          ) : null}
          {route === "tools" ? (
            <ToolsAndPluginsScreen
              actions={actions}
              claudeDesktopDevMode={claudeDesktopDevMode}
              claudeDesktopMarketplace={claudeDesktopMarketplace}
              claudeDesktopOrgPlugin={claudeDesktopOrgPlugin}
              codexPluginMarketplace={codexPluginMarketplace}
              hub={pluginHub}
              overview={overview}
              preview={pluginPreview}
              settings={settings}
              watcher={watcher}
            />
          ) : null}
          {route === "sessions" ? (
            <SessionManagementScreen
              actions={actions}
              claudeChinese={claudeChinese}
              claudeDesktop={claudeDesktop}
              localSessions={localSessions}
              memoryAssist={memoryAssist}
              memoryCandidates={memoryCandidates}
              memoryExport={memoryExport}
              memoryItems={memoryItems}
              memorySearch={memorySearch}
              memorySelfCheck={memorySelfCheck}
              providerSync={providerSync}
              settings={settings}
            />
          ) : null}
          {route === "scripts" ? <ScriptsScreen actions={actions} market={scriptMarket} settings={settings} /> : null}
          {route === "logs" ? <LogsScreen actions={actions} logs={logs} /> : null}
          {route === "settings" ? <SettingsScreen actions={actions} claudeChinese={claudeChinese} claudeZhPatch={claudeZhPatch} draft={settingsDraft} onDraftChange={setSettingsDraft} overview={overview} settings={settings} watcher={watcher} /> : null}
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
  claudeDesktopDevMode,
  claudeDevModeBusy,
  memoryAssist,
}: {
  actions: ReturnType<typeof createActionsShape>;
  overview: OverviewResult | null;
  claudeDesktop: ClaudeDesktopResult | null;
  claudeChinese: ClaudeChineseWindowResult | null;
  claudeDesktopDevMode: ClaudeDesktopDevModeStatusResult | null;
  claudeDevModeBusy: boolean;
  memoryAssist: MemoryStatusResult | null;
}) {
  const memory = memoryAssist?.memory;
  const devModeConfigured = !!claudeDesktopDevMode?.devModeStatus.configured;
  const devModeStatus = claudeDevModeBusy ? "running" : devModeConfigured ? "ok" : "not_checked";
  const devModeValue = claudeDevModeBusy ? "写入中..." : devModeConfigured ? "已配置" : "写入开发配置";
  return (
    <div className="ops-dashboard">
      <section className="relay-banner">
        <Network className="h-5 w-5" />
        <div>
          <span>官方中转站</span>
          <strong>拓扑熵减API</strong>
          <p>ClaudeCodexPro 官方中转站，主打稳定接入和划算价格，支持 GPT-5.5、GPT-5.4、Claude Opus 4.8、Claude Opus 4.7、gpt-image-2 等模型与图像能力。</p>
        </div>
        <Button className="relay-banner-open" onClick={() => void actions.openExternalUrl("https://api.toporeduce.cn")} variant="outline">
          <ExternalLink className="h-4 w-4" />
          打开
        </Button>
      </section>
      <div className="ops-matrix">
        <StatusTile icon={Power} label="Codex 运行" status={overview?.latest_launch?.status ?? "not_checked"} value={overview?.latest_launch?.status ?? "无记录"} />
        <StatusTile icon={Activity} label="Codex 版本" status={overview?.codex_version ? "ok" : "not_checked"} value={overview?.codex_version ?? "未检测"} />
        <StatusTile icon={MessageCircle} label="Claude 状态" status={claudeDesktop?.status ?? "not_checked"} value={`${claudeDesktop?.installKind ?? "unknown"} / ${claudeDesktop?.cdpStatus ?? "unknown"}`} />
        <StatusActionTile disabled={claudeDevModeBusy} icon={Wrench} label="Claude 一键开发模式" onClick={() => void actions.configureClaudeDesktopDevMode()} status={devModeStatus} value={devModeValue} />
        <StatusTile icon={ShieldCheck} label="记忆大脑" status={memory?.status ?? "not_checked"} value={`${memory?.totalItems ?? 0} 条 / 待确认 ${memory?.pendingCandidates ?? 0}`} />
      </div>
      <div className="ops-overview-grid">
        <Panel title="Codex 诊断" detail="默认只展示运行、入口和版本状态；修复动作需要手动触发。">
          <InfoRow label="运行状态" value={overview?.latest_launch?.status ?? "无记录"} />
          <InfoRow label="Codex 版本" value={overview?.codex_version ?? "未检测"} />
          <InfoRow label="静默入口" value={overview?.silent_shortcut.status ?? "未检测"} />
          <InfoRow label="管理入口" value={overview?.management_shortcut.status ?? "未检测"} />
        </Panel>
        <Panel title="Claude 诊断" detail="一键汉化会写入本机 Claude Desktop zh-CN 资源、locale 和前端 chunk 补丁。">
          <InfoRow label="安装类型" value={claudeDesktop?.installKind ?? "未检测"} />
          <InfoRow label="CDP 状态" value={claudeDesktop?.cdpStatus ?? "未检测"} />
          <InfoRow label="阻断原因" value={claudeDesktop?.cdpBlocker || "无"} />
          <InfoRow label="一键汉化方式" value="本机资源补丁（zh-CN JSON + locale + chunk）" />
        </Panel>
        <Panel title="盘古记忆总览" detail="本地长期记忆、待确认学习、工作区隔离和备份状态。">
          <InfoRow label="状态" value={memory?.status ?? "未检测"} />
          <InfoRow label="长期记忆" value={`${memory?.totalItems ?? 0} 条`} />
          <InfoRow label="待确认" value={`${memory?.pendingCandidates ?? 0} 条`} />
          <InfoRow label="数据库" value={compactPath(memory?.dbPath)} />
          <ActionButton icon={ShieldCheck} label="盘古记忆自检并备份" onClick={() => void actions.runMemoryAssistSelfcheck()} />
          <ActionButton icon={Wrench} label="Claude 一键开发模式" onClick={() => void actions.configureClaudeDesktopDevMode()} />
        </Panel>
        <Panel title="诊断与修复" detail="检查和修复入口集中在这里；不会自动改写配置。">
          <ActionButton icon={RefreshCw} label="刷新概览" onClick={() => void actions.refreshRoute("overview")} />
          <ActionButton icon={Wrench} label="修复入口" onClick={() => void actions.repairEntrypoints()} />
          <ActionButton icon={Wrench} label="修复后端" onClick={() => void actions.repairBackend()} />
        </Panel>
      </div>
    </div>
  );
}

function SupplierScreen({
  actions,
  settings,
  claudeDesktopDevMode,
  claudeDesktopProviderPreview,
  claudeDesktopProviderApply,
  claudeDesktopProviderDraft,
  onClaudeDesktopProviderDraftChange,
}: {
  actions: ReturnType<typeof createActionsShape>;
  settings: SettingsResult | null;
  claudeDesktopDevMode: ClaudeDesktopDevModeStatusResult | null;
  claudeDesktopProviderPreview: ClaudeDesktopProviderPreviewResult | null;
  claudeDesktopProviderApply: ClaudeDesktopProviderApplyResult | null;
  claudeDesktopProviderDraft: {
    name: string;
    baseUrl: string;
    apiKey: string;
    modelList: string;
  };
  onClaudeDesktopProviderDraftChange: Dispatch<SetStateAction<{
    name: string;
    baseUrl: string;
    apiKey: string;
    modelList: string;
  }>>;
}) {
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draft, setDraft] = useState<RelayProfile | null>(null);
  const [modelFetch, setModelFetch] = useState<RelayProfileModelsResult | null>(null);
  const [supplierSaveBusy, setSupplierSaveBusy] = useState(false);
  const appSettings = settings?.settings ?? null;
  const profiles = appSettings?.relayProfiles ?? [];
  const active = profiles.find((profile) => profile.id === appSettings?.activeRelayId) ?? profiles[0];
  const editingExisting = draft && editingId ? profiles.find((profile) => profile.id === editingId) : null;
  const isNewDraft = !!draft && !editingExisting;
  const updateClaudeDraft = (field: keyof typeof claudeDesktopProviderDraft, value: string) => {
    onClaudeDesktopProviderDraftChange((current) => ({ ...current, [field]: value }));
  };
  const saveSupplierSettings = async (next: BackendSettings) => {
    const result = await actions.saveSettings(next);
    if (!result) return null;
    if (statusFailed(result.status)) {
      actions.showNotice({ title: "供应商保存", message: result.message || "保存设置失败。", status: "failed" });
      return null;
    }
    return result.settings;
  };
  const openProfileEditor = (profile: RelayProfile) => {
    setModelFetch(null);
    setEditingId(profile.id);
    setDraft(normalizeSupplierProfile(profile));
  };
  const createProfile = () => {
    if (!appSettings) return;
    setModelFetch(null);
    setEditingId(null);
    setDraft(createSupplierProfile(appSettings));
  };
  const duplicateProfile = (profile: RelayProfile) => {
    if (!appSettings) return;
    const copy = {
      ...normalizeSupplierProfile(profile),
      id: uniqueSupplierProfileId(appSettings.relayProfiles, `${profile.id || "provider"}-copy`),
      name: `${profile.name || profile.id || "供应商"} 副本`,
    };
    setModelFetch(null);
    setEditingId(null);
    setDraft(copy);
  };
  const updateDraft = (patch: Partial<RelayProfile>) => {
    setDraft((current) => current ? normalizeSupplierProfile(withSupplierGeneratedFiles({ ...current, ...patch })) : current);
  };
  const updateDraftId = (value: string, options: { normalize?: boolean } = {}) => {
    setDraft((current) => {
      if (!current) return current;
      const nextId = options.normalize ? supplierIdFromName(value || current.name) : value;
      const next = withSupplierGeneratedFiles({ ...current, id: nextId });
      return options.normalize ? normalizeSupplierProfile(next) : { ...next, id: nextId };
    });
  };
  const saveDraft = async (options: { stayInEditor?: boolean } = {}): Promise<SupplierSaveResult | null> => {
    if (!appSettings || !draft || supplierSaveBusy) return null;
    const requestedId = draft.id.trim();
    const normalizedId = supplierIdFromName(requestedId || draft.name);
    const idWasNormalized = requestedId !== normalizedId;
    const normalized = normalizeSupplierProfile(withSupplierGeneratedFiles({ ...draft, id: normalizedId }));
    if (!normalized.name.trim() || !normalized.baseUrl.trim()) {
      window.alert("请填写供应商名称和 Base URL 后再保存。API Key 可以后续补入。");
      return null;
    }
    const originalId = editingId;
    const conflicts = profiles.some((profile) => profile.id === normalized.id && profile.id !== originalId);
    if (conflicts) {
      window.alert(`供应商 ID「${normalized.id}」已存在，请换一个 ID。`);
      return null;
    }
    const nextProfiles = originalId && profiles.some((profile) => profile.id === originalId)
      ? profiles.map((profile) => (profile.id === originalId ? normalized : profile))
      : profiles.some((profile) => profile.id === normalized.id)
        ? profiles.map((profile) => (profile.id === normalized.id ? normalized : profile))
        : [...profiles, normalized];
    const nextActiveRelayId = appSettings.activeRelayId === originalId || !appSettings.activeRelayId
      ? normalized.id
      : appSettings.activeRelayId;
    setSupplierSaveBusy(true);
    try {
      actions.showNotice({ title: "供应商保存", message: `正在保存供应商「${normalized.name || normalized.id}」...`, status: "running" });
      const saved = await saveSupplierSettings({
        ...appSettings,
        relayProfilesEnabled: true,
        relayProfiles: nextProfiles,
        activeRelayId: nextActiveRelayId,
      });
      if (saved) {
        const savedProfile = saved.relayProfiles.find((profile) => profile.id === normalized.id) ?? normalized;
        if (options.stayInEditor) {
          setEditingId(savedProfile.id);
          setDraft(normalizeSupplierProfile(withSupplierGeneratedFiles(savedProfile)));
        } else {
          setEditingId(null);
          setDraft(null);
        }
        actions.showNotice({ title: "供应商保存", message: `已保存供应商「${savedProfile.name || savedProfile.id}」。`, status: "ok" });
        if (idWasNormalized) {
          actions.showNotice({ title: "供应商保存", message: `供应商 ID 已自动整理为「${savedProfile.id}」。`, status: "ok" });
        }
        return { settings: saved, profile: savedProfile };
      }
      return null;
    } finally {
      setSupplierSaveBusy(false);
    }
  };
  const saveAndSwitchDraft = async () => {
    if (!draft) return;
    const saved = await saveDraft({ stayInEditor: true });
    if (saved) {
      const savedProfile = normalizeSupplierProfile(saved.profile);
      if (!savedProfile.apiKey.trim()) {
        actions.showNotice({ title: "供应商切换", message: "供应商已保存。请先补入 API Key，再写入为当前供应商。", status: "failed" });
        return;
      }
      await actions.switchCodexRelayProfile(savedProfile.id, saved.settings);
    }
  };
  const removeProfile = async (profile: RelayProfile) => {
    if (!appSettings || profiles.length <= 1) {
      window.alert("至少保留一个供应商配置。");
      return;
    }
    if (!window.confirm(`确认删除供应商「${profile.name || profile.id}」？`)) return;
    const nextProfiles = profiles.filter((item) => item.id !== profile.id);
    const nextActive = appSettings.activeRelayId === profile.id ? nextProfiles[0]?.id ?? "" : appSettings.activeRelayId;
    const saved = await saveSupplierSettings({ ...appSettings, relayProfiles: nextProfiles, activeRelayId: nextActive });
    if (saved && editingId === profile.id) {
      setEditingId(null);
      setDraft(null);
    }
  };
  const applyPreset = (preset: SupplierPreset) => {
    if (!draft) return;
    updateDraft({
      id: isNewDraft ? uniqueSupplierProfileId(profiles, preset.id) : draft.id,
      name: preset.name,
      baseUrl: preset.baseUrl,
      upstreamBaseUrl: preset.baseUrl,
      protocol: preset.protocol,
      relayMode: "pureApi",
      model: preset.model,
      testModel: preset.model,
      modelList: preset.modelList?.join("\n") ?? preset.model,
    });
  };
  const fetchModels = async () => {
    if (!draft) return;
    const normalized = normalizeSupplierProfile(withSupplierGeneratedFiles(draft));
    const result = await actions.fetchRelayProfileModels(normalized);
    if (result) {
      setModelFetch(result);
      if (result.models.length) {
        updateDraft({
          modelList: result.models.join("\n"),
          model: normalized.model || result.models[0],
          testModel: normalized.testModel || result.models[0],
        });
      }
    }
  };
  const toggleMasterSwitch = async (enabled: boolean) => {
    if (!appSettings) return;
    await saveSupplierSettings({ ...appSettings, relayProfilesEnabled: enabled });
  };

  if (draft) {
    const generated = normalizeSupplierProfile(withSupplierGeneratedFiles(draft));
    const canSwitch = !!editingExisting && appSettings?.relayProfilesEnabled !== false;
    return (
      <div className="supplier-workbench">
        <Panel title={isNewDraft ? "添加 Codex 供应商" : "编辑 Codex 供应商"} detail="保存会写入管理器 settings；设为当前会调用真实切换命令写入 Codex config.toml 和 auth.json。">
          <div className="supplier-editor-toolbar">
            <Button onClick={() => { setDraft(null); setEditingId(null); }} variant="outline">返回列表</Button>
            <Button disabled={supplierSaveBusy} onClick={() => void saveDraft()} type="button" variant="outline">
              <Save className="h-4 w-4" />
              {supplierSaveBusy ? "保存中" : "保存"}
            </Button>
            <Button disabled={!canSwitch || supplierSaveBusy} onClick={() => void saveAndSwitchDraft()} type="button">
              <KeyRound className="h-4 w-4" />
              {generated.id === appSettings?.activeRelayId ? "重新写入当前供应商" : "保存并设为当前"}
            </Button>
          </div>
          <div className="supplier-editor-layout">
            <div className="supplier-main-editor">
              {isNewDraft ? (
                <div className="supplier-preset-strip">
                  {SUPPLIER_PRESETS.map((preset) => (
                    <button key={preset.id} onClick={() => applyPreset(preset)} type="button">
                      <strong>{preset.name}</strong>
                      <span>{supplierCategoryLabel(preset.category)} · {supplierProtocolLabel(preset.protocol)}</span>
                    </button>
                  ))}
                </div>
              ) : null}
              <div className="supplier-form-grid">
                <label className="ops-form-field">
                  <span>供应商名称</span>
                  <input onChange={(event) => updateDraft({ name: event.currentTarget.value })} value={generated.name} />
                </label>
                <label className="ops-form-field">
                  <span>供应商 ID</span>
                  <input onBlur={(event) => updateDraftId(event.currentTarget.value || draft.name, { normalize: true })} onChange={(event) => updateDraftId(event.currentTarget.value)} value={draft.id} />
                </label>
                <label className="ops-form-field">
                  <span>Base URL</span>
                  <input onChange={(event) => updateDraft({ baseUrl: event.currentTarget.value, upstreamBaseUrl: event.currentTarget.value })} placeholder="https://api.example.com/v1" value={generated.baseUrl} />
                </label>
                <label className="ops-form-field">
                  <span>默认模型</span>
                  <input onChange={(event) => updateDraft({ model: event.currentTarget.value, testModel: event.currentTarget.value })} placeholder="gpt-5.5" value={generated.model} />
                </label>
                <label className="ops-form-field">
                  <span>API Key / Bearer Token</span>
                  <input onChange={(event) => updateDraft({ apiKey: event.currentTarget.value })} type="password" value={generated.apiKey} />
                </label>
                <label className="ops-form-field">
                  <span>协议</span>
                  <select className="ops-select" onChange={(event) => updateDraft({ protocol: event.currentTarget.value })} value={generated.protocol || "responses"}>
                    <option value="responses">Responses API</option>
                    <option value="chatCompletions">Chat Completions（本地协议代理）</option>
                  </select>
                </label>
                <label className="ops-form-field span-2">
                  <span>模型列表（一行一个）</span>
                  <textarea className="ops-textarea mono" onChange={(event) => updateDraft({ modelList: event.currentTarget.value })} rows={5} value={generated.modelList} />
                </label>
              </div>
              <div className="action-row">
                <Button onClick={() => void fetchModels()} variant="outline">
                  <RefreshCw className="h-4 w-4" />
                  从供应商拉取模型
                </Button>
                {modelFetch?.models.length ? <span className="supplier-inline-note">已从 {modelFetch.endpoint || "模型接口"} 获取 {modelFetch.models.length} 个模型</span> : null}
              </div>
            </div>
            <div className="supplier-config-preview">
              <StatusRow label="写入模式" status="running" value="纯 API：config.toml + auth.json" />
              <StatusRow label="当前状态" status={generated.id === appSettings?.activeRelayId ? "running" : "not_checked"} value={generated.id === appSettings?.activeRelayId ? "当前供应商" : "未启用"} />
              <div className="preview-box">
                <strong>config.toml 预览</strong>
                <pre>{generated.configContents}</pre>
              </div>
              <div className="preview-box">
                <strong>auth.json 预览</strong>
                <pre>{redactSupplierAuth(generated.authContents)}</pre>
              </div>
            </div>
          </div>
        </Panel>
      </div>
    );
  }

  return (
    <div className="ops-two-column supplier-layout">
      <div className="ops-wide-column">
        <Panel title="供应商配置" detail="管理 Codex API 供应商、协议、Key 与配置文件；切换时会真实写入 Codex live 配置。">
          <div className="supplier-headline">
            <div>
              <span>当前供应商</span>
              <strong>{active?.name || active?.id || "未配置"}</strong>
              <small>{active?.baseUrl || "官方登录模式"} · {supplierProtocolLabel(active?.protocol)}</small>
            </div>
            <ToggleSwitch checked={appSettings?.relayProfilesEnabled !== false} disabled={!appSettings} onChange={(value) => void toggleMasterSwitch(value)} />
          </div>
          <div className="supplier-toolbar">
            <Button disabled={!appSettings} onClick={createProfile}>
              <Plus className="h-4 w-4" />
              添加供应商
            </Button>
            <Button disabled title="当前后端尚未实现聚合轮转代理；为避免假功能，暂不开放保存。" variant="outline">
              <Plus className="h-4 w-4" />
              添加聚合供应商
            </Button>
            <Button onClick={() => window.alert("当前版本的后端模型没有聚合轮转字段；等核心代理接入后再开放真实聚合供应商。")} variant="outline">
              查看聚合说明
            </Button>
          </div>
          <div className="supplier-card-list">
            {profiles.length ? profiles.map((profile) => {
              const selected = profile.id === appSettings?.activeRelayId;
              return (
                <div className={`supplier-card ${selected ? "selected" : ""}`} key={profile.id}>
                  <div className="supplier-card-main">
                    <div className="supplier-title-line">
                      <strong>{profile.name || profile.id}</strong>
                      {selected ? <span className="supplier-badge">当前</span> : null}
                    </div>
                    <span>{supplierRelayModeLabel(profile.relayMode)} · {profile.model || profile.testModel || "未设置模型"}</span>
                    <small>{profile.baseUrl || "官方登录"} · {supplierProtocolLabel(profile.protocol)}</small>
                  </div>
                  <div className="supplier-card-actions">
                    <Button disabled={selected || appSettings?.relayProfilesEnabled === false} onClick={() => void actions.switchCodexRelayProfile(profile.id)} size="sm" variant="outline">
                      {selected ? "使用中" : "切换"}
                    </Button>
                    <Button onClick={() => openProfileEditor(profile)} size="sm" variant="outline">编辑</Button>
                    <Button onClick={() => duplicateProfile(profile)} size="sm" variant="outline" title="复制供应商">
                      <Copy className="h-4 w-4" />
                    </Button>
                    <Button disabled={profiles.length <= 1} onClick={() => void removeProfile(profile)} size="sm" variant="outline" title="删除供应商">
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              );
            }) : <Empty text="暂无供应商配置，点击“添加供应商”创建一个真实可切换的 Codex API 配置。" />}
          </div>
        </Panel>
        <Panel title="Claude Desktop 开发模式供应商" detail="写入 Claude Desktop 3P gateway profile；不修改 MSIX，不需要 Claude CLI 登录。">
          <div className="supplier-form-grid">
            <label className="ops-form-field">
              <span>显示名称</span>
              <input onChange={(event) => updateClaudeDraft("name", event.currentTarget.value)} value={claudeDesktopProviderDraft.name} />
            </label>
            <label className="ops-form-field">
              <span>Gateway Base URL</span>
              <input onChange={(event) => updateClaudeDraft("baseUrl", event.currentTarget.value)} placeholder="https://api.toporeduce.cn" value={claudeDesktopProviderDraft.baseUrl} />
            </label>
            <label className="ops-form-field">
              <span>API Key / Bearer Token</span>
              <input onChange={(event) => updateClaudeDraft("apiKey", event.currentTarget.value)} placeholder="写入前不会出现在日志和预览中" type="password" value={claudeDesktopProviderDraft.apiKey} />
            </label>
            <label className="ops-form-field span-2">
              <span>Claude Desktop 模型菜单，可选，一行一个，支持 [1m]</span>
              <textarea className="ops-textarea mono" onChange={(event) => updateClaudeDraft("modelList", event.currentTarget.value)} rows={5} value={claudeDesktopProviderDraft.modelList} />
            </label>
          </div>
          <div className="action-row">
            <Button onClick={() => void actions.previewClaudeDesktopProvider(claudeDesktopProviderDraft)} variant="outline">
              <FileCode2 className="h-4 w-4" />
              预览写入
            </Button>
            <Button onClick={() => void actions.applyClaudeDesktopProvider(claudeDesktopProviderDraft)}>
              <KeyRound className="h-4 w-4" />
              写入 Claude Desktop
            </Button>
            <Button onClick={() => void actions.restoreClaudeDesktopProviderOfficial()} variant="outline">
              <Trash2 className="h-4 w-4" />
              恢复官方模式
            </Button>
          </div>
          {claudeDesktopProviderPreview?.preview.configDiff ? (
            <pre className="preview-box">{claudeDesktopProviderPreview.preview.configDiff}</pre>
          ) : null}
          {claudeDesktopProviderApply?.outcome.backupPaths?.length ? (
            <div className="risk-box">
              <strong>已创建备份</strong>
              <span>{claudeDesktopProviderApply.outcome.backupPaths.map(compactPath).join("；")}</span>
            </div>
          ) : null}
        </Panel>
      </div>
      <div className="stack">
        <Panel title="Codex 写入状态" detail="这里展示当前 RelayProfile 如何落地到 Codex 配置。">
          <div className="ops-status-list">
            <StatusRow label="供应商开关" status={appSettings?.relayProfilesEnabled !== false ? "running" : "not_checked"} value={appSettings?.relayProfilesEnabled !== false ? "已开启" : "已关闭"} />
            <StatusRow label="纯 API" status={active?.relayMode === "pureApi" ? "running" : "not_checked"} value="写入当前供应商 ID，并将 auth.json 切换到当前供应商。" />
            <StatusRow label="协议代理" status={active?.protocol === "chatCompletions" ? "running" : "not_checked"} value={active?.protocol === "chatCompletions" ? "Chat Completions 会通过本地协议代理转为 Responses。" : "Responses 直连供应商。"} />
          </div>
          <div className="action-row">
            <Button onClick={() => void actions.applyPureApiMode()} variant="outline">
              <Network className="h-4 w-4" />
              重新写入当前供应商
            </Button>
            <Button onClick={() => void actions.clearRelayMode()} variant="outline">
              <Trash2 className="h-4 w-4" />
              清除 API 模式
            </Button>
          </div>
        </Panel>
        <Panel title="Claude Desktop 3P 状态" detail="开发模式和 profile 写入状态，配置后需要重启 Claude Desktop。">
          <div className="info-grid compact">
            <InfoRow label="开发模式" value={claudeDesktopDevMode?.devModeStatus.configured ? "已配置" : "未配置"} />
            <InfoRow label="普通配置" value={compactPath(claudeDesktopDevMode?.devModeStatus.normalConfigPath)} />
            <InfoRow label="3P 配置" value={compactPath(claudeDesktopDevMode?.devModeStatus.threepConfigPath)} />
            <InfoRow label="Profile Meta" value={compactPath(claudeDesktopDevMode?.devModeStatus.profileMetaPath)} />
          </div>
        </Panel>
        <Panel title="当前配置摘要" detail="只展示路径和非敏感字段。">
          <div className="info-grid compact">
            <InfoRow label="设置文件" value={compactPath(settings?.settings_path)} />
            <InfoRow label="供应商数量" value={`${profiles.length} 个`} />
            <InfoRow label="当前供应商 ID" value={appSettings?.activeRelayId ?? "default"} />
            <InfoRow label="测试模型" value={active?.testModel || appSettings?.relayTestModel || "默认"} />
          </div>
        </Panel>
      </div>
    </div>
  );
}

function LegacySupplierScreen({
  actions,
  settings,
  claudeDesktopDevMode,
  claudeDesktopProviderPreview,
  claudeDesktopProviderApply,
  claudeDesktopProviderDraft,
  onClaudeDesktopProviderDraftChange,
}: {
  actions: ReturnType<typeof createActionsShape>;
  settings: SettingsResult | null;
  claudeDesktopDevMode: ClaudeDesktopDevModeStatusResult | null;
  claudeDesktopProviderPreview: ClaudeDesktopProviderPreviewResult | null;
  claudeDesktopProviderApply: ClaudeDesktopProviderApplyResult | null;
  claudeDesktopProviderDraft: {
    name: string;
    baseUrl: string;
    apiKey: string;
    modelList: string;
  };
  onClaudeDesktopProviderDraftChange: Dispatch<SetStateAction<{
    name: string;
    baseUrl: string;
    apiKey: string;
    modelList: string;
  }>>;
}) {
  const profiles = settings?.settings.relayProfiles ?? [];
  const active = profiles.find((profile) => profile.id === settings?.settings.activeRelayId) ?? profiles[0];
  const updateClaudeDraft = (field: keyof typeof claudeDesktopProviderDraft, value: string) => {
    onClaudeDesktopProviderDraftChange((draft) => ({ ...draft, [field]: value }));
  };
  return (
    <div className="ops-two-column">
      <div className="ops-wide-column">
        <Panel title="Codex 供应商" detail="复用现有 RelayProfile 真实写入 ~/.codex/config.toml 和 auth.json，失败会回滚设置。">
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
        <Panel title="Codex 供应商列表" detail={`${profiles.length} 个配置；点击切换会写入 Codex live 配置，不只是改 UI 状态。`}>
          <div className="ops-status-list">
            {profiles.length ? profiles.map((profile) => {
              const selected = profile.id === settings?.settings.activeRelayId;
              return (
                <div className="supplier-profile-row" key={profile.id}>
                  <StatusRow
                    label={profile.name || profile.id}
                    status={selected ? "running" : "not_checked"}
                    value={`${profile.relayMode || "official"} · ${profile.model || profile.testModel || "默认模型"}`}
                  />
                  <Button disabled={selected} onClick={() => void actions.switchCodexRelayProfile(profile.id)} variant="outline">
                    {selected ? "当前" : "切换"}
                  </Button>
                </div>
              );
            }) : <Empty text="暂无供应商配置。可在设置文件中添加 RelayProfile 后回到这里切换。" />}
          </div>
        </Panel>
        <Panel title="Claude Desktop 开发模式供应商" detail="写入 Claude Desktop 3P gateway profile；不修改 MSIX，不需要 Claude CLI 登录。">
          <div className="supplier-form-grid">
            <label className="ops-form-field">
              <span>显示名称</span>
              <input onChange={(event) => updateClaudeDraft("name", event.currentTarget.value)} value={claudeDesktopProviderDraft.name} />
            </label>
            <label className="ops-form-field">
              <span>Gateway Base URL</span>
              <input onChange={(event) => updateClaudeDraft("baseUrl", event.currentTarget.value)} placeholder="https://api.toporeduce.cn" value={claudeDesktopProviderDraft.baseUrl} />
            </label>
            <label className="ops-form-field">
              <span>API Key / Bearer Token</span>
              <input onChange={(event) => updateClaudeDraft("apiKey", event.currentTarget.value)} placeholder="写入前不会出现在日志和预览中" type="password" value={claudeDesktopProviderDraft.apiKey} />
            </label>
            <label className="ops-form-field span-2">
              <span>Claude Desktop 模型菜单，可选；一行一个，支持 [1m]</span>
              <textarea className="ops-textarea mono" onChange={(event) => updateClaudeDraft("modelList", event.currentTarget.value)} rows={5} value={claudeDesktopProviderDraft.modelList} />
            </label>
          </div>
          <div className="action-row">
            <Button onClick={() => void actions.previewClaudeDesktopProvider(claudeDesktopProviderDraft)} variant="outline">
              <FileCode2 className="h-4 w-4" />
              预览写入
            </Button>
            <Button onClick={() => void actions.applyClaudeDesktopProvider(claudeDesktopProviderDraft)}>
              <KeyRound className="h-4 w-4" />
              写入 Claude Desktop
            </Button>
            <Button onClick={() => void actions.restoreClaudeDesktopProviderOfficial()} variant="outline">
              <Trash2 className="h-4 w-4" />
              恢复官方模式
            </Button>
          </div>
          {claudeDesktopProviderPreview?.preview.configDiff ? (
            <pre className="preview-box">{claudeDesktopProviderPreview.preview.configDiff}</pre>
          ) : null}
          {claudeDesktopProviderApply?.outcome.backupPaths?.length ? (
            <div className="risk-box">
              <strong>已创建备份</strong>
              <span>{claudeDesktopProviderApply.outcome.backupPaths.map(compactPath).join("；")}</span>
            </div>
          ) : null}
        </Panel>
      </div>
      <div className="stack">
        <Panel title="Codex 写入模式" detail="按使用场景选择，不混淆 Claude Desktop 插件安装。">
          <div className="ops-status-list">
            <StatusRow label="官方混入 API Key" status={active?.officialMixApiKey ? "running" : "not_checked"} value="保留官方账号能力，把模型请求转到自定义兼容 API。" />
            <StatusRow label="纯 API" status={active?.relayMode === "pure_api" ? "running" : "not_checked"} value="写入当前供应商 ID，并将 auth 状态切换到当前供应商。" />
            <StatusRow label="清除 API 模式" status="not_checked" value="移除中转 API 配置，回到官方 ChatGPT 登录态。" />
          </div>
        </Panel>
        <Panel title="Claude Desktop 3P 状态" detail="开发模式和 profile 写入状态，配置后需要重启 Claude Desktop。">
          <div className="info-grid compact">
            <InfoRow label="开发模式" value={claudeDesktopDevMode?.devModeStatus.configured ? "已配置" : "未配置"} />
            <InfoRow label="普通配置" value={compactPath(claudeDesktopDevMode?.devModeStatus.normalConfigPath)} />
            <InfoRow label="3P 配置" value={compactPath(claudeDesktopDevMode?.devModeStatus.threepConfigPath)} />
            <InfoRow label="Profile Meta" value={compactPath(claudeDesktopDevMode?.devModeStatus.profileMetaPath)} />
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

function ToolsAndPluginsScreen({
  actions,
  claudeDesktopDevMode,
  claudeDesktopMarketplace,
  claudeDesktopOrgPlugin,
  codexPluginMarketplace,
  hub,
  overview,
  preview,
  settings,
  watcher,
}: {
  actions: ReturnType<typeof createActionsShape>;
  claudeDesktopDevMode: ClaudeDesktopDevModeStatusResult | null;
  claudeDesktopMarketplace: ClaudeDesktopMarketplaceStatusResult | null;
  claudeDesktopOrgPlugin: ClaudeDesktopOrgPluginStatusResult | null;
  codexPluginMarketplace: CodexPluginMarketplaceStatusResult | null;
  hub: PluginHubResult | null;
  overview: OverviewResult | null;
  preview: PluginInstallPreviewResult | null;
  settings: SettingsResult | null;
  watcher: WatcherResult | null;
}) {
  const common = settings?.settings.relayContextConfigContents || settings?.settings.relayCommonConfigContents || "";
  const codexPluginSource = hub?.catalog.sources.find((source) => source.id === "codex-plugins");
  return (
    <div className="stack">
      <div className="ops-tools-command-deck">
        <Panel title="工具与插件" detail="插件目录、MCP 配置和启动入口集中在这里。">
          <div className="ops-note">
            <ShieldCheck className="h-4 w-4" />
            <span>第三方插件和 MCP 安装前仍先展示命令或配置 diff；社区资源不会静默执行脚本。</span>
          </div>
          <div className="action-row">
            <Button onClick={() => void actions.refreshRoute("tools")}>
              <RefreshCw className="h-4 w-4" />
              刷新工具与插件
            </Button>
            <Button onClick={() => void actions.openExternalUrl("https://github.com/openai/plugins")} variant="outline">
              <ExternalLink className="h-4 w-4" />
              Codex 插件仓库
            </Button>
            <Button onClick={() => void actions.openExternalUrl("https://developers.openai.com/codex/plugins")} variant="outline">
              <ExternalLink className="h-4 w-4" />
              Codex 插件文档
            </Button>
          </div>
        </Panel>
      </div>
      <div className="ops-tools-columns">
        <div className="ops-tools-column">
          <PluginHubScreen
            actions={actions}
            hub={hub}
            devMode={claudeDesktopDevMode}
            marketplace={claudeDesktopMarketplace}
            orgPlugin={claudeDesktopOrgPlugin}
            preview={preview}
          />
          <PromptOptimizerCard actions={actions} />
        </div>
        <div className="ops-tools-column">
          <ClaudeDesktopOrgPluginPanel actions={actions} devMode={claudeDesktopDevMode} marketplace={claudeDesktopMarketplace} status={claudeDesktopOrgPlugin} />
          <Panel title="工具与插件配置" detail="MCP、Skills、Plugins 仍保存在统一 TOML 配置中。">
            <pre className="ops-code">{common.trim() || "暂无通用 MCP / Skills / Plugins 配置。"}</pre>
          </Panel>
          <Panel title="Codex OpenAI 插件仓库" detail={codexPluginMarketplace?.marketplace.message ?? codexPluginSource?.message ?? "下载 openai/plugins 并注册为本地 Codex marketplace。"}>
            <InfoRow label="目录来源" value={codexPluginSource?.label ?? "OpenAI Codex Plugins"} />
            <InfoRow label="目录状态" value={codexPluginSource?.status ?? "未加载"} />
            <InfoRow label="目录条目" value={`${codexPluginSource?.itemCount ?? 0} 个`} />
            <InfoRow label="Codex Home" value={compactPath(codexPluginMarketplace?.marketplace.codexHome)} />
            <InfoRow label="本地仓库" value={compactPath(codexPluginMarketplace?.marketplace.marketplaceRoot)} />
            <InfoRow label="config.toml" value={codexPluginMarketplace?.marketplace.configRegistered ? "已注册" : "未注册"} />
            <div className={codexPluginMarketplace?.marketplace.needsRepair ? "ops-danger-zone" : "ops-note"}>
              {codexPluginMarketplace?.marketplace.needsRepair ? <AlertTriangle className="h-4 w-4" /> : <ShieldCheck className="h-4 w-4" />}
              <span>{codexPluginMarketplace?.message ?? "尚未读取本地 Codex 插件仓库状态。"}</span>
            </div>
            <div className="action-row">
              <Button onClick={() => void actions.repairCodexPluginMarketplace()} size="sm">
                <Download className="h-4 w-4" />
                下载并注册
              </Button>
              <Button onClick={() => void actions.refreshCodexPluginMarketplace()} size="sm" variant="outline">刷新状态</Button>
              <Button onClick={() => void actions.openExternalUrl("https://github.com/openai/plugins")} size="sm" variant="outline">打开仓库</Button>
              <Button onClick={() => void actions.openExternalUrl("https://developers.openai.com/codex/plugins")} size="sm" variant="outline">查看文档</Button>
            </div>
          </Panel>
          <MaintenanceToolsPanel actions={actions} overview={overview} settings={settings} watcher={watcher} />
        </div>
      </div>
    </div>
  );
}

function MemoryAssistPanel({
  actions,
  candidates,
  exported,
  items,
  search,
  selfCheck,
  status,
}: {
  actions: ReturnType<typeof createActionsShape>;
  candidates: MemoryCandidatesResult | null;
  exported: MemoryExportResult | null;
  items: MemoryItemsResult | null;
  search: MemoryQueryResult | null;
  selfCheck: MemorySelfCheckResult | null;
  status: MemoryStatusResult | null;
}) {
  const [draft, setDraft] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [importText, setImportText] = useState("");
  const [replaceExisting, setReplaceExisting] = useState(false);
  const recentItems = items?.items.slice(0, 5) ?? [];
  const pending = candidates?.candidates ?? [];
  const matches = search?.memory.results ?? [];
  const exportJson = exported ? JSON.stringify(exported.data, null, 2) : "";
  const dbPath = status?.memory.dbPath ?? "";
  return (
    <Panel title="盘古记忆" detail="本地长期记忆、待确认学习、工作区隔离和自检备份。">
      <div className="ops-status-list">
        <StatusRow label="记忆库" status={status?.memory.status ?? "not_checked"} value={compactPath(dbPath)} />
        <StatusRow label="长期记忆" status={(status?.memory.totalItems ?? 0) > 0 ? "ok" : "not_checked"} value={`${status?.memory.totalItems ?? 0} 条`} />
        <StatusRow label="待确认" status={(status?.memory.pendingCandidates ?? 0) > 0 ? "running" : "not_checked"} value={`${status?.memory.pendingCandidates ?? 0} 条`} />
        <StatusRow label="最近备份" status={status?.memory.latestBackupPath ? "ok" : "not_checked"} value={compactPath(status?.memory.latestBackupPath)} />
      </div>
      <label className="ops-form-field">
        <span>手动记忆</span>
        <textarea
          className="ops-textarea compact"
          onChange={(event) => setDraft(event.currentTarget.value)}
          placeholder="输入要长期保存的项目约定、构建命令、偏好或修复结论"
          value={draft}
        />
      </label>
      <div className="action-row">
        <Button
          disabled={!draft.trim()}
          onClick={() => {
            void (async () => {
              if (await actions.learnMemoryAssistItem(draft)) setDraft("");
            })();
          }}
          size="sm"
        >
          <CheckCircle2 className="h-4 w-4" />
          记住
        </Button>
        <Button onClick={() => void actions.refreshMemoryAssist()} size="sm" variant="outline">
          <RefreshCw className="h-4 w-4" />
          刷新
        </Button>
        <Button onClick={() => void actions.runMemoryAssistSelfcheck()} size="sm" variant="outline">
          <ShieldCheck className="h-4 w-4" />
          自检并备份
        </Button>
      </div>
      <div className="memory-assist-search">
        <label className="ops-form-field">
          <span>搜索记忆</span>
          <input
            onChange={(event) => setSearchQuery(event.currentTarget.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && searchQuery.trim()) void actions.searchMemoryAssist(searchQuery);
            }}
            placeholder="搜索项目约定、构建命令、历史修复结论"
            value={searchQuery}
          />
        </label>
        <Button disabled={!searchQuery.trim()} onClick={() => void actions.searchMemoryAssist(searchQuery)} size="sm" variant="outline">
          <RefreshCw className="h-4 w-4" />
          搜索
        </Button>
      </div>
      {matches.length ? (
        <div className="memory-assist-list">
          <strong>搜索结果：{search?.memory.query}</strong>
          {matches.slice(0, 6).map((match) => (
            <div className="memory-assist-row" key={match.item.id}>
              <span>{match.item.category} · {match.item.workspace} · score {match.score.toFixed(2)}</span>
              <p>{match.item.text}</p>
              {match.matchedKeywords.length ? <em>命中：{match.matchedKeywords.slice(0, 8).join(" / ")}</em> : null}
            </div>
          ))}
        </div>
      ) : search ? <Empty text="没有匹配到记忆。" /> : null}
      {selfCheck ? (
        <div className="ops-note">
          <ShieldCheck className="h-4 w-4" />
          <span>{selfCheck.report.status} · {selfCheck.report.checks.map((check) => `${check.name}:${check.status}`).join(" / ")}</span>
        </div>
      ) : null}
      <div className="memory-assist-columns">
        <div className="memory-assist-list">
          <strong>最近记忆</strong>
          {recentItems.length ? recentItems.map((item) => (
            <div className="memory-assist-row" key={item.id}>
              <span>{item.category} · {item.workspace}</span>
              <p>{item.text}</p>
              <div className="action-row">
                <Button onClick={() => void actions.deleteMemoryAssistItem(item.id)} size="sm" variant="outline">删除</Button>
              </div>
            </div>
          )) : <Empty text="暂无长期记忆。" />}
        </div>
        <div className="memory-assist-list">
          <strong>待确认</strong>
          {pending.length ? pending.slice(0, 5).map((candidate) => (
            <div className="memory-assist-row" key={candidate.id}>
              <span>{candidate.category} · {candidate.source || "auto"}</span>
              <p>{candidate.text}</p>
              <div className="action-row">
                <Button onClick={() => void actions.approveMemoryAssistCandidate(candidate.id)} size="sm">确认</Button>
                <Button onClick={() => void actions.rejectMemoryAssistCandidate(candidate.id)} size="sm" variant="outline">忽略</Button>
              </div>
            </div>
          )) : <Empty text="暂无待确认记忆。" />}
        </div>
      </div>
      <div className="memory-assist-transfer">
        <div className="memory-assist-list">
          <strong>导出</strong>
          <div className="action-row">
            <Button onClick={() => void actions.exportMemoryAssist()} size="sm" variant="outline">
              <FileDown className="h-4 w-4" />
              生成导出 JSON
            </Button>
          </div>
          <textarea
            className="ops-textarea compact mono"
            placeholder="点击生成导出 JSON 后会显示完整迁移包。"
            readOnly
            value={exportJson}
          />
        </div>
        <div className="memory-assist-list">
          <strong>导入</strong>
          <textarea
            className="ops-textarea compact mono"
            onChange={(event) => setImportText(event.currentTarget.value)}
            placeholder="粘贴 memory-assist/v1 导出 JSON；导入前会再次确认。"
            value={importText}
          />
          <div className="ops-toggle-line">
            <span>替换现有记忆库</span>
            <ToggleSwitch checked={replaceExisting} onChange={setReplaceExisting} />
          </div>
          <Button disabled={!importText.trim()} onClick={() => void actions.importMemoryAssist(importText, replaceExisting)} size="sm">
            <FileUp className="h-4 w-4" />
            导入记忆
          </Button>
        </div>
      </div>
    </Panel>
  );
}

function SessionManagementScreen({
  actions,
  claudeChinese,
  claudeDesktop,
  localSessions,
  memoryAssist,
  memoryCandidates,
  memoryExport,
  memoryItems,
  memorySearch,
  memorySelfCheck,
  providerSync,
  settings,
}: {
  actions: ReturnType<typeof createActionsShape>;
  claudeChinese: ClaudeChineseWindowResult | null;
  claudeDesktop: ClaudeDesktopResult | null;
  localSessions: LocalSessionsResult | null;
  memoryAssist: MemoryStatusResult | null;
  memoryCandidates: MemoryCandidatesResult | null;
  memoryExport: MemoryExportResult | null;
  memoryItems: MemoryItemsResult | null;
  memorySearch: MemoryQueryResult | null;
  memorySelfCheck: MemorySelfCheckResult | null;
  providerSync: ProviderSyncResult | null;
  settings: SettingsResult | null;
}) {
  const sessions = localSessions?.sessions ?? [];
  const latestSessions = sessions.slice(0, 8);
  const syncSummary = providerSync
    ? `${providerSync.changedSessionFiles ?? 0} 个会话文件，${providerSync.sqliteRowsUpdated ?? 0} 行索引`
    : "尚未执行";

  return (
    <div className="stack">
      <Panel title="会话管理" detail="历史会话修复、盘古记忆、Codex 会话管理和 Claude 会话诊断集中在这里。">
        <div className="ops-note">
          <ShieldCheck className="h-4 w-4" />
          <span>会话相关动作会优先在这里刷新和核对，避免在工具页和会话页之间来回跳。</span>
        </div>
      </Panel>
      <div className="ops-two-column">
        <div className="ops-wide-column">
          <Panel title="历史会话修复" detail="用于修复切换供应商后 Codex 历史会话不可见或元数据不一致的问题。">
            <div className="ops-status-list">
              <StatusRow label="Provider Sync" status={settings?.settings.providerSyncEnabled ? "running" : "disabled"} value={settings?.settings.providerSyncEnabled ? "已开启" : "未开启"} />
              <StatusRow label="最近修复" status={providerSync ? "ok" : "not_checked"} value={syncSummary} />
              <StatusRow label="目标供应商" status={providerSync?.targetProvider ? "ok" : "not_checked"} value={providerSync?.targetProvider || settings?.settings.providerSyncLastSelectedProvider || "自动识别"} />
            </div>
            <div className="action-row">
              <Button onClick={() => void actions.repairHistorySessions()}>
                <Wrench className="h-4 w-4" />
                修复历史会话
              </Button>
              <Button onClick={() => void actions.refreshLocalSessions()} variant="outline">
                <RefreshCw className="h-4 w-4" />
                刷新会话
              </Button>
            </div>
            {providerSync?.encryptedContentWarning ? (
              <div className="ops-danger-zone">
                <AlertTriangle className="h-4 w-4" />
                <span>{providerSync.encryptedContentWarning}</span>
              </div>
            ) : null}
          </Panel>
          <Panel title="Codex 会话管理" detail={`${sessions.length} 个本地会话；删除会先写备份。`}>
            <div className="ops-status-list">
              <StatusRow label="数据库" status={localSessions?.dbPath ? "found" : "not_checked"} value={compactPath(localSessions?.dbPath)} />
              <StatusRow label="候选库" status={(localSessions?.dbPaths.length ?? 0) > 0 ? "found" : "not_checked"} value={`${localSessions?.dbPaths.length ?? 0} 个`} />
              <StatusRow label="会话数" status={sessions.length ? "ok" : "not_checked"} value={`${sessions.length} 个`} />
            </div>
            <div className="session-list">
              {latestSessions.length ? latestSessions.map((session) => (
                <div className="session-row" key={`${session.dbPath}:${session.id}`}>
                  <div>
                    <strong>{session.title || "未命名会话"}</strong>
                    <span>{session.modelProvider || "unknown"} · {compactPath(session.cwd || session.rolloutPath || session.id)}</span>
                  </div>
                  <Button onClick={() => void actions.deleteLocalSession(session)} size="sm" variant="outline">
                    <Trash2 className="h-4 w-4" />
                    删除
                  </Button>
                </div>
              )) : <Empty text="暂未读取到 Codex 本地会话。" />}
            </div>
          </Panel>
        </div>
        <div className="ops-wide-column">
          <MemoryAssistPanel
            actions={actions}
            candidates={memoryCandidates}
            exported={memoryExport}
            items={memoryItems}
            search={memorySearch}
            selfCheck={memorySelfCheck}
            status={memoryAssist}
          />
          <Panel title="Claude 会话诊断" detail="官方 Claude 历史会话不写入本工具可直接修复的本地 SQLite；这里提供可验证入口和包装窗口。">
            <div className="ops-status-list">
              <StatusRow label="官方 Claude" status={claudeDesktop?.status ?? "not_checked"} value={`${claudeDesktop?.installKind ?? "未检测"} / ${claudeDesktop?.cdpStatus ?? "unknown"}`} />
              <StatusRow label="Claude 一键汉化" status={claudeChinese?.open ? "ok" : "not_checked"} value={claudeChinese?.open ? "已打开" : "未打开"} />
              <StatusRow label="安全边界" status="ok" value="不修改官方 MSIX / app.asar" />
            </div>
            <div className="action-row">
              <Button onClick={() => void actions.launchClaudeDesktop()} variant="outline">启动/重启Claude</Button>
              <Button onClick={() => void actions.installClaudeZhPatch()} variant="outline">Claude 一键汉化</Button>
            </div>
          </Panel>
        </div>
      </div>
    </div>
  );
}

function PluginHubScreen({
  actions,
  devMode,
  hub,
  preview,
  orgPlugin,
  marketplace,
}: {
  actions: ReturnType<typeof createActionsShape>;
  devMode: ClaudeDesktopDevModeStatusResult | null;
  hub: PluginHubResult | null;
  preview: PluginInstallPreviewResult | null;
  orgPlugin: ClaudeDesktopOrgPluginStatusResult | null;
  marketplace: ClaudeDesktopMarketplaceStatusResult | null;
}) {
  const [filter, setFilter] = useState<"all" | "official" | "ponytail" | "codex" | "mcp" | "skill" | "installed" | "review">("all");
  const [selectedId, setSelectedId] = useState("");
  const items = hub?.catalog?.items ?? [];
  const visible = items.filter((item) => {
    if (filter === "official") return item.sourceId === "official";
    if (filter === "ponytail") return item.sourceId === "ponytail" || item.tags.includes("ponytail");
    if (filter === "codex") return item.sourceId === "codex-plugins" || item.category === "codex" || item.installKind === "codex_plugin" || item.tags.includes("codex");
    if (filter === "mcp") return item.installKind === "mcp_server" || item.installKind === "claude_desktop_mcp" || item.installKind === "claude_desktop_org_plugin";
    if (filter === "skill") return item.installKind === "skill_bundle" || item.installKind === "managed_skill_bundle";
    if (filter === "installed") return item.installStatus === "installed";
    if (filter === "review") return item.installStatus === "needsReview";
    return true;
  });
  const selected = items.find((item) => item.id === selectedId) ?? visible[0] ?? null;
  const selectedPreview = preview?.item.id === selected?.id ? preview : null;
  const selectedCanInstall = selected ? pluginCanInstall(selected.installKind) : false;
  const installButtonLabel = selected ? pluginInstallButtonLabel(selected.installKind) : "Install";
  return (
    <div className="stack">
      <div className="plugin-layout">
      <Panel title="插件目录" detail="Claude 插件、Codex 插件仓库、MCP Registry 与 awesome-claude-code 社区资源。">
        <div className="filter-row">
          {[
            ["all", "全部"],
            ["official", "官方插件"],
            ["codex", "Codex 插件"],
            ["mcp", "MCP"],
            ["skill", "Skills"],
            ["installed", "已安装"],
            ["review", "需审查"],
          ].map(([id, label]) => (
            <button className={filter === id ? "active" : ""} key={id} onClick={() => setFilter(id as typeof filter)} type="button">
              {label}
            </button>
          ))}
          <button className={filter === "ponytail" ? "active" : ""} onClick={() => setFilter("ponytail")} type="button">
            Ponytail
          </button>
          <Button onClick={() => void actions.refreshPluginHub()} size="sm" variant="outline">
            <RefreshCw className="h-4 w-4" />
            刷新
          </Button>
        </div>
        <div className="source-strip">
          {(hub?.catalog?.sources ?? []).map((source) => (
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
                  卸载
                </Button>
              ) : selectedCanInstall ? (
                <Button onClick={() => void actions.installPlugin(selected.id)}>
                  <Download className="h-4 w-4" />
                  <span className="desktop-install-label">{installButtonLabel}</span>
                </Button>
              ) : (
                <Button disabled variant="outline">
                  <ShieldCheck className="h-4 w-4" />
                  Review required
                </Button>
              )}
              {selected.id === "ponytail:codex-plugin" ? (
                <>
                  <Button onClick={() => void actions.previewPonytailCodexHooks()} variant="outline">
                    <ShieldCheck className="h-4 w-4" />
                    Review hooks
                  </Button>
                  <Button onClick={() => void actions.trustPonytailCodexHooks()} variant="outline">
                    <ShieldCheck className="h-4 w-4" />
                    Trust hooks
                  </Button>
                </>
              ) : null}
              {selected.id === "ponytail:claude-desktop-mcp" ? (
                <Button onClick={() => void actions.generatePonytailMcpbInstaller()} variant="outline">
                  <Download className="h-4 w-4" />
                  Generate MCPB
                </Button>
              ) : null}
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
    </div>
  );
}

function ClaudeDesktopOrgPluginPanel({
  actions,
  devMode,
  marketplace,
  status,
}: {
  actions: ReturnType<typeof createActionsShape>;
  devMode: ClaudeDesktopDevModeStatusResult | null;
  marketplace: ClaudeDesktopMarketplaceStatusResult | null;
  status: ClaudeDesktopOrgPluginStatusResult | null;
}) {
  const orgStatus = status?.orgPluginStatus;
  const marketStatus = marketplace?.marketplaceStatus;
  const devStatus = devMode?.devModeStatus;
  return (
    <Panel title="Claude Desktop 本地插件" detail="开发模式下直接写 MCP 配置和组织插件 skills 目录，不依赖 Claude CLI 登录。">
      <div className="info-grid compact">
        <InfoRow label="开发模式" value={devStatus?.configured ? "已配置" : "未配置"} />
        <InfoRow label="本地写入" value="MCP + skills + 组织插件目录" />
        <InfoRow label="官方仓库" value={`${marketStatus?.marketplace ?? "DietrichGebert/ponytail"}（可选）`} />
        <InfoRow label="组织目录" value={compactPath(orgStatus?.orgPluginsDir)} />
        <InfoRow label="Ponytail" value={orgStatus?.ponytailInstalled ? "已安装" : "未安装"} />
        <InfoRow label="目录可写" value={orgStatus?.writable ? "是" : "否"} />
      </div>
      <div className="risk-box">
        {devStatus?.message ?? "Claude Desktop 开发模式会写入本机 deploymentMode=3p 和 Claude-3p 配置库。"}
        {" "}
        {orgStatus?.message ?? "正在检测 Claude Desktop 组织插件目录。"}
      </div>
      <div className="action-row">
        <Button
          onClick={() => {
            void actions.refreshClaudeDesktopDevMode();
            void actions.refreshClaudeDesktopMarketplace();
            void actions.refreshClaudeDesktopOrgPlugin();
          }}
          variant="outline"
        >
          <RefreshCw className="h-4 w-4" />
          刷新状态
        </Button>
        <Button onClick={() => void actions.configureClaudeDesktopDevMode()}>
          <Wrench className="h-4 w-4" />
          Claude 一键开发模式
        </Button>
        <Button onClick={() => void actions.installPonytailClaudeDesktopLocalBundle()}>
          <Download className="h-4 w-4" />
          一键写入本地插件
        </Button>
        <Button onClick={() => void actions.installPonytailClaudeDesktopOrgPlugin()} variant="outline">
          <Download className="h-4 w-4" />
          仅写入组织插件
        </Button>
        <Button onClick={() => void actions.openClaudeDesktopOrgPluginsDir()} variant="outline">
          <ExternalLink className="h-4 w-4" />
          打开组织目录
        </Button>
        <Button onClick={() => void actions.openPonytailClaudeDesktopMarketplaceSetup()} variant="outline">
          <ExternalLink className="h-4 w-4" />
          可选官方仓库
        </Button>
        <Button onClick={() => void actions.openExternalUrl(PONYTAIL_REPOSITORY_URL)} variant="outline">
          <ExternalLink className="h-4 w-4" />
          Ponytail 源码
        </Button>
      </div>
    </Panel>
  );
}

function PromptOptimizerCard({ actions }: { actions: ReturnType<typeof createActionsShape> }) {
  return (
    <section className="ops-panel prompt-optimizer-card">
      <header>
        <div>
          <h2>提示词优化</h2>
          <p>把提示词优化放到 Codex/Claude 运维流旁边。</p>
        </div>
      </header>
      <div className="ops-panel-body">
        <Button className="prompt-optimizer-card-button" onClick={() => void actions.goPromptOptimizer()}>
          <PencilRuler className="h-4 w-4" />
          提示词优化
        </Button>
        <button
          className="prompt-optimizer-source-link"
          onClick={() => void actions.openExternalUrl("https://github.com/linshenkx/prompt-optimizer")}
          type="button"
        >
          <ExternalLink className="h-4 w-4" />
          linshenkx/prompt-optimizer
        </button>
      </div>
    </section>
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

function MaintenanceToolsPanel({
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
    <Panel title="入口与修复" detail="会话工具旁保留启动、快捷方式、后端和 Watcher 修复入口。">
      <div className="ops-status-list">
        <StatusRow label="Codex 应用" status={overview?.codex_app.status ?? "not_checked"} value={compactPath(overview?.codex_app.path)} />
        <StatusRow label="静默启动入口" status={overview?.silent_shortcut.status ?? "not_checked"} value={compactPath(overview?.silent_shortcut.path)} />
        <StatusRow label="管理工具入口" status={overview?.management_shortcut.status ?? "not_checked"} value={compactPath(overview?.management_shortcut.path)} />
        <StatusRow label="Watcher 自动接管" status={watcher?.enabled ? "running" : "disabled"} value={watcher?.enabled ? "已启用" : "未启用"} />
      </div>
      <div className="action-row">
        <Button onClick={() => void actions.launchCodex()} size="sm">启动/重启Codex</Button>
        <Button onClick={() => void actions.restartCodex()} size="sm" variant="outline">重启 Codex</Button>
        <Button onClick={() => void actions.repairEntrypoints()} size="sm" variant="outline">修复入口</Button>
        <Button onClick={() => void actions.repairBackend()} size="sm" variant="outline">修复后端</Button>
        <Button onClick={() => void actions.installEntrypoints()} size="sm" variant="outline">安装入口</Button>
        <Button onClick={() => void actions.repairShortcuts()} size="sm" variant="outline">修复快捷方式</Button>
      </div>
      <div className="action-row">
        <Button onClick={() => void actions.installWatcher()} size="sm" variant="outline">安装 Watcher</Button>
        <Button onClick={() => void actions.enableWatcher()} size="sm" variant="outline">启用 Watcher</Button>
        <Button onClick={() => void actions.disableWatcher()} size="sm" variant="outline">禁用 Watcher</Button>
        <Button onClick={() => void actions.uninstallWatcher()} size="sm" variant="outline">移除 Watcher</Button>
      </div>
      <div className="info-grid compact">
        <InfoRow label="设置文件" value={compactPath(settings?.settings_path)} />
        <InfoRow label="日志文件" value={compactPath(overview?.logs_path)} />
        <InfoRow label="当前版本" value={overview?.current_version ?? "未加载"} />
      </div>
    </Panel>
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
  claudeZhPatch,
  draft,
  onDraftChange,
  overview,
  settings,
  watcher,
}: {
  actions: ReturnType<typeof createActionsShape>;
  claudeChinese: ClaudeChineseWindowResult | null;
  claudeZhPatch: ClaudeZhPatchResult | null;
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
    ["盘古记忆", "memoryAssistEnabled"],
    ["盘古记忆 DOM 标识", "memoryAssistInjectEnabled"],
    ["待确认学习", "memoryAssistAutoSuggestEnabled"],
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
        <Panel title="Claude 一键汉化" detail="一键汉化目标是本机 zh-CN 资源补丁；MSIX/WindowsApps 不可写时会提示选择可写安装目录。">
          <div className="info-grid compact">
            <InfoRow label="本机汉化" value={claudeZhPatch?.status.status ?? "not_checked"} />
            <InfoRow label="安装类型" value={claudeZhPatch?.status.installKind ?? claudeChinese?.officialInstallKind ?? "未检测"} />
            <InfoRow label="补丁目标" value={compactPath(claudeZhPatch?.status.appRoot)} />
            <InfoRow label="目录可写" value={claudeZhPatch?.status.writable ? "是" : "否，需要管理员授权"} />
            <InfoRow label="备份目录" value={compactPath(claudeZhPatch?.backupDir)} />
            <InfoRow label="诊断日志" value={compactPath(claudeZhPatch?.logsPath)} />
            <InfoRow label="桌面资源" value={claudeZhPatch?.status.resourcesPresent ? "已写入" : "未写入"} />
            <InfoRow label="前端资源" value={claudeZhPatch?.status.frontendI18nPresent ? "已写入" : "未写入"} />
            <InfoRow label="Statsig 资源" value={claudeZhPatch?.status.statsigI18nPresent ? "已写入" : "未写入"} />
            <InfoRow label="Locale" value={claudeZhPatch?.status.localeConfigured ? "zh-CN" : "未设置"} />
            <InfoRow label="语言白名单" value={claudeZhPatch?.status.languageWhitelistPatched ? "已激活" : "未激活"} />
            <InfoRow label="Chunk 注入" value={claudeZhPatch?.status.chunkPatchPresent ? "已注入" : "未注入"} />
          </div>
          <div className="action-row">
            <Button onClick={() => void actions.installClaudeZhPatch()}>
              <Languages className="h-4 w-4" />
              Claude 一键汉化
            </Button>
            <Button onClick={() => void actions.installClaudeZhPatchFromDirectory()} variant="outline">
              <Languages className="h-4 w-4" />
              手动选择安装目录
            </Button>
            <Button onClick={() => void actions.restoreClaudeZhPatch()} variant="outline">
              <RefreshCw className="h-4 w-4" />
              恢复官方 Claude
            </Button>
            <Button onClick={() => void actions.launchClaudeDesktop()} variant="outline">
              <MessageCircle className="h-4 w-4" />
              启动/重启Claude
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
          <div className="info-grid compact">
            <InfoRow label="生效方式" value="保存后重建 Codex CLI Wrapper" />
            <InfoRow label="依赖" value="需要本机可执行 Codex CLI" />
          </div>
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
          <div className="info-grid compact">
            <InfoRow label="生效方式" value="保存后下次 Codex 注入生效" />
            <InfoRow label="启用条件" value={s?.codexAppImageOverlayPath?.trim() ? "已设置图片路径" : "路径为空，不会注入"} />
          </div>
          <div className="action-row">
            <Button disabled={!s} onClick={() => void saveDraft()} variant="outline">保存图片覆盖</Button>
            <Button onClick={() => void actions.resetImageOverlaySettings()} variant="outline">
              <RefreshCw className="h-4 w-4" />
              重置图片覆盖
            </Button>
          </div>
        </Panel>
        <Panel title="盘古记忆" detail="控制 Codex 页面顶部盘古记忆标识、会话摘要注入和待确认学习。">
          <div className="ops-toggle-line">
            <span>启用盘古记忆</span>
            <ToggleSwitch checked={Boolean(s?.memoryAssistEnabled)} disabled={!s} onChange={(value) => updateDraft("memoryAssistEnabled", value)} />
          </div>
          <div className="ops-toggle-line">
            <span>显示 DOM 注入标识</span>
            <ToggleSwitch checked={Boolean(s?.memoryAssistInjectEnabled)} disabled={!s} onChange={(value) => updateDraft("memoryAssistInjectEnabled", value)} />
          </div>
          <div className="ops-toggle-line">
            <span>启用待确认学习</span>
            <ToggleSwitch checked={Boolean(s?.memoryAssistAutoSuggestEnabled)} disabled={!s} onChange={(value) => updateDraft("memoryAssistAutoSuggestEnabled", value)} />
          </div>
          <label className="ops-form-field">
            <span>每次最多注入：{s?.memoryAssistMaxInjectedItems ?? 5} 条</span>
            <input disabled={!s} max={20} min={1} onChange={(event) => updateDraft("memoryAssistMaxInjectedItems", Number(event.currentTarget.value))} type="range" value={s?.memoryAssistMaxInjectedItems ?? 5} />
          </label>
          <div className="info-grid compact">
            <InfoRow label="工作区模式" value={s?.memoryAssistWorkspaceMode || "project_plus_global"} />
            <InfoRow label="存储位置" value="~/.claude-codex-pro/memory_assist.sqlite" />
          </div>
          <Button disabled={!s} onClick={() => void saveDraft()} variant="outline">保存盘古记忆设置</Button>
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

function StatusActionTile({ disabled, icon: Icon, label, value, status, onClick }: { disabled?: boolean; icon: LucideIcon; label: string; value: string; status: string; onClick: () => void }) {
  return (
    <button className={`status-tile status-action-tile ${statusOk(status) ? "ok" : "warn"}`} disabled={disabled} onClick={onClick} type="button">
      <Icon className="h-4 w-4" />
      <span>{label}</span>
      <strong>{value}</strong>
    </button>
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

const SUPPLIER_PRESETS: SupplierPreset[] = [
  {
    id: "openai",
    name: "OpenAI Official",
    category: "official",
    baseUrl: "https://api.openai.com/v1",
    protocol: "responses",
    model: "gpt-5.5",
    websiteUrl: "https://chatgpt.com/codex",
  },
  {
    id: "deepseek",
    name: "DeepSeek",
    category: "cn_official",
    baseUrl: "https://api.deepseek.com",
    protocol: "chatCompletions",
    model: "deepseek-v4-flash",
    modelList: ["deepseek-v4-flash", "deepseek-v4-pro"],
    apiKeyUrl: "https://platform.deepseek.com/api_keys",
  },
  {
    id: "kimi",
    name: "Kimi",
    category: "cn_official",
    baseUrl: "https://api.moonshot.cn/v1",
    protocol: "chatCompletions",
    model: "kimi-k2.6",
    modelList: ["kimi-k2.6"],
  },
  {
    id: "qwen",
    name: "Qwen / Bailian",
    category: "cn_official",
    baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
    protocol: "chatCompletions",
    model: "qwen3-coder-plus",
    modelList: ["qwen3-coder-plus", "qwen3-max"],
  },
  {
    id: "siliconflow",
    name: "SiliconFlow",
    category: "aggregator",
    baseUrl: "https://api.siliconflow.cn/v1",
    protocol: "chatCompletions",
    model: "Pro/MiniMaxAI/MiniMax-M2.7",
    modelList: ["Pro/MiniMaxAI/MiniMax-M2.7"],
  },
  {
    id: "openrouter",
    name: "OpenRouter",
    category: "aggregator",
    baseUrl: "https://openrouter.ai/api/v1",
    protocol: "chatCompletions",
    model: "openai/gpt-5.5",
  },
];

function supplierIdFromName(value: string) {
  const id = value.trim().toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "");
  return id || "provider";
}

function uniqueSupplierProfileId(profiles: RelayProfile[], base: string) {
  const root = supplierIdFromName(base);
  const existing = new Set(profiles.map((profile) => profile.id));
  if (!existing.has(root)) return root;
  for (let index = 2; index < 999; index += 1) {
    const candidate = `${root}-${index}`;
    if (!existing.has(candidate)) return candidate;
  }
  return `${root}-${Date.now().toString(36)}`;
}

function createSupplierProfile(settings: BackendSettings): RelayProfile {
  return normalizeSupplierProfile(withSupplierGeneratedFiles({
    id: uniqueSupplierProfileId(settings.relayProfiles, "provider"),
    name: `供应商 ${settings.relayProfiles.length + 1}`,
    model: "gpt-5.5",
    baseUrl: "",
    upstreamBaseUrl: "",
    apiKey: "",
    protocol: "responses",
    relayMode: "pureApi",
    officialMixApiKey: false,
    testModel: "gpt-5.5",
    configContents: "",
    authContents: "",
    useCommonConfig: true,
    contextSelection: { mcpServers: [], skills: [], plugins: [] },
    contextSelectionInitialized: false,
    contextWindow: "",
    autoCompactLimit: "",
    modelList: "gpt-5.5",
    userAgent: "",
  }));
}

function normalizeSupplierProfile(profile: RelayProfile): RelayProfile {
  const modelList = profile.modelList ?? "";
  const apiKey = profile.apiKey ?? "";
  const baseUrl = profile.baseUrl || profile.upstreamBaseUrl || "";
  const model = profile.model || profile.testModel || firstSupplierModel(modelList) || "gpt-5.5";
  return {
    ...profile,
    id: supplierIdFromName(profile.id || profile.name),
    name: profile.name || profile.id || "未命名供应商",
    model,
    testModel: profile.testModel || model,
    baseUrl,
    upstreamBaseUrl: profile.upstreamBaseUrl || baseUrl,
    apiKey,
    protocol: profile.protocol || "responses",
    relayMode: profile.relayMode === "official" ? "official" : "pureApi",
    officialMixApiKey: false,
    configContents: profile.configContents ?? "",
    authContents: profile.authContents ?? "",
    modelList: modelList || model,
    contextWindow: profile.contextWindow ?? "",
    autoCompactLimit: profile.autoCompactLimit ?? "",
    userAgent: profile.userAgent ?? "",
  };
}

function withSupplierGeneratedFiles(profile: RelayProfile): RelayProfile {
  const normalized = normalizeSupplierProfile({
    ...profile,
    configContents: "",
    authContents: "",
  });
  return {
    ...normalized,
    configContents: buildSupplierConfigToml(normalized),
    authContents: `${JSON.stringify({ OPENAI_API_KEY: normalized.apiKey.trim() }, null, 2)}\n`,
  };
}

function buildSupplierConfigToml(profile: RelayProfile) {
  const model = profile.model.trim();
  const baseUrl = profile.baseUrl.trim();
  const providerId = supplierIdFromName(profile.id || profile.name);
  return [
    model ? `model = ${tomlString(model)}` : null,
    `model_provider = ${tomlString(providerId)}`,
    'model_reasoning_effort = "high"',
    "disable_response_storage = true",
    "",
    `[model_providers.${providerId}]`,
    `name = ${tomlString(providerId)}`,
    'wire_api = "responses"',
    "requires_openai_auth = true",
    baseUrl ? `base_url = ${tomlString(baseUrl)}` : null,
    "",
  ].filter((line): line is string => line !== null).join("\n");
}

function tomlString(value: string) {
  return JSON.stringify(value);
}

function firstSupplierModel(modelList: string) {
  return modelList.split(/\r?\n/).map((item) => item.trim()).find(Boolean) || "";
}

function redactSupplierAuth(contents: string) {
  try {
    const parsed = JSON.parse(contents || "{}") as Record<string, unknown>;
    if (typeof parsed.OPENAI_API_KEY === "string" && parsed.OPENAI_API_KEY) {
      parsed.OPENAI_API_KEY = `${parsed.OPENAI_API_KEY.slice(0, 6)}...${parsed.OPENAI_API_KEY.slice(-4)}`;
    }
    return `${JSON.stringify(parsed, null, 2)}\n`;
  } catch {
    return "{\n  \"OPENAI_API_KEY\": \"***\"\n}\n";
  }
}

function supplierCategoryLabel(category: SupplierPreset["category"]) {
  const labels: Record<SupplierPreset["category"], string> = {
    official: "官方",
    cn_official: "国内官方",
    aggregator: "聚合/中转",
    third_party: "第三方",
  };
  return labels[category];
}

function supplierProtocolLabel(protocol?: string) {
  return protocol === "chatCompletions" ? "Chat Completions" : "Responses";
}

function supplierRelayModeLabel(mode?: string) {
  if (mode === "official") return "官方登录";
  if (mode === "mixedApi") return "官方混入 API Key";
  return "纯 API";
}

function Notice({ notice, onClose }: { notice: { title: string; message: string; status?: Status }; onClose: () => void }) {
  const ok = statusOk(notice.status);
  const running = notice.status === "running";
  return (
    <div className="toast-wrap" role="status" aria-live={ok ? "polite" : "assertive"}>
      <div className={`${ok ? "toast-card" : "toast-card failed"}${running ? " running" : ""}`}>
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
  if (kind === "claude_desktop_org_plugin") return "Claude Desktop 组织插件";
  if (kind === "claude_code_plugin") return "Claude Code 插件";
  if (kind === "codex_plugin") return "Codex 插件";
  if (kind === "copilot_plugin") return "GitHub Copilot CLI 插件";
  if (kind === "managed_skill_bundle") return "托管 Skill Bundle";
  if (kind === "claude_plugin_marketplace") return "Claude Code 插件";
  const labels: Partial<Record<PluginInstallKind, string>> = {
    claude_plugin_marketplace: "Claude 插件",
    mcp_server: "MCP 服务器",
    skill_bundle: "Skill Bundle",
    resource_link: "资源链接",
  };
  return labels[kind] ?? kind;
}

function pluginCanInstall(kind: PluginInstallKind) {
  return [
    "claude_desktop_mcp",
    "claude_desktop_org_plugin",
    "claude_plugin_marketplace",
    "claude_code_plugin",
    "codex_plugin",
    "copilot_plugin",
    "managed_skill_bundle",
  ].includes(kind);
}

function pluginInstallButtonLabel(kind: PluginInstallKind) {
  const labels: Partial<Record<PluginInstallKind, string>> = {
    claude_desktop_mcp: "Install to Claude Desktop",
    claude_desktop_org_plugin: "Install to Claude Desktop",
    claude_plugin_marketplace: "Install with Claude CLI",
    claude_code_plugin: "Install to Claude Code",
    codex_plugin: "Install to Codex",
    copilot_plugin: "Install to Copilot CLI",
    managed_skill_bundle: "Install Skills",
  };
  return labels[kind] ?? "Install";
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
  const injectedRoute = normalizeRoute(window.__CLAUDE_CODEX_PRO_INITIAL_ROUTE);
  if (routes.some((item) => item.id === injectedRoute)) return injectedRoute as Route;
  try {
    const view = normalizeRoute(new URLSearchParams(window.location.search).get("view"));
    if (routes.some((item) => item.id === view)) return view as Route;
  } catch {
    // Fall back to overview when running outside a normal browser URL.
  }
  return "overview";
}

function normalizeRoute(value: unknown): unknown {
  if (value === "pluginHub" || value === "context" || value === "maintenance" || value === "promptOptimizer") return "tools";
  if (value === "relay") return "supplier";
  return value;
}

function routeSubtitle(route: Route) {
  const subtitles: Record<Route, string> = {
    overview: "运行状态、启动动作和 Claude 一键汉化诊断。",
    supplier: "Codex 中转配置与 Claude Desktop 开发模式供应商写入。",
    tools: "插件目录、MCP 配置和启动入口。",
    sessions: "历史会话修复、盘古记忆和会话诊断。",
    scripts: "Codex 前端用户脚本市场。",
    logs: "诊断日志与运行信息。",
    settings: "全局开关和配置摘要。",
  };
  return subtitles[route];
}

function routeDocumentTitle(route: Route) {
  return route === "overview" ? "Claude Codex Pro 管理工具" : `${routeLabel(route)} - Claude Codex Pro 管理工具`;
}

function stringifyError(error: unknown) {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return JSON.stringify(error);
}

function waitForPaint() {
  return new Promise<void>((resolve) => {
    window.requestAnimationFrame(() => window.requestAnimationFrame(() => resolve()));
  });
}

function createActionsShape() {
  return {
    refreshRoute: async (_route?: Route) => {},
    showNotice: (_notice: { title: string; message: string; status?: Status } | null) => {},
    openClaudeChinese: async () => {},
    installClaudeZhPatch: async () => {},
    installClaudeZhPatchFromDirectory: async () => {},
    restoreClaudeZhPatch: async () => {},
    launchClaudeDesktop: async () => {},
    launchCodex: async () => {},
    restartCodex: async () => {},
    openExternalUrl: async (_url: string) => {},
    goPluginHub: async () => {},
    goPromptOptimizer: async () => {},
    previewPlugin: async (_id: string) => null as PluginInstallPreviewResult | null,
    installPlugin: async (_id: string) => {},
    uninstallPlugin: async (_id: string) => {},
    previewPonytailCodexHooks: async () => null as CodexHookTrustResult | null,
    trustPonytailCodexHooks: async () => {},
    generatePonytailMcpbInstaller: async () => {},
    installPonytailClaudeDesktopOrgPlugin: async () => {},
    installPonytailClaudeDesktopLocalBundle: async () => {},
    openClaudeDesktopOrgPluginsDir: async () => {},
    openPonytailClaudeDesktopMarketplaceSetup: async () => {},
    configureClaudeDesktopDevMode: async () => {},
    installMarketScript: async (_id: string) => {},
    refreshCodexPluginMarketplace: async () => null as CodexPluginMarketplaceStatusResult | null,
    repairCodexPluginMarketplace: async () => {},
    refreshPluginHub: async () => null as PluginHubResult | null,
    refreshClaudeDesktopOrgPlugin: async () => null as ClaudeDesktopOrgPluginStatusResult | null,
    refreshClaudeDesktopMarketplace: async () => null as ClaudeDesktopMarketplaceStatusResult | null,
    refreshClaudeDesktopDevMode: async () => null as ClaudeDesktopDevModeStatusResult | null,
    refreshScripts: async () => null as ScriptMarketResult | null,
    repairEntrypoints: async () => {},
    repairBackend: async () => {},
    repairHistorySessions: async () => {},
    refreshLocalSessions: async () => null as LocalSessionsResult | null,
    deleteLocalSession: async (_session: LocalSession) => {},
    refreshMemoryAssist: async () => null as MemoryStatusResult | null,
    learnMemoryAssistItem: async (_text: string, _category?: string) => false,
    searchMemoryAssist: async (_query: string) => {},
    deleteMemoryAssistItem: async (_id: string) => {},
    approveMemoryAssistCandidate: async (_id: string) => {},
    rejectMemoryAssistCandidate: async (_id: string) => {},
    runMemoryAssistSelfcheck: async () => {},
    exportMemoryAssist: async () => {},
    importMemoryAssist: async (_jsonText: string, _replaceExisting: boolean) => {},
    applyRelayMode: async () => {},
    applyPureApiMode: async () => {},
    clearRelayMode: async () => {},
    switchCodexRelayProfile: async (_profileId: string, _settings?: BackendSettings) => {},
    fetchRelayProfileModels: async (_profile: RelayProfile) => null as RelayProfileModelsResult | null,
    previewClaudeDesktopProvider: async (_request: { name: string; baseUrl: string; apiKey: string; modelList: string }) => {},
    applyClaudeDesktopProvider: async (_request: { name: string; baseUrl: string; apiKey: string; modelList: string }) => {},
    restoreClaudeDesktopProviderOfficial: async () => {},
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
  };
}
