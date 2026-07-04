import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  Download,
  FileDown,
  FileUp,
  ExternalLink,
  FileCode2,
  GripVertical,
  Info,
  KeyRound,
  Languages,
  LayoutDashboard,
  MessageCircle,
  MessageSquare,
  Network,
  PackageSearch,
  Pencil,
  PencilRuler,
  Pin,
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
import { type Dispatch, type DragEvent, type SetStateAction, memo, useCallback, useEffect, useMemo, useRef, useState } from "react";

import { Button } from "@/components/ui/button";
import { open } from "@tauri-apps/plugin-dialog";
import { invokeCommand } from "@/tauriBridge";

const PONYTAIL_REPOSITORY_URL = "https://github.com/DietrichGebert/ponytail";
const CODEX_THIRD_PARTY_PLUGIN_MARKETPLACE_NAME = "awesome-codex-plugins";
const CODEX_THIRD_PARTY_PLUGIN_REPOSITORY_URL =
  "https://github.com/hashgraph-online/awesome-codex-plugins.git";
const CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_NAME = "codex-skills-alternative";
const CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_SOURCE =
  "https://github.com/DKeken/codex-skills-alternative";
const CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_LOCAL_SOURCE =
  "~\\.codex\\plugins\\cache\\codex-skills-alternative-marketplace";
const PLUGIN_REPOSITORY_REPAIR_PROMPT_KEY_PREFIX = "tools-plugin-repository-repair";
const SUPPLIER_DRAG_MIME_TYPE = "application/x-claude-codex-pro-supplier-id";

type Status = "ok" | "failed" | "not_implemented" | "not_checked" | string;

type CommandResult<T> = T & {
  status: Status;
  message: string;
};

type StatusChipTone = "ok" | "warn" | "muted";
type StatusChip = {
  label: string;
  tone: StatusChipTone;
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
  debug_port_online: boolean;
  helper_port_online: boolean;
  frontend_runtime_online: boolean;
  frontend_runtime_seen_at_ms: number | null;
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
  inspectorPorts: number[];
  listeningPorts: number[];
  debugEvidence: string[];
  supportedIntegration: string;
  integrityStatus: string;
  integrityMessage: string;
  executableAudits: Array<Record<string, unknown>>;
}>;

type RepairConnectionResult = CommandResult<{
  target: string;
  frontendInjected: boolean;
  backendOnline: boolean;
  codexFrontendInjected: boolean;
  codexBackendOnline: boolean;
  claudeBackendOnline: boolean;
  debugPort: number | null;
  helperPort: number | null;
  claudeProxyPort: number | null;
  details: string[];
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
  aggregateEnabled?: boolean;
  aggregateStrategy?: string;
  aggregateMembers?: string[];
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

type CcswitchImportResult = CommandResult<{
  dbPath: string;
  profiles: RelayProfile[];
  scanned: number;
}>;

type ContextKind = "mcp" | "skill" | "plugin";

type ContextEntry = {
  id: string;
  kind: string;
  title: string;
  summary: string;
  tomlBody: string;
  enabled: boolean;
};

type ContextEntries = {
  mcpServers: ContextEntry[];
  skills: ContextEntry[];
  plugins: ContextEntry[];
};

type ContextEntriesResult = CommandResult<{
  settings: BackendSettings;
  entries: ContextEntries;
}>;

type LiveContextEntriesResult = CommandResult<{
  entries: ContextEntries;
}>;

type ClaudeContextEntriesResult = CommandResult<{
  configPath: string;
  entries: ContextEntries;
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

type AggregateStrategy = {
  id: string;
  label: string;
  detail: string;
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

type UpdateReleasePayload = {
  version: string;
  url: string;
  body: string;
  asset_name: string | null;
  asset_url: string | null;
};

type UpdateResult = CommandResult<{
  currentVersion: string;
  latestVersion?: string | null;
  releaseSummary?: string;
  assetName?: string | null;
  assetUrl?: string | null;
  updateAvailable?: boolean;
  progress?: number;
  installedPath?: string;
  launched?: boolean;
}>;

type CodexPluginMarketplaceStatus = {
  codexHome: string;
  marketplaceRoot: string | null;
  configRegistered: boolean;
  needsRepair: boolean;
  message: string;
  repositories?: Array<{
    label: string;
    name: string;
    sourceType: string;
    source: string;
    configured: boolean;
  }>;
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

type LocalSessionProjectGroup = {
  key: string;
  label: string;
  subtitle: string;
  sessions: LocalSession[];
};

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

type MemoryItemEditRequest = Pick<MemoryItem, "text" | "workspace" | "category" | "tags" | "source" | "sourceSessionId">;

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
    enabled: boolean;
    injectEnabled: boolean;
    autoSuggestEnabled: boolean;
    runtimeStatus: string;
    runtimeMessage: string;
    codexInjected: boolean;
    claudeInjected: boolean;
    codexWorkspace: string;
    active: boolean;
    activeSource: string;
    dbPath: string;
    totalItems: number;
    pendingCandidates: number;
    totalCaptures: number;
    workspaces: Array<{ workspace: string; itemCount: number; pendingCount: number; captureCount: number; sessionCount: number; latestCaptureAt: number }>;
    latestBackupPath: string | null;
  };
}>;

type MemoryItemsResult = CommandResult<{ items: MemoryItem[] }>;
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
    configPath: string;
    repositories: Array<{
      label: string;
      repository: string;
      url: string;
      configured: boolean;
    }>;
    message: string;
  };
}>;

type ClaudeDesktopMarketplaceOpenResult = CommandResult<{
  outcome: {
    repaired: boolean;
    configPath: string;
    repositories: ClaudeDesktopMarketplaceStatusResult["marketplaceStatus"]["repositories"];
    message: string;
  };
  marketplaceStatus: ClaudeDesktopMarketplaceStatusResult["marketplaceStatus"];
}>;

type ClaudeDesktopMarketplaceRepairResult = ClaudeDesktopMarketplaceOpenResult;

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
  | "maintenance"
  | "settings"
  | "about";
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
  { id: "maintenance", label: "维护", icon: Wrench },
  { id: "settings", label: "设置", icon: Settings },
  { id: "about", label: "关于", icon: Info },
];

function isRoute(value: unknown): value is Route {
  return routes.some((item) => item.id === value);
}

function statusOk(status?: string | null) {
  return status === "ok" || status === "accepted" || status === "found" || status === "installed" || status === "running" || status === "idle";
}

function statusFailed(status?: string | null) {
  return status === "failed" || status === "not_implemented";
}

function compactPath(path?: string | null) {
  if (!path) return "未设置";
  if (path.length <= 58) return path;
  return `${path.slice(0, 24)}...${path.slice(-28)}`;
}

function pathTail(path?: string | null) {
  const value = (path || "").trim().replace(/[\\/]+$/, "");
  if (!value) return "";
  const parts = value.split(/[\\/]+/).filter(Boolean);
  return parts.at(-1) || value;
}

function localSessionProjectLabel(session: LocalSession) {
  return pathTail(session.cwd) || pathTail(session.rolloutPath) || pathTail(session.dbPath) || "未归类项目";
}

function formatSessionRelativeTime(updatedAtMs?: number | null) {
  if (!updatedAtMs) return "未知";
  const diffMs = Math.max(0, Date.now() - updatedAtMs);
  const minute = 60 * 1000;
  const hour = 60 * minute;
  const day = 24 * hour;
  const week = 7 * day;
  if (diffMs < minute) return "刚刚";
  if (diffMs < hour) return `${Math.max(1, Math.floor(diffMs / minute))} 分钟`;
  if (diffMs < day) return `${Math.max(1, Math.floor(diffMs / hour))} 小时`;
  if (diffMs < week) return `${Math.max(1, Math.floor(diffMs / day))} 天`;
  return `${Math.max(1, Math.floor(diffMs / week))} 周`;
}

function groupLocalSessionsByProject(sessions: LocalSession[]) {
  const groups = new Map<string, LocalSessionProjectGroup>();
  for (const session of sessions) {
    const label = localSessionProjectLabel(session);
    const key = (session.cwd || session.rolloutPath || session.dbPath || label).toLowerCase();
    const group = groups.get(key) ?? {
      key,
      label,
      subtitle: session.cwd || session.rolloutPath || session.dbPath || "",
      sessions: [],
    };
    group.sessions.push(session);
    groups.set(key, group);
  }
  return Array.from(groups.values())
    .map((group) => ({
      ...group,
      sessions: group.sessions
        .slice()
        .sort((left, right) => (right.updatedAtMs ?? 0) - (left.updatedAtMs ?? 0)),
    }))
    .sort((left, right) => (right.sessions[0]?.updatedAtMs ?? 0) - (left.sessions[0]?.updatedAtMs ?? 0));
}

function codexPluginMarketplaceNeedsRepair(result?: CodexPluginMarketplaceStatusResult | null) {
  const status = result?.marketplace;
  return Boolean(result && (status?.needsRepair || !status?.configRegistered || !status?.marketplaceRoot));
}

function claudeDesktopMarketplaceNeedsRepair(result?: ClaudeDesktopMarketplaceStatusResult | null) {
  const status = result?.marketplaceStatus;
  const repositories = status?.repositories ?? [];
  return Boolean(
    result &&
      status?.canAutoWrite &&
      (repositories.length === 0 || repositories.some((repository) => !repository.configured)),
  );
}

function pluginRepositoryRepairPromptKey(
  codex: CodexPluginMarketplaceStatusResult | null,
  claude: ClaudeDesktopMarketplaceStatusResult | null,
) {
  const codexStatus = codex?.marketplace;
  const claudeStatus = claude?.marketplaceStatus;
  const claudeMissing = (claudeStatus?.repositories ?? [])
    .filter((repository) => !repository.configured)
    .map((repository) => repository.repository)
    .join(",");
  return [
    PLUGIN_REPOSITORY_REPAIR_PROMPT_KEY_PREFIX,
    codexStatus?.needsRepair ? "codex-needs-repair" : "codex-ok",
    codexStatus?.marketplaceRoot ?? "codex-no-root",
    codexStatus?.configRegistered ? "codex-registered" : "codex-unregistered",
    claudeMissing || "claude-ok",
    claudeStatus?.configPath ?? "claude-no-config",
  ].join("|");
}

function pluginRepositoryRepairPromptMessage(
  codex: CodexPluginMarketplaceStatusResult | null,
  claude: ClaudeDesktopMarketplaceStatusResult | null,
) {
  const items: string[] = [];
  if (codexPluginMarketplaceNeedsRepair(codex)) {
    items.push("Codex OpenAI 插件仓库未下载、未注册或配置异常。");
  }
  if (claudeDesktopMarketplaceNeedsRepair(claude)) {
    const missing = (claude?.marketplaceStatus.repositories ?? [])
      .filter((repository) => !repository.configured)
      .map((repository) => repository.label || repository.repository)
      .join("、");
    items.push(`Claude 插件仓库配置缺失${missing ? `：${missing}` : ""}。`);
  }
  return `检测到插件仓库配置异常，是否立即修复？\n\n${items.join("\n")}`;
}

function displayProductPath(path?: string | null) {
  if (!path) return "";
  return path
    .replace(/Codex\+\+/g, "Claude Code Pro")
    .replace(/Claude Codex Pro 管理工具/g, "Claude Code Pro 管理工具")
    .replace(/Claude Codex Pro/g, "Claude Code Pro");
}

function compactDisplayPath(path?: string | null) {
  const display = displayProductPath(path);
  return display ? compactPath(display) : "未设置";
}

function codexOverviewStatus(overview: OverviewResult | null) {
  const launch = overview?.latest_launch;
  const launchStatus = launch?.status ?? "not_checked";
  const running = launchStatus === "running" || launchStatus === "degraded";
  const failed = statusFailed(launchStatus);
  const frontendOnline = Boolean(launch?.frontend_runtime_online || launch?.debug_port_online);
  const backendOnline = Boolean(launch?.helper_port_online);
  const items: StatusChip[] = [
    { label: failed ? "运行异常" : running ? "运行中" : "未运行", tone: failed ? "warn" : running ? "ok" : "muted" },
    { label: launchStatus === "degraded" || failed ? "注入异常" : running ? "注入成功" : "未注入", tone: launchStatus === "degraded" || failed ? "warn" : running ? "ok" : "muted" },
    { label: frontendOnline ? "前端在线" : launch?.debug_port ? "CDP 离线" : "前端未检测", tone: frontendOnline ? "ok" : running ? "warn" : "muted" },
    { label: backendOnline ? "后端在线" : "后端离线", tone: backendOnline ? "ok" : running ? "warn" : "muted" },
  ];
  const status = items.some((item) => item.tone === "warn") ? "failed" : running ? "running" : "not_checked";
  return { status, items };
}

function claudeOverviewStatus(claudeDesktop: ClaudeDesktopResult | null, claudeZhPatch: ClaudeZhPatchResult | null) {
  const hasProcess = (claudeDesktop?.processCount ?? 0) > 0;
  const cdpStatus = claudeDesktop?.cdpStatus ?? "not_checked";
  const detectFailed = !!claudeDesktop && statusFailed(claudeDesktop.status);
  const injected = !!claudeZhPatch?.status.localeConfigured && !!claudeZhPatch?.status.frontendI18nPresent && !!claudeZhPatch?.status.chunkPatchPresent;
  const inspectorReady = cdpStatus === "node_inspector_ready" || (claudeDesktop?.inspectorPorts?.length ?? 0) > 0;
  const cdpWarn = !inspectorReady && cdpStatus === "failed";
  const items: StatusChip[] = [
    { label: detectFailed ? "检测异常" : hasProcess ? "运行中" : "未运行", tone: detectFailed ? "warn" : hasProcess ? "ok" : "muted" },
    { label: injected ? "汉化已注入" : "汉化未注入", tone: injected ? "ok" : "muted" },
  ];
  if (inspectorReady || cdpStatus === "ok" || cdpStatus === "failed") {
    items.push({
      label: inspectorReady ? "Inspector 在线" : cdpStatus === "failed" ? "CDP 异常" : "CDP 在线",
      tone: cdpWarn ? "warn" : "ok",
    });
  }
  const status = detectFailed ? "failed" : hasProcess ? "running" : "not_checked";
  return { status, items };
}

function memoryOverviewStatus(memoryAssist: MemoryStatusResult | null, settings: BackendSettings | null) {
  const memory = memoryAssist?.memory;
  const enabled = memory?.enabled ?? Boolean(settings?.memoryAssistEnabled);
  const injectEnabled = memory?.injectEnabled ?? Boolean(settings?.memoryAssistInjectEnabled);
  const autoSuggest = memory?.autoSuggestEnabled ?? Boolean(settings?.memoryAssistAutoSuggestEnabled);
  const healthy = memory?.status === "ok";
  const hasDb = Boolean(memory?.dbPath);
  const runtimeStatus = memory?.runtimeStatus ?? "not_checked";
  const codexInjected = Boolean(memory?.codexInjected);
  const listening = Boolean(memory?.active);
  const items: StatusChip[] = [
    { label: enabled ? "开关已开启" : "开关已关闭", tone: enabled ? "ok" : "muted" },
    { label: healthy ? "运行正常" : memoryAssist ? "运行异常" : "未检测", tone: healthy ? "ok" : memoryAssist ? "warn" : "muted" },
    { label: codexInjected ? "Codex 已注入" : injectEnabled ? "等待 Codex 注入" : "注入已关闭", tone: codexInjected ? "ok" : enabled && injectEnabled ? "warn" : "muted" },
    { label: listening ? "对话监控运行中" : codexInjected && autoSuggest ? "等待会话变化" : autoSuggest ? "等待 Codex 注入" : "对话监控关闭", tone: listening || (codexInjected && autoSuggest) ? "ok" : enabled && autoSuggest ? "warn" : "muted" },
    { label: hasDb ? "数据库在线" : "数据库未检测", tone: hasDb ? "ok" : enabled ? "warn" : "muted" },
  ];
  const status = items.some((item) => item.tone === "warn")
    ? "failed"
    : enabled && healthy && (codexInjected || runtimeStatus === "disabled")
      ? "running"
      : "not_checked";
  return { status, items };
}

function codexLaunchRequestFromOverview(overview: OverviewResult | null) {
  return {
    appPath: overview?.codex_app.path || overview?.latest_launch?.codex_app || "",
  };
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

function afterFirstPaint(task: () => void, delayMs = 0) {
  window.requestAnimationFrame(() => {
    window.requestAnimationFrame(() => {
      window.setTimeout(task, delayMs);
    });
  });
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
  const [memorySelfCheck, setMemorySelfCheck] = useState<MemorySelfCheckResult | null>(null);
  const [memorySearch, setMemorySearch] = useState<MemoryQueryResult | null>(null);
  const [memoryExport, setMemoryExport] = useState<MemoryExportResult | null>(null);
  const [providerSync, setProviderSync] = useState<ProviderSyncResult | null>(null);
  const [logs, setLogs] = useState<LogsResult | null>(null);
  const [watcher, setWatcher] = useState<WatcherResult | null>(null);
  const [updateInfo, setUpdateInfo] = useState<UpdateResult | null>(null);
  const [codexContextEntries, setCodexContextEntries] = useState<ContextEntriesResult | null>(null);
  const [liveCodexContextEntries, setLiveCodexContextEntries] = useState<LiveContextEntriesResult | null>(null);
  const [claudeContextEntries, setClaudeContextEntries] = useState<ClaudeContextEntriesResult | null>(null);
  const codexMarketplaceAutoRegisterRef = useRef(false);
  const pluginRepositoryRepairPromptKeyRef = useRef<string | null>(null);
  // Monotonic token bumped on every refreshRoute call. Rapid tab switches used
  // to let a slow route's trailing work (e.g. the plugin-repair prompt) fire
  // after the user had already navigated away; capturing the token and checking
  // it before the trailing side-effect discards stale route loads.
  const routeLoadEpochRef = useRef(0);

  const call = <T,>(command: string, args?: Record<string, unknown>) => invokeCommand<T>(command, args);
  const notifyIfNeedsAttention = (next: { title: string; message: string; status?: Status }) => {
    if (!statusOk(next.status)) setNotice(next);
  };
  // 用户主动动作（启动/修复/保存/删除/安装等）统一走这里：无论成功或失败都在右下角给出反馈。
  // 数据加载/切页面刷新仍用 notifyIfNeedsAttention，成功时保持静默，避免频繁弹出。
  const notifyResult = (next: { title: string; message?: string; status?: Status }) => {
    const message = next.message?.trim()
      ? next.message
      : statusOk(next.status)
        ? "操作完成。"
        : "操作未成功，请检查状态或日志。";
    setNotice({ title: next.title, message, status: next.status });
  };

  const run = async <T,>(
    task: () => Promise<T>,
    title?: string,
    options: { trackBusy?: boolean; notify?: boolean } = {},
  ): Promise<T | null> => {
    const trackBusy = options.trackBusy !== false;
    const notify = options.notify !== false;
    const actionTitle = title || "未命名操作";
    if (trackBusy) setBusyCount((count) => count + 1);
    void writeUiEvent("manager.ui.action.start", { title: actionTitle, trackBusy, notify });
    try {
      const result = await task();
      const resultRecord = result && typeof result === "object" ? (result as Record<string, unknown>) : {};
      void writeUiEvent("manager.ui.action.result", {
        title: actionTitle,
        status: typeof resultRecord.status === "string" ? resultRecord.status : "ok",
        message: typeof resultRecord.message === "string" ? resultRecord.message : "",
      });
      return result;
    } catch (error) {
      const message = stringifyError(error);
      void writeUiEvent("manager.ui.action.failed", { title: actionTitle, message });
      if (notify) setNotice({ title: title || "调用失败", message, status: "failed" });
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

  const refreshClaudeLight = async (silent = false) => {
    const [desktop, wrapped] = await Promise.all([
      run(() => call<ClaudeDesktopResult>("load_claude_desktop_status_light"), "Claude Desktop", { trackBusy: !silent, notify: !silent }),
      run(() => call<ClaudeChineseWindowResult>("load_claude_chinese_window_status"), "Claude 一键汉化", { trackBusy: !silent, notify: !silent }),
    ]);
    if (desktop) setClaudeDesktop(desktop);
    if (wrapped) setClaudeChinese(wrapped);
    if (!silent && desktop) notifyIfNeedsAttention({ title: "Claude Desktop", message: desktop.message, status: desktop.status });
  };

  const refreshClaudeZhPatch = async (silent = false) => {
    const result = await run(() => call<ClaudeZhPatchResult>("load_claude_zh_patch_status"), "Claude 本机汉化", { trackBusy: !silent, notify: !silent });
    if (result) setClaudeZhPatch(result);
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
    if (!silent) {
      setNotice({ title: "刷新 Claude 插件仓库", message: "正在检测 Claude 插件仓库配置...", status: "running" });
      await waitForPaint();
    }
    const result = await run(() => call<ClaudeDesktopMarketplaceStatusResult>("load_claude_desktop_marketplace_status"), "Claude Desktop 插件仓库", { trackBusy: !silent, notify: !silent });
    if (result) {
      setClaudeDesktopMarketplace(result);
      if (!silent) setNotice({ title: "刷新 Claude 插件仓库", message: result.message, status: result.status });
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
    if (!silent) {
      setNotice({ title: "刷新 Codex 插件仓库", message: "正在检测 Codex OpenAI 插件仓库配置...", status: "running" });
      await waitForPaint();
    }
    const result = await run(() => call<CodexPluginMarketplaceStatusResult>("load_codex_plugin_marketplace_status"), "Codex OpenAI 插件仓库", { trackBusy: !silent, notify: !silent });
    if (result) {
      setCodexPluginMarketplace(result);
      if (!silent) setNotice({ title: "刷新 Codex 插件仓库", message: result.message, status: result.status });
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
    const [status, items] = await Promise.all([
      refreshMemoryAssistStatus(silent),
      run(() => call<MemoryItemsResult>("list_memory_assist_items", { request: { workspace: MEMORY_ALL_WORKSPACES, includeGlobal: true, limit: 80 } }), "记忆列表", { trackBusy: !silent, notify: !silent }),
    ]);
    if (items) setMemoryItems(items);
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

  const refreshContextEntries = async (silent = false, sourceSettings = settings?.settings ?? settingsDraft) => {
    if (!sourceSettings) return null;
    const [managed, live] = await Promise.all([
      run(() => call<ContextEntriesResult>("list_context_entries", { request: { settings: sourceSettings } }), "Codex 工具与插件", { trackBusy: !silent, notify: !silent }),
      run(() => call<LiveContextEntriesResult>("read_live_context_entries"), "Codex 当前工具与插件", { trackBusy: !silent, notify: !silent }),
    ]);
    if (managed) {
      setCodexContextEntries(managed);
      setSettingsDraft(managed.settings);
      if (!silent) notifyIfNeedsAttention({ title: "Codex 工具与插件", message: managed.message, status: managed.status });
    }
    if (live) setLiveCodexContextEntries(live);
    return managed;
  };

  const saveContextEntry = async (kind: ContextKind, id: string, tomlBody: string, sourceSettings = settings?.settings ?? settingsDraft) => {
    if (!sourceSettings) return null;
    const result = await run(
      () => call<ContextEntriesResult>("upsert_context_entry", { request: { settings: sourceSettings, kind, id, tomlBody } }),
      "保存工具与插件",
    );
    if (result) {
      setCodexContextEntries(result);
      setSettingsDraft(result.settings);
      notifyResult({ title: "保存工具与插件", message: result.message, status: result.status });
      await saveSettings(result.settings);
      await refreshContextEntries(true, result.settings);
    }
    return result;
  };

  const deleteContextEntry = async (kind: ContextKind, id: string, sourceSettings = settings?.settings ?? settingsDraft) => {
    if (!sourceSettings) return null;
    const result = await run(
      () => call<ContextEntriesResult>("delete_context_entry", { request: { settings: sourceSettings, kind, id } }),
      "删除工具与插件",
    );
    if (result) {
      setCodexContextEntries(result);
      setSettingsDraft(result.settings);
      notifyResult({ title: "删除工具与插件", message: result.message, status: result.status });
      await saveSettings(result.settings);
      await refreshContextEntries(true, result.settings);
    }
    return result;
  };

  const syncLiveContextEntries = async (sourceSettings = settings?.settings ?? settingsDraft) => {
    if (!sourceSettings) return null;
    const result = await run(
      () => call<LiveContextEntriesResult>("sync_live_context_entries", { request: { settings: sourceSettings } }),
      "同步当前 Codex 配置",
    );
    if (result) {
      setLiveCodexContextEntries(result);
      notifyResult({ title: "同步当前 Codex 配置", message: result.message, status: result.status });
    }
    return result;
  };

  const refreshClaudeContextEntries = async (silent = false) => {
    const result = await run(() => call<ClaudeContextEntriesResult>("list_claude_context_entries"), "Claude 工具与插件", { trackBusy: !silent, notify: !silent });
    if (result) {
      setClaudeContextEntries(result);
      if (!silent) notifyIfNeedsAttention({ title: "Claude 工具与插件", message: result.message, status: result.status });
    }
    return result;
  };

  const saveClaudeContextEntry = async (kind: ContextKind, id: string, body: string) => {
    const result = await run(
      () => call<ClaudeContextEntriesResult>("upsert_claude_context_entry", { request: { kind, id, body } }),
      "保存 Claude 工具与插件",
    );
    if (result) {
      setClaudeContextEntries(result);
      notifyResult({ title: "保存 Claude 工具与插件", message: result.message, status: result.status });
    }
    return result;
  };

  const deleteClaudeContextEntry = async (kind: ContextKind, id: string) => {
    const result = await run(
      () => call<ClaudeContextEntriesResult>("delete_claude_context_entry", { request: { kind, id } }),
      "删除 Claude 工具与插件",
    );
    if (result) {
      setClaudeContextEntries(result);
      notifyResult({ title: "删除 Claude 工具与插件", message: result.message, status: result.status });
    }
    return result;
  };

  const checkUpdate = async (silent = false) => {
    const result = await run(() => call<UpdateResult>("check_update"), "检查更新", { trackBusy: !silent, notify: !silent });
    if (result) {
      setUpdateInfo(result);
      if (!silent) notifyIfNeedsAttention({ title: "检查更新", message: result.message, status: result.status });
    }
    return result;
  };

  const performUpdate = async (release?: UpdateReleasePayload | null) => {
    const result = await run(() => call<UpdateResult>("perform_update", release ? { release } : undefined), "下载并运行安装包");
    if (result) {
      setUpdateInfo(result);
      notifyResult({ title: "下载并运行安装包", message: result.message, status: result.status });
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

  useEffect(() => {
    const handleButtonClick = (event: MouseEvent) => {
      const target = event.target;
      if (!(target instanceof Element)) return;
      const button = target.closest("button");
      if (!(button instanceof HTMLButtonElement)) return;
      void writeUiEvent("manager.ui.button.click", {
        route,
        label: buttonLogLabel(button),
        disabled: button.disabled,
      });
    };
    document.addEventListener("click", handleButtonClick, true);
    return () => document.removeEventListener("click", handleButtonClick, true);
  }, [route]);

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
      notifyResult({ title: "Claude 一键汉化", message: result.message, status: result.status });
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
      notifyResult({ title: "启动/重启Claude", message: result.message, status: result.status });
      await refreshClaude(true);
    }
  };

  const restartCodex = async () => {
    const request = codexLaunchRequestFromOverview(overview);
    const result = await run(() => call<CommandResult<Record<string, unknown>>>("restart_claude_codex_pro", { request }), "重启 Codex");
    if (result) {
      notifyResult({ title: "重启 Codex", message: result.message, status: result.status });
      await refreshOverview(true);
    }
  };

  const launchCodex = async () => {
    const request = codexLaunchRequestFromOverview(overview);
    const result = await run(() => call<CommandResult<Record<string, unknown>>>("launch_claude_codex_pro", { request }), "启动/重启Codex");
    if (result) {
      notifyResult({ title: "启动/重启Codex", message: result.message, status: result.status });
      await refreshOverview(true);
    }
  };

  const previewPlugin = async (id: string) => {
    const result = await run(() => call<PluginInstallPreviewResult>("preview_plugin_hub_install", { request: { id } }), "安装预览");
    if (result) {
      setPluginPreview(result);
      notifyResult({ title: "安装预览", message: result.message, status: result.status });
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
      notifyResult({ title: "插件中心", message, status: result.status });
      await refreshPluginHub(true);
    }
  };

  const uninstallPlugin = async (id: string) => {
    if (!window.confirm("卸载该条目？会撤销本工具写入的 Claude Desktop MCP 配置和托管 Skills；外部 CLI 插件只移除安装记录。")) return;
    const result = await run(() => call<PluginHubResult>("uninstall_plugin_hub_item", { request: { id } }), "卸载插件");
    if (result) {
      setPluginHub(result);
      notifyResult({ title: "插件中心", message: result.message, status: result.status });
    }
  };

  const previewPonytailCodexHooks = async () => {
    const result = await run(() => call<CodexHookTrustResult>("preview_ponytail_codex_hooks"), "Ponytail Codex Hooks");
    if (result) {
      setCodexHookTrust(result);
      notifyResult({ title: "Ponytail Codex Hooks", message: result.message, status: result.status });
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
      notifyResult({ title: "Ponytail Codex Hooks", message: result.message, status: result.status });
    }
  };

  const generatePonytailMcpbInstaller = async () => {
    const result = await run(() => call<McpbPackageResult>("generate_ponytail_mcpb_installer"), "Ponytail MCPB");
    if (result) {
      notifyResult({ title: "Ponytail MCPB", message: result.message || result.package.message, status: result.status });
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
      notifyResult({ title: "Claude Desktop 本地插件包", message: result.message || result.outcome.message, status: result.status });
      await refreshPluginHub(true);
    }
  };

  const openClaudeDesktopOrgPluginsDir = async () => {
    const result = await run(() => call<ClaudeDesktopOrgPluginStatusResult>("open_claude_desktop_org_plugins_dir"), "Claude Desktop 组织插件目录");
    if (result) {
      setClaudeDesktopOrgPlugin(result);
      notifyResult({ title: "Claude Desktop 组织插件目录", message: result.message, status: result.status });
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
      notifyResult({ title: "Claude Desktop 插件仓库", message: result.message || result.outcome.message, status: result.status });
    }
  };

  const repairClaudeDesktopMarketplaces = async () => {
    setNotice({ title: "修复 Claude 插件仓库", message: "正在写入 Claude 官方与 Ponytail 插件仓库配置...", status: "running" });
    await waitForPaint();
    const result = await run(() => call<ClaudeDesktopMarketplaceRepairResult>("repair_claude_desktop_marketplaces"), "修复 Claude 插件仓库");
    if (result) {
      setClaudeDesktopMarketplace({
        status: result.status,
        message: result.message,
        marketplaceStatus: result.marketplaceStatus,
      });
      notifyResult({ title: "Claude 插件仓库", message: result.message || result.outcome.message, status: result.status });
      await refreshClaudeDesktopDevMode(true);
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
        await refreshClaudeDesktopMarketplace(true);
      }
    } finally {
      setClaudeDevModeBusy(false);
    }
  };

  const installMarketScript = async (id: string) => {
    const result = await run(() => call<ScriptMarketResult>("install_market_script", { id }), "安装脚本");
    if (result) {
      setScriptMarket(result);
      notifyResult({ title: "脚本市场", message: result.message, status: result.status });
    }
  };

  const repairCodexPluginMarketplace = async (silent = false) => {
    if (!silent) {
      setNotice({ title: "修复 Codex 插件仓库", message: "正在下载、校验并注册 Codex OpenAI 与第三方插件仓库...", status: "running" });
      await waitForPaint();
    }
    const result = await run(
      () => call<CodexPluginMarketplaceRepairResult>("repair_codex_plugin_marketplace"),
      "下载并注册 Codex 插件仓库",
      { trackBusy: !silent, notify: !silent },
    );
    if (result) {
      setCodexPluginMarketplace({
        status: result.status,
        message: result.message,
        marketplace: result.marketplace,
      });
      if (!silent) setNotice({ title: "Codex 插件仓库", message: result.message || result.repair.message, status: result.status });
      await refreshPluginHub(true);
    }
  };

  const promptAndRepairPluginRepositories = async (
    codex: CodexPluginMarketplaceStatusResult | null,
    claude: ClaudeDesktopMarketplaceStatusResult | null,
  ) => {
    const codexNeedsRepair = codexPluginMarketplaceNeedsRepair(codex);
    const claudeNeedsRepair = claudeDesktopMarketplaceNeedsRepair(claude);
    if (!codexNeedsRepair && !claudeNeedsRepair) return;

    const promptKey = pluginRepositoryRepairPromptKey(codex, claude);
    if (pluginRepositoryRepairPromptKeyRef.current === promptKey) return;
    pluginRepositoryRepairPromptKeyRef.current = promptKey;

    setNotice({
      title: "插件仓库需要修复",
      message: "检测到 Codex 或 Claude 插件仓库配置异常，等待确认修复。",
      status: "needs_review",
    });
    await waitForPaint();
    if (!window.confirm(pluginRepositoryRepairPromptMessage(codex, claude))) return;

    setNotice({ title: "修复插件仓库", message: "正在修复 Codex/Claude 插件仓库配置...", status: "running" });
    await waitForPaint();
    if (codexNeedsRepair) await repairCodexPluginMarketplace();
    if (claudeNeedsRepair) await repairClaudeDesktopMarketplaces();
    await Promise.all([refreshCodexPluginMarketplace(true), refreshClaudeDesktopMarketplace(true), refreshPluginHub(true)]);
  };

  const refreshClaudeThirdPartyConfig = async () => {
    setNotice({ title: "刷新 Claude 第三方配置", message: "正在刷新 Claude Desktop 第三方开发配置...", status: "running" });
    await waitForPaint();
    const result = await run(() => call<ClaudeDesktopDevModeConfigureResult>("refresh_claude_third_party_config"), "刷新 Claude 第三方配置");
    if (result) {
      setClaudeDesktopDevMode({
        status: result.status,
        message: result.message,
        devModeStatus: result.devModeStatus,
      });
      setNotice({ title: "刷新 Claude 第三方配置", message: result.message || result.outcome.message, status: result.status });
      await Promise.all([refreshClaudeDesktopDevMode(true), refreshClaudeDesktopMarketplace(true), refreshPluginHub(true)]);
    }
  };

  const repairFrontendConnection = async () => {
    setNotice({ title: "修复前端连接", message: "正在重新检查并注入 Codex 前端连接...", status: "running" });
    await waitForPaint();
    const result = await run(() => call<RepairConnectionResult>("repair_frontend_connection"), "修复前端连接");
    if (result) {
      const details = result.details?.length ? `\n${result.details.join("\n")}` : "";
      setNotice({ title: "修复前端连接", message: `${result.message}${details}`, status: result.status });
      await Promise.all([refreshOverview(true), refreshClaudeLight(true), refreshClaudeZhPatch(true), refreshMemoryAssist(true)]);
      window.setTimeout(() => {
        void Promise.all([refreshOverview(true), refreshMemoryAssist(true)]);
      }, 3500);
    }
  };

  const repairBackendService = async () => {
    setNotice({ title: "修复后端服务", message: "正在检查并恢复 Codex / Claude 本地后端服务...", status: "running" });
    await waitForPaint();
    const result = await run(() => call<RepairConnectionResult>("repair_backend_service"), "修复后端服务");
    if (result) {
      const details = result.details?.length ? `\n${result.details.join("\n")}` : "";
      setNotice({ title: "修复后端服务", message: `${result.message}${details}`, status: result.status });
      await Promise.all([refreshOverview(true), refreshClaudeDesktopDevMode(true)]);
    }
  };

  const openExternalUrl = async (url: string) => {
    const result = await run(() => call<CommandResult<Record<string, unknown>>>("open_external_url", { url }), "打开链接");
    if (result) {
      notifyResult({ title: "打开链接", message: result.message, status: result.status });
    }
  };

  const goPluginHub = async () => {
    setRoute("tools");
    await refreshRoute("tools");
  };

  const goMemoryAssist = async () => {
    setRoute("sessions");
    await refreshRoute("sessions");
  };

  const goPromptOptimizer = async () => {
    await openExternalUrl(PROMPT_OPTIMIZER_URL);
  };

  const repairEntrypoints = async () => {
    const result = await run(() => call<CommandResult<Record<string, unknown>>>("repair_shortcuts"), "修复入口");
    if (result) notifyResult({ title: "修复入口", message: result.message, status: result.status });
    await refreshOverview(true);
  };

  const repairBackend = async () => {
    const result = await run(() => call<SettingsResult>("repair_backend"), "修复后端");
    if (result) {
      setSettings(result);
      setSettingsDraft(result.settings);
      notifyResult({ title: "修复后端", message: result.message, status: result.status });
    }
  };

  const repairHistorySessions = async () => {
    const result = await run(() => call<ProviderSyncResult>("sync_providers_now"), "历史会话修复");
    if (result) {
      setProviderSync(result);
      notifyResult({ title: "历史会话修复", message: result.message, status: result.status });
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
      notifyResult({ title: "删除 Codex 会话", message: result.message, status: result.status });
      await refreshLocalSessions(true);
    }
  };

  const learnMemoryAssistItem = async (text: string, category = "manual") => {
    const result = await run(
      () => call<MemoryItemResult>("learn_memory_assist_item", { request: { text, category, workspace: MEMORY_GLOBAL_WORKSPACE, source: "manager" } }),
      "保存记忆",
    );
    if (result) {
      notifyResult({ title: "盘古记忆", message: result.message, status: result.status });
      await refreshMemoryAssist(true);
    }
    return result?.status === "ok";
  };

  const updateMemoryAssistItem = async (id: string, item: MemoryItemEditRequest) => {
    const result = await run(
      () => call<MemoryItemResult>("update_memory_assist_item", { request: { id, item } }),
      "更新记忆",
    );
    if (result) {
      notifyResult({ title: "盘古记忆", message: result.message, status: result.status });
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
      notifyResult({ title: "记忆搜索", message: result.message, status: result.status });
    }
  };

  const deleteMemoryAssistItem = async (id: string) => {
    if (!window.confirm("确认删除这条经验教训？")) return;
    const result = await run(() => call<MemoryItemResult>("delete_memory_assist_item", { request: { id } }), "删除经验教训");
    if (result) {
      notifyResult({ title: "盘古记忆", message: result.message, status: result.status });
      await refreshMemoryAssist(true);
    }
  };

  const approveMemoryAssistCandidate = async (id: string) => {
    const result = await run(() => call<MemoryItemResult>("approve_memory_assist_candidate", { request: { id } }), "确认候选记忆");
    if (result) {
      notifyResult({ title: "盘古记忆", message: result.message, status: result.status });
      await refreshMemoryAssist(true);
    }
  };

  const rejectMemoryAssistCandidate = async (id: string) => {
    const result = await run(() => call<MemoryCandidateResult>("reject_memory_assist_candidate", { request: { id } }), "忽略候选记忆");
    if (result) {
      notifyResult({ title: "盘古记忆", message: result.message, status: result.status });
      await refreshMemoryAssist(true);
    }
  };

  const exportMemoryAssist = async () => {
    const result = await run(() => call<MemoryExportResult>("export_memory_assist"), "导出记忆");
    if (result) {
      setMemoryExport(result);
      notifyResult({ title: "记忆导出", message: result.message, status: result.status });
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
    if (!window.confirm(`确认导入记忆数据？\n\n${action}\n经验教训：${data.items.length} 条\n候选缓存：${data.candidates.length} 条`)) return;
    const result = await run(
      () => call<MemoryStatusResult>("import_memory_assist", { request: { data, replaceExisting } }),
      "导入记忆",
    );
    if (result) {
      setMemoryAssist(result);
      notifyResult({ title: "记忆导入", message: result.message, status: result.status });
      await refreshMemoryAssist(true);
    }
  };

  const runMemoryAssistSelfcheck = async () => {
    const result = await run(() => call<MemorySelfCheckResult>("run_memory_assist_selfcheck", { request: { repair: true } }), "盘古记忆自检");
    if (result) {
      setMemorySelfCheck(result);
      notifyResult({ title: "盘古记忆自检", message: result.message, status: result.status });
      await refreshMemoryAssist(true);
    }
  };

  const refineLongTermMemory = async () => {
    setNotice({
      title: "提炼经验教训",
      message: "正在使用 Codex 本地 SQLite、rollout 会话文件和 memory_assist.sqlite 遍历工作区与会话...",
      status: "running",
    });
    await waitForPaint();
    void writeUiEvent("memory.refine_long_term.click", {
      sources: ["codex_sqlite", "codex_rollout_files", "memory_assist.sqlite"],
      mode: "repair_selfcheck_full_history",
    });
    const result = await run(() => call<MemorySelfCheckResult>("run_memory_assist_selfcheck", { request: { repair: true } }), "提炼经验教训");
    if (result) {
      setMemorySelfCheck(result);
      setNotice({ title: "提炼经验教训", message: memoryRefineSummary(result), status: result.status });
      await refreshMemoryAssist(true);
    }
  };

  const applyRelayMode = async () => {
    const result = await run(() => call<CommandResult<Record<string, unknown>>>("apply_relay_injection"), "官方混入 API Key");
    if (result) {
      notifyResult({ title: "官方混入 API Key", message: result.message, status: result.status });
      await refreshSettings(true);
    }
  };

  const applyPureApiMode = async () => {
    const result = await run(() => call<CommandResult<Record<string, unknown>>>("apply_pure_api_injection"), "纯 API");
    if (result) {
      notifyResult({ title: "纯 API", message: result.message, status: result.status });
      await refreshSettings(true);
    }
  };

  const clearRelayMode = async () => {
    const result = await run(() => call<CommandResult<Record<string, unknown>>>("clear_relay_injection"), "清除 API 模式");
    if (result) {
      notifyResult({ title: "清除 API 模式", message: result.message, status: result.status });
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
    if (targetProfile && !supplierProfileHasApiKey(targetProfile)) {
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
      notifyResult({ title: "切换 Codex 供应商", message: result.message, status: result.status });
      await refreshSettings(true);
    }
  };

  const fetchRelayProfileModels = async (profile: RelayProfile) => {
    const result = await run(() => call<RelayProfileModelsResult>("fetch_relay_profile_models", { profile }), "获取供应商模型");
    if (result) {
      notifyResult({ title: "获取供应商模型", message: result.message, status: result.status });
    }
    return result;
  };

  const importCcswitchCodexProviders = async () => {
    const result = await run(() => call<CcswitchImportResult>("import_ccswitch_codex_providers"), "CC-switch 导入");
    if (result) {
      notifyResult({ title: "CC-switch 导入", message: result.message, status: result.status });
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
      notifyResult({ title: "Claude Desktop 供应商预览", message: result.message, status: result.status });
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
      notifyResult({ title: "Claude Desktop 供应商", message: result.message, status: result.status });
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
      notifyResult({ title: "Claude Desktop 官方模式", message: result.message, status: result.status });
      await refreshClaudeDesktopDevMode(true);
    }
  };

  const saveSettings = async (next: BackendSettings) => {
    const result = await run(() => call<SettingsResult>("save_settings", { settings: next }), "保存设置");
    if (result) {
      setSettings(result);
      setSettingsDraft(result.settings);
      notifyResult({ title: "保存设置", message: result.message, status: result.status });
    }
    return result;
  };

  const installEntrypoints = async () => {
    const result = await run(() => call<InstallEntrypointsResult>("install_entrypoints"), "安装入口");
    if (result) notifyResult({ title: "安装入口", message: result.message, status: result.status });
    await refreshOverview(true);
  };

  const uninstallEntrypoints = async () => {
    if (!window.confirm("卸载入口会移除静默启动和管理工具快捷方式，不会删除配置数据。继续？")) return;
    const result = await run(
      () => call<InstallEntrypointsResult>("uninstall_entrypoints", { options: { removeOwnedData: false } }),
      "卸载入口",
    );
    if (result) notifyResult({ title: "卸载入口", message: result.message, status: result.status });
    await refreshOverview(true);
  };

  const repairShortcuts = async () => {
    const result = await run(() => call<InstallEntrypointsResult>("repair_shortcuts"), "修复快捷方式");
    if (result) notifyResult({ title: "修复快捷方式", message: result.message, status: result.status });
    await refreshOverview(true);
  };

  const watcherAction = async (command: "install_watcher" | "uninstall_watcher" | "enable_watcher" | "disable_watcher", title: string) => {
    const result = await run(() => call<WatcherResult>(command), title);
    if (result) {
      setWatcher(result);
      notifyResult({ title, message: result.message, status: result.status });
    }
  };

  const resetSettings = async () => {
    if (!window.confirm("确认重置管理工具设置？该操作会恢复默认配置。")) return;
    const result = await run(() => call<SettingsResult>("reset_settings"), "重置设置");
    if (result) {
      setSettings(result);
      setSettingsDraft(result.settings);
      notifyResult({ title: "重置设置", message: result.message, status: result.status });
    }
  };

  const resetImageOverlaySettings = async () => {
    const result = await run(() => call<SettingsResult>("reset_image_overlay_settings"), "重置图片覆盖");
    if (result) {
      setSettings(result);
      setSettingsDraft(result.settings);
      notifyResult({ title: "重置图片覆盖", message: result.message, status: result.status });
    }
  };

  const refreshRoute = async (target = route) => {
    // Capture this load's epoch so trailing work can bail if the user has
    // navigated to another route (or re-triggered this one) in the meantime.
    const loadEpoch = routeLoadEpochRef.current + 1;
    routeLoadEpochRef.current = loadEpoch;
    const isStaleRouteLoad = () => routeLoadEpochRef.current !== loadEpoch;
    if (target === "overview") {
      await Promise.all([refreshOverview(true), refreshClaudeLight(true), refreshClaudeDesktopDevMode(true), refreshSettings(true)]);
      afterFirstPaint(() => {
        void refreshMemoryAssistStatus(true);
      }, 250);
      afterFirstPaint(() => {
        void refreshClaudeZhPatch(true);
      }, 650);
    } else if (target === "settings") {
      await Promise.all([refreshSettings(true), refreshLogs(true)]);
    } else if (target === "supplier") {
      await Promise.all([refreshSettings(true), refreshClaudeDesktopDevMode(true)]);
    } else if (target === "tools") {
      const loadedSettings = await refreshSettings(true);
      const [
        ,
        codexMarketplaceStatus,
        ,
        claudeMarketplaceStatus,
      ] = await Promise.all([
        refreshPluginHub(true),
        refreshCodexPluginMarketplace(true),
        refreshClaudeDesktopOrgPlugin(true),
        refreshClaudeDesktopMarketplace(true),
        refreshClaudeDesktopDevMode(true),
        refreshClaudeContextEntries(true),
        refreshOverview(true),
        refreshClaude(true),
        refreshWatcher(true),
      ]);
      if (isStaleRouteLoad()) return;
      const sourceSettings = loadedSettings?.settings ?? settings?.settings ?? settingsDraft;
      if (sourceSettings) await refreshContextEntries(true, sourceSettings);
      if (isStaleRouteLoad()) return;
      await promptAndRepairPluginRepositories(codexMarketplaceStatus, claudeMarketplaceStatus);
    } else if (target === "sessions") {
      await Promise.all([
        refreshLocalSessions(true),
        refreshMemoryAssist(true),
        refreshSettings(true),
        refreshOverview(true),
        refreshClaude(true),
      ]);
    } else if (target === "maintenance") {
      await Promise.all([refreshOverview(true), refreshSettings(true), refreshWatcher(true), refreshClaudeLight(true)]);
    } else if (target === "about") {
      await Promise.all([refreshOverview(true), refreshClaudeLight(true), checkUpdate(true)]);
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
    if (codexMarketplaceAutoRegisterRef.current) return;
    codexMarketplaceAutoRegisterRef.current = true;
    void (async () => {
      const status = await refreshCodexPluginMarketplace(true);
      if (codexPluginMarketplaceNeedsRepair(status)) {
        await repairCodexPluginMarketplace(true);
        const afterRepair = await refreshCodexPluginMarketplace(true);
        // The auto-repair used to run silently and swallow any failure, leaving
        // the plugin repositories broken with no signal to the user. If repair
        // ran but the repositories still need repair, surface it instead of
        // hiding it.
        if (codexPluginMarketplaceNeedsRepair(afterRepair)) {
          setNotice({
            title: "Codex 插件仓库",
            message:
              afterRepair?.message ||
              "Codex 插件仓库自动修复未成功，请在工具页手动点击修复并查看详情。",
            status: "needs_review",
          });
        }
      }
    })();
  }, []);

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
      goMemoryAssist,
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
      repairClaudeDesktopMarketplaces,
      configureClaudeDesktopDevMode,
      installMarketScript,
      refreshCodexPluginMarketplace,
      repairCodexPluginMarketplace,
      refreshClaudeThirdPartyConfig,
      repairFrontendConnection,
      repairBackendService,
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
      updateMemoryAssistItem,
      searchMemoryAssist,
      deleteMemoryAssistItem,
      approveMemoryAssistCandidate,
      rejectMemoryAssistCandidate,
      runMemoryAssistSelfcheck,
      refineLongTermMemory,
      exportMemoryAssist,
      importMemoryAssist,
      applyRelayMode,
      applyPureApiMode,
      clearRelayMode,
      switchCodexRelayProfile,
      fetchRelayProfileModels,
      importCcswitchCodexProviders,
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
      checkUpdate,
      performUpdate,
      refreshContextEntries,
      saveContextEntry,
      deleteContextEntry,
      syncLiveContextEntries,
      refreshClaudeContextEntries,
      saveClaudeContextEntry,
      deleteClaudeContextEntry,
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
          {route === "overview" ? <OverviewScreen actions={actions} claudeDesktop={claudeDesktop} claudeDesktopDevMode={claudeDesktopDevMode} claudeDevModeBusy={claudeDevModeBusy} claudeZhPatch={claudeZhPatch} memoryAssist={memoryAssist} memoryItems={memoryItems} overview={overview} settings={settingsDraft ?? settings?.settings ?? null} /> : null}
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
              claudeContextEntries={claudeContextEntries}
              claudeDesktopDevMode={claudeDesktopDevMode}
              claudeDesktopMarketplace={claudeDesktopMarketplace}
              claudeDesktopOrgPlugin={claudeDesktopOrgPlugin}
              codexPluginMarketplace={codexPluginMarketplace}
              codexContextEntries={codexContextEntries}
              hub={pluginHub}
              liveCodexContextEntries={liveCodexContextEntries}
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
              memoryExport={memoryExport}
              memoryItems={memoryItems}
              memorySearch={memorySearch}
              memorySelfCheck={memorySelfCheck}
              providerSync={providerSync}
              settings={settings}
            />
          ) : null}
          {route === "maintenance" ? <MaintenanceScreen actions={actions} claudeDesktop={claudeDesktop} overview={overview} settings={settings} watcher={watcher} /> : null}
          {route === "settings" ? <SettingsScreen actions={actions} claudeChinese={claudeChinese} claudeZhPatch={claudeZhPatch} draft={settingsDraft} logs={logs} onDraftChange={setSettingsDraft} overview={overview} settings={settings} watcher={watcher} /> : null}
          {route === "about" ? <AboutScreen actions={actions} claudeDesktop={claudeDesktop} overview={overview} updateInfo={updateInfo} /> : null}
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
  claudeZhPatch,
  claudeDesktopDevMode,
  claudeDevModeBusy,
  memoryAssist,
  memoryItems,
  settings,
}: {
  actions: ReturnType<typeof createActionsShape>;
  overview: OverviewResult | null;
  claudeDesktop: ClaudeDesktopResult | null;
  claudeZhPatch: ClaudeZhPatchResult | null;
  claudeDesktopDevMode: ClaudeDesktopDevModeStatusResult | null;
  claudeDevModeBusy: boolean;
  memoryAssist: MemoryStatusResult | null;
  memoryItems: MemoryItemsResult | null;
  settings: BackendSettings | null;
}) {
  const [showMemoryDetails, setShowMemoryDetails] = useState(false);
  const memory = memoryAssist?.memory;
  const codexStatus = codexOverviewStatus(overview);
  const claudeStatus = claudeOverviewStatus(claudeDesktop, claudeZhPatch);
  const memoryStatus = memoryOverviewStatus(memoryAssist, settings);
  const devModeConfigured = !!claudeDesktopDevMode?.devModeStatus.configured;
  const devModeStatus = claudeDevModeBusy ? "running" : devModeConfigured ? "ok" : "not_checked";
  const devModeValue = claudeDevModeBusy ? "写入中..." : devModeConfigured ? "已写入" : "写入开发配置";
  const memoryEnabled = memory?.enabled ?? Boolean(settings?.memoryAssistEnabled);
  const memoryInjectEnabled = memory?.injectEnabled ?? Boolean(settings?.memoryAssistInjectEnabled);
  const memoryAutoSuggestEnabled = memory?.autoSuggestEnabled ?? Boolean(settings?.memoryAssistAutoSuggestEnabled);
  const memoryCodexInjected = Boolean(memory?.codexInjected);
  const memoryMonitorActive = Boolean(memory?.active);
  const memoryRuntimeStatus = memory?.runtimeStatus ?? "not_checked";
  const memoryRuntimeMessage = memory?.runtimeMessage || "尚未检测到 Codex 记忆运行时。";
  const memoryInjectStatus = memoryCodexInjected ? "ok" : memoryEnabled && memoryInjectEnabled ? "running" : memoryEnabled ? "failed" : "not_checked";
  const memoryInjectValue = memoryCodexInjected ? "已注入" : memoryEnabled && memoryInjectEnabled ? "等待 Codex 注入" : "未开启";
  const memoryMonitorValue = memoryMonitorActive
    ? "自动学习运行中"
    : memoryEnabled && memoryAutoSuggestEnabled
      ? memoryCodexInjected
        ? "等待会话变化"
        : "等待 Codex 注入"
      : "未监听";
  const memoryMonitorStatus = memoryMonitorActive || (memoryCodexInjected && memoryAutoSuggestEnabled) ? "running" : memoryEnabled && memoryAutoSuggestEnabled ? "failed" : "not_checked";
  const memoryWorkspaceCount = memory?.workspaces?.length ?? 0;
  const memoryCaptureCount = memory?.totalCaptures ?? memory?.workspaces?.reduce((total, workspace) => total + (workspace.captureCount || 0), 0) ?? 0;
  const openMemoryDetails = async () => {
    setShowMemoryDetails(true);
    await actions.refreshMemoryAssist();
  };
  const toggleMemoryAssistEnabled = async (enabled: boolean) => {
    if (!settings) {
      actions.showNotice({ title: "盘古记忆开关", message: "设置尚未加载，请先刷新概览。", status: "failed" });
      return;
    }
    const saved = await actions.saveSettings({ ...settings, memoryAssistEnabled: enabled });
    if (saved) await actions.refreshMemoryAssist();
  };
  return (
    <div className="ops-dashboard">
      <div className="ops-matrix">
        <StatusTile icon={Power} items={codexStatus.items} label="Codex 状态" status={codexStatus.status} />
        <StatusTile icon={MessageCircle} items={claudeStatus.items} label="Claude 状态" status={claudeStatus.status} />
        <StatusActionTile disabled={claudeDevModeBusy} icon={Wrench} label="Claude 一键开发模式" onClick={() => void actions.configureClaudeDesktopDevMode()} status={devModeStatus} value={devModeValue} />
        <StatusTile icon={ShieldCheck} items={memoryStatus.items} label="盘古记忆" status={memoryStatus.status} />
      </div>
      <div className="ops-overview-grid">
        <Panel title="盘古记忆总览" hideHeader>
          <div className="memory-overview-header">
            <div>
              <strong>盘古记忆开关</strong>
              <p>{memoryEnabled ? "已允许 Codex 使用本地经验教训与会话摘要。" : "当前不会向 Codex 注入盘古记忆。可在这里直接开启。"}</p>
            </div>
            <ToggleSwitch checked={memoryEnabled} disabled={!settings} onChange={(value) => void toggleMemoryAssistEnabled(value)} />
          </div>
          <div className="ops-status-list">
            <StatusRow label="运行状态" status={memoryRuntimeStatus} value={memoryRuntimeMessage} />
            <StatusRow label="Codex 注入" status={memoryInjectStatus} value={memoryInjectValue} />
            <StatusRow label="对话监控" status={memoryMonitorStatus} value={memoryMonitorValue} />
          </div>
          <div className="ops-note">
            <Activity className="h-4 w-4" />
            <span>对话监控</span>
            <span className="memory-activity-wave" data-active={memoryMonitorActive}>
              <span />
              <span />
              <span />
              <span />
            </span>
          </div>
          <div className="info-grid compact memory-overview-matrix">
            <InfoRow
              action={(
                <button
                  aria-label="查看/编辑经验教训"
                  className="info-row-action"
                  onClick={() => void openMemoryDetails()}
                  type="button"
                >
                  <Pencil className="h-3.5 w-3.5" />
                  查看/编辑
                </button>
              )}
              label="经验教训"
              value={(memory?.totalItems ?? 0) > 0 ? "已沉淀" : "待提炼"}
            />
            <InfoRow label="采集记录" value={`${memoryCaptureCount} 条`} />
            <InfoRow label="工作区" value={`${memoryWorkspaceCount} 个`} />
            <InfoRow label="数据库" value={compactPath(memory?.dbPath)} />
            <InfoRow label="最近备份" value={compactPath(memory?.latestBackupPath)} />
          </div>
          <div className="action-row memory-overview-actions">
            <ActionButton icon={PencilRuler} label="提炼经验教训" onClick={() => void actions.refineLongTermMemory()} />
            <ActionButton icon={RefreshCw} label="刷新盘古记忆" onClick={() => void actions.refreshMemoryAssist()} />
            <ActionButton icon={ShieldCheck} label="盘古记忆自检并备份" onClick={() => void actions.runMemoryAssistSelfcheck()} />
          </div>
        </Panel>
        <div className="overview-side-stack">
          <Panel title="诊断与修复" detail="检查和修复入口集中在这里；修复动作会先显示运行反馈，再调用后端命令。">
            <ActionButton icon={RefreshCw} label="刷新概览" onClick={() => void actions.refreshRoute("overview")} />
            <ActionButton icon={RefreshCw} label="刷新 Claude 第三方配置" onClick={() => void actions.refreshClaudeThirdPartyConfig()} />
            <ActionButton icon={Wrench} label="修复前端连接" onClick={() => void actions.repairFrontendConnection()} />
            <ActionButton icon={Wrench} label="修复后端服务" onClick={() => void actions.repairBackendService()} />
          </Panel>
          {showMemoryDetails ? (
            <OverviewMemoryDetails
              actions={actions}
              items={memoryItems}
              onClose={() => setShowMemoryDetails(false)}
            />
          ) : null}
        </div>
      </div>
    </div>
  );
}

function OverviewMemoryDetails({
  actions,
  items,
  onClose,
}: {
  actions: ReturnType<typeof createActionsShape>;
  items: MemoryItemsResult | null;
  onClose: () => void;
}) {
  const [editingMemoryId, setEditingMemoryId] = useState("");
  const [editingText, setEditingText] = useState("");
  const [editingCategory, setEditingCategory] = useState("");
  const allItems = items?.items ?? [];
  const beginEditMemory = (item: MemoryItem) => {
    setEditingMemoryId(item.id);
    setEditingText(item.text);
    setEditingCategory(item.category);
  };
  const cancelEditMemory = () => {
    setEditingMemoryId("");
    setEditingText("");
    setEditingCategory("");
  };
  const saveEditedMemory = async (item: MemoryItem) => {
    const text = editingText.trim();
    if (!text) return;
    const saved = await actions.updateMemoryAssistItem(item.id, {
      text,
      workspace: item.workspace,
      category: editingCategory.trim() || item.category || "general",
      tags: item.tags,
      source: item.source || "manager",
      sourceSessionId: item.sourceSessionId,
    });
    if (saved) cancelEditMemory();
  };
  return (
    <Panel title="经验教训手册详情" detail="提炼结果会合成为一条精简手册，可在这里直接查看和编辑。">
      <div className="overview-memory-toolbar">
        <Button onClick={() => void actions.refineLongTermMemory()} size="sm">
          <PencilRuler className="h-4 w-4" />
          提炼经验教训
        </Button>
        <Button onClick={() => void actions.refreshMemoryAssist()} size="sm" variant="outline">
          <RefreshCw className="h-4 w-4" />
          刷新
        </Button>
        <Button onClick={onClose} size="sm" variant="outline">收起</Button>
      </div>
      <div className="overview-memory-list">
        {allItems.length ? allItems.map((item) => {
          const editing = editingMemoryId === item.id;
          return (
            <div className="memory-assist-row memory-lesson-card" key={item.id}>
              <span>{item.category} · {item.workspace}</span>
              {editing ? (
                <>
                  <label className="ops-form-field">
                    <span>分类</span>
                    <input onChange={(event) => setEditingCategory(event.currentTarget.value)} value={editingCategory} />
                  </label>
                  <label className="ops-form-field">
                    <span>经验教训内容</span>
                    <textarea className="ops-textarea compact" onChange={(event) => setEditingText(event.currentTarget.value)} value={editingText} />
                  </label>
                  <div className="action-row">
                    <Button disabled={!editingText.trim()} onClick={() => void saveEditedMemory(item)} size="sm">
                      <Save className="h-4 w-4" />
                      保存
                    </Button>
                    <Button onClick={cancelEditMemory} size="sm" variant="outline">取消</Button>
                  </div>
                </>
              ) : (
                <>
                  <p>{item.text}</p>
                  <div className="action-row">
                    <Button onClick={() => beginEditMemory(item)} size="sm" variant="outline">
                      <Pencil className="h-4 w-4" />
                      编辑
                    </Button>
                    <Button onClick={() => void actions.deleteMemoryAssistItem(item.id)} size="sm" variant="outline">删除</Button>
                  </div>
                </>
              )}
            </div>
          );
        }) : <Empty text="暂无经验教训。" />}
      </div>
    </Panel>
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
  const [importOpen, setImportOpen] = useState(false);
  const [draggedId, setDraggedId] = useState<string | null>(null);
  const [dragOverId, setDragOverId] = useState<string | null>(null);
  const [supplierOrderIds, setSupplierOrderIds] = useState<string[]>([]);
  const appSettings = settings?.settings ?? null;
  const profiles = useMemo(() => appSettings?.relayProfiles ?? [], [appSettings]);
  const profileIdsKey = profiles.map((profile) => profile.id).join("\u001f");
  const active = profiles.find((profile) => profile.id === appSettings?.activeRelayId) ?? profiles[0];
  const editingExisting = draft && editingId ? profiles.find((profile) => profile.id === editingId) : null;
  const isNewDraft = !!draft && !editingExisting;
  const aggregateProfiles = useMemo(() => profiles.filter((profile) => profile.aggregateEnabled), [profiles]);
  const apiProfiles = useMemo(() => profiles.filter((profile) => !profile.aggregateEnabled && profile.relayMode !== "official"), [profiles]);
  const updateClaudeDraft = (field: keyof typeof claudeDesktopProviderDraft, value: string) => {
    onClaudeDesktopProviderDraftChange((current) => ({ ...current, [field]: value }));
  };
  useEffect(() => {
    setSupplierOrderIds(profiles.map((profile) => profile.id));
  }, [profileIdsKey]);
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
  const createAggregateProfile = () => {
    if (!appSettings) return;
    const profile = createAggregateSupplierProfile(appSettings);
    setModelFetch(null);
    setEditingId(null);
    setDraft(profile);
    if (!apiProfiles.length) {
      actions.showNotice({ title: "添加聚合供应商", message: "已打开聚合供应商详情；请先添加或选择至少 1 个普通 API 供应商的 Base URL / Key，再勾选为成员。", status: "failed" });
    }
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
    const aggregateDraft = !!draft.aggregateEnabled;
    const requestedId = draft.id.trim();
    const normalizedId = supplierIdFromName(requestedId || draft.name);
    const idWasNormalized = requestedId !== normalizedId;
    const normalized = normalizeSupplierProfile(withSupplierGeneratedFiles({ ...draft, id: normalizedId }));
    if (!normalized.name.trim() || (!aggregateDraft && !normalized.baseUrl.trim())) {
      window.alert(aggregateDraft ? "请填写聚合供应商名称后再保存。" : "请填写供应商名称和 Base URL 后再保存。API Key 可以后续补入。");
      return null;
    }
    if (aggregateDraft && !(normalized.aggregateMembers ?? []).length) {
      actions.showNotice({ title: "添加聚合供应商", message: "请先添加或选择至少 1 个普通 API 供应商的 Base URL / Key，再勾选为成员。", status: "failed" });
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
    const nextActiveRelayId = !aggregateDraft && (appSettings.activeRelayId === originalId || !appSettings.activeRelayId)
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
    if (draft.aggregateEnabled) {
      actions.showNotice({ title: "供应商切换", message: "聚合供应商已经保存为真实配置记录；当前版本还没有聚合轮转代理，不能直接写入 Codex。", status: "failed" });
      return;
    }
    const saved = await saveDraft({ stayInEditor: true });
    if (saved) {
      const savedProfile = normalizeSupplierProfile(saved.profile);
      if (!supplierProfileHasApiKey(savedProfile)) {
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
    const nextProfiles = profiles
      .filter((item) => item.id !== profile.id)
      .map((item) => item.aggregateEnabled ? { ...item, aggregateMembers: (item.aggregateMembers ?? []).filter((id) => id !== profile.id) } : item);
    const nextActive = appSettings.activeRelayId === profile.id ? nextProfiles.find((item) => !item.aggregateEnabled)?.id ?? nextProfiles[0]?.id ?? "" : appSettings.activeRelayId;
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
      aggregateEnabled: false,
      aggregateMembers: [],
      aggregateStrategy: "",
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
  const supplierOrderFromIds = (ids: string[]) => {
    const byId = new Map(profiles.map((profile) => [profile.id, profile]));
    const ordered = ids
      .map((id) => byId.get(id))
      .filter((profile): profile is RelayProfile => !!profile);
    const used = new Set(ordered.map((profile) => profile.id));
    return [...ordered, ...profiles.filter((profile) => !used.has(profile.id))];
  };
  // 渲染用的排序结果：drag 期间 dragOverId 频繁变化，避免每次重建 Map + 重排。
  // supplierOrderFromIds 是纯函数，仅依赖 profiles 与传入的 ids。
  // 必须置于任何条件 return 之前以遵守 Hooks 规则。
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const orderedProfiles = useMemo(() => supplierOrderFromIds(supplierOrderIds), [profiles, supplierOrderIds]);
  const reorderSupplierIds = (sourceId: string, targetId: string, ids = supplierOrderIds) => {
    const currentIds = supplierOrderFromIds(ids.length ? ids : profiles.map((profile) => profile.id)).map((profile) => profile.id);
    const fromIndex = currentIds.indexOf(sourceId);
    const toIndex = currentIds.indexOf(targetId);
    if (fromIndex < 0 || toIndex < 0) return;
    const nextIds = [...currentIds];
    const [moved] = nextIds.splice(fromIndex, 1);
    nextIds.splice(toIndex, 0, moved);
    return nextIds;
  };
  const previewSupplierOrder = (sourceId: string, targetId: string) => {
    if (sourceId === targetId || dragOverId === targetId) return;
    setDragOverId(targetId);
    setSupplierOrderIds((current) => reorderSupplierIds(sourceId, targetId, current) ?? current);
  };
  const beginSupplierDrag = (event: DragEvent<HTMLElement>, profileId: string) => {
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData(SUPPLIER_DRAG_MIME_TYPE, profileId);
    event.dataTransfer.setData("text/plain", profileId);
    setDraggedId(profileId);
    setDragOverId(null);
  };
  const supplierDragSourceId = (event: DragEvent<HTMLElement>) =>
    event.dataTransfer.getData(SUPPLIER_DRAG_MIME_TYPE) || event.dataTransfer.getData("text/plain") || draggedId;
  const previewSupplierDrag = (event: DragEvent<HTMLElement>, targetId: string) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
    const sourceId = supplierDragSourceId(event);
    if (sourceId) previewSupplierOrder(sourceId, targetId);
  };
  const saveSupplierOrder = async (orderedIds: string[]) => {
    if (!appSettings) return;
    const reordered = supplierOrderFromIds(orderedIds);
    const previousIds = profiles.map((profile) => profile.id);
    const nextIds = reordered.map((profile) => profile.id);
    if (nextIds.join("\u001f") === previousIds.join("\u001f")) return;
    actions.showNotice({ title: "供应商排序", message: "正在保存供应商顺序...", status: "running" });
    const saved = await saveSupplierSettings({ ...appSettings, relayProfiles: reordered });
    if (saved) {
      setSupplierOrderIds(saved.relayProfiles.map((profile) => profile.id));
      actions.showNotice({ title: "供应商排序", message: "供应商顺序已保存。", status: "ok" });
    } else {
      setSupplierOrderIds(previousIds);
      actions.showNotice({ title: "供应商排序", message: "供应商顺序保存失败，已恢复原顺序。", status: "failed" });
    }
  };
  const pinSupplierToTop = (profileId: string) => {
    const currentIds = supplierOrderFromIds(supplierOrderIds.length ? supplierOrderIds : profiles.map((profile) => profile.id))
      .map((profile) => profile.id);
    if (!currentIds.length || currentIds[0] === profileId) return;
    const nextIds = [profileId, ...currentIds.filter((id) => id !== profileId)];
    setSupplierOrderIds(nextIds);
    void saveSupplierOrder(nextIds);
  };
  const importFromCcswitch = async () => {
    if (!appSettings) return;
    setImportOpen(false);
    const result = await actions.importCcswitchCodexProviders();
    if (!result || statusFailed(result.status)) return;
    const imported = result.profiles.map((profile) => normalizeSupplierProfile(withSupplierGeneratedFiles(profile)));
    const importedById = new Map(imported.map((profile) => [profile.id, profile]));
    let updatedCount = 0;
    const nextProfiles = appSettings.relayProfiles.map((profile) => {
      const importedProfile = importedById.get(profile.id);
      if (importedProfile && supplierProfileIsCcswitch(profile)) {
        importedById.delete(profile.id);
        updatedCount += 1;
        return importedProfile;
      }
      return profile;
    });
    const existingIds = new Set(nextProfiles.map((profile) => profile.id));
    let addedCount = 0;
    for (const profile of importedById.values()) {
      const nextProfile = existingIds.has(profile.id)
        ? normalizeSupplierProfile(withSupplierGeneratedFiles({ ...profile, id: uniqueSupplierProfileId(nextProfiles, profile.id) }))
        : profile;
      existingIds.add(nextProfile.id);
      nextProfiles.push(nextProfile);
      addedCount += 1;
    }
    await saveSupplierSettings({ ...appSettings, relayProfiles: nextProfiles });
    actions.showNotice({ title: "CC-switch 导入", message: `已从 cc-switch 更新 ${updatedCount} 个、新增 ${addedCount} 个供应商配置。`, status: "ok" });
  };

  if (draft?.aggregateEnabled) {
    const generated = normalizeSupplierProfile(withSupplierGeneratedFiles(draft));
    const members = generated.aggregateMembers ?? [];
    return (
      <div className="supplier-workbench">
        <Panel title={generated.name || "聚合供应商1"} detail="聚合供应商会保存策略和成员关系；当前版本不直接写入 Codex，后续聚合代理会读取这些字段。">
          <div className="supplier-editor-toolbar sticky">
            <Button onClick={() => { setDraft(null); setEditingId(null); }} variant="outline">返回列表</Button>
            <Button disabled={supplierSaveBusy} onClick={() => void saveDraft()} type="button">
              <Save className="h-4 w-4" />
              {supplierSaveBusy ? "保存中" : "保存"}
            </Button>
          </div>
          <div className="supplier-editor-card">
            <div className="supplier-editor-titleline"><strong>{generated.name}</strong><span className="supplier-badge">聚合</span></div>
            <div className="supplier-form-grid">
              <label className="ops-form-field"><span>名称</span><input onChange={(event) => updateDraft({ name: event.currentTarget.value })} value={generated.name} /></label>
              <label className="ops-form-field"><span>测试模型</span><input onChange={(event) => updateDraft({ testModel: event.currentTarget.value, model: event.currentTarget.value })} value={generated.testModel || generated.model} /></label>
              <label className="ops-form-field span-2"><span>聚合策略</span><select className="ops-select" onChange={(event) => updateDraft({ aggregateStrategy: event.currentTarget.value })} value={generated.aggregateStrategy || "failover"}>{AGGREGATE_STRATEGIES.map((strategy) => <option key={strategy.id} value={strategy.id}>{strategy.label}</option>)}</select></label>
            </div>
            <div className="supplier-aggregate-grid">
              {AGGREGATE_STRATEGIES.map((strategy) => <button className={strategy.id === (generated.aggregateStrategy || "failover") ? "selected" : ""} key={strategy.id} onClick={() => updateDraft({ aggregateStrategy: strategy.id })} type="button"><strong>{strategy.label}</strong><span>{strategy.detail}</span></button>)}
            </div>
            <div className="supplier-member-box">
              <div className="supplier-member-head"><strong>成员供应商</strong><span>{members.length}/{apiProfiles.length}</span></div>
              {apiProfiles.length ? apiProfiles.map((profile) => {
                const checked = members.includes(profile.id);
                return <label className="supplier-member-row" key={profile.id}><input checked={checked} onChange={(event) => updateDraft({ aggregateMembers: event.currentTarget.checked ? [...members, profile.id] : members.filter((id) => id !== profile.id) })} type="checkbox" /><span>{profile.name || profile.id}</span><small>{profile.baseUrl || "未配置 Base URL"}</small></label>;
              }) : <p>请先添加或选择至少 1 个普通 API 供应商的 Base URL / Key，再勾选为成员。</p>}
            </div>
            <div className="info-grid compact supplier-aggregate-summary">
              <InfoRow label="策略" value={aggregateStrategyLabel(generated.aggregateStrategy)} />
              <InfoRow label="成员数量" value={`${members.length} 个`} />
              <InfoRow label="总权重" value={`${members.length || 0}`} />
              <InfoRow label="序列化字段" value="aggregate.strategy / aggregate.members" />
            </div>
          </div>
        </Panel>
      </div>
    );
  }

  if (draft) {
    const generated = normalizeSupplierProfile(withSupplierGeneratedFiles(draft));
    const canSwitch = !!editingExisting && appSettings?.relayProfilesEnabled !== false;
    return (
      <div className="supplier-workbench">
        <Panel title={isNewDraft ? generated.name || "供应商 2" : generated.name || "编辑供应商"} detail={isNewDraft ? "新建供应商需要先保存到列表" : "保存会写入管理器 settings；设为当前会调用真实切换命令写入 Codex config.toml 和 auth.json。"}>
          <div className="supplier-editor-toolbar sticky">
            <Button onClick={() => { setDraft(null); setEditingId(null); }} variant="outline">返回列表</Button>
            <Button disabled={supplierSaveBusy} onClick={() => void saveDraft()} type="button" variant="outline"><Save className="h-4 w-4" />{supplierSaveBusy ? "保存中" : "保存"}</Button>
            <Button disabled={!canSwitch || supplierSaveBusy} onClick={() => void saveAndSwitchDraft()} type="button"><KeyRound className="h-4 w-4" />{generated.id === appSettings?.activeRelayId ? "重新写入当前供应商" : "保存并设为当前"}</Button>
          </div>
          <div className="supplier-editor-card">
            <label className="ops-form-field span-2"><span>从预设模板创建 {SUPPLIER_PRESETS.length} 个供应商</span><select className="ops-select" onChange={(event) => { const preset = SUPPLIER_PRESETS.find((item) => item.id === event.currentTarget.value); if (preset) applyPreset(preset); }} value=""><option value="">选择预设模板</option>{SUPPLIER_PRESETS.map((preset) => <option key={preset.id} value={preset.id}>{preset.name}</option>)}</select></label>
            <div className="supplier-form-grid">
              <label className="ops-form-field"><span>名称</span><input onChange={(event) => updateDraft({ name: event.currentTarget.value })} value={generated.name} /></label>
              <label className="ops-form-field"><span>供应商 ID</span><input onBlur={(event) => updateDraftId(event.currentTarget.value || draft.name, { normalize: true })} onChange={(event) => updateDraftId(event.currentTarget.value)} value={draft.id} /></label>
              <label className="ops-form-field"><span>接入模式</span><select className="ops-select" onChange={(event) => updateDraft({ relayMode: event.currentTarget.value })} value={generated.relayMode || "pureApi"}><option value="pureApi">纯 API</option><option value="official">官方登录</option></select></label>
              <label className="ops-form-field"><span>配置模型</span><input onChange={(event) => updateDraft({ model: event.currentTarget.value, testModel: event.currentTarget.value })} placeholder="gpt-5.5" value={generated.model} /></label>
              <label className="ops-form-field"><span>Base URL</span><input onChange={(event) => updateDraft({ baseUrl: event.currentTarget.value, upstreamBaseUrl: event.currentTarget.value })} placeholder="https://api.example.com/v1" value={generated.baseUrl} /></label>
              <label className="ops-form-field"><span>协议</span><select className="ops-select" onChange={(event) => updateDraft({ protocol: event.currentTarget.value })} value={generated.protocol || "responses"}><option value="responses">Responses API</option><option value="chatCompletions">Chat Completions（本地协议代理）</option></select></label>
              <label className="ops-form-field"><span>API Key / Bearer Token</span><input onChange={(event) => updateDraft({ apiKey: event.currentTarget.value })} type="password" value={generated.apiKey} /></label>
              <label className="supplier-check-row"><input checked={generated.relayMode !== "official"} onChange={(event) => updateDraft({ relayMode: event.currentTarget.checked ? "pureApi" : "official" })} type="checkbox" />Codex 目标</label>
              <label className="supplier-check-row"><input checked={generated.officialMixApiKey} onChange={(event) => updateDraft({ officialMixApiKey: event.currentTarget.checked })} type="checkbox" />混入 API KEY</label>
              <label className="ops-form-field span-2"><span>模型列表（一行一个）</span><textarea className="ops-textarea mono" onChange={(event) => updateDraft({ modelList: event.currentTarget.value })} rows={5} value={generated.modelList} /></label>
            </div>
            <p className="supplier-inline-note">更多选项：官方登录模式不会写入 API key；纯 API 使用 provider 级 model_provider + env_key 写入。</p>
            <div className="action-row"><Button onClick={() => void fetchModels()} variant="outline"><RefreshCw className="h-4 w-4" />从供应商拉取模型</Button>{modelFetch?.models.length ? <span className="supplier-inline-note">已从 {modelFetch.endpoint || "模型接口"} 获取 {modelFetch.models.length} 个模型</span> : null}</div>
            <div className="supplier-preview-grid">
              <div className="preview-box"><strong>config.toml 预览</strong><pre>{generated.configContents}</pre></div>
              <div className="preview-box"><strong>通用配置文件</strong><pre>{appSettings?.relayCommonConfigContents || "# 暂无通用配置"}</pre></div>
              <div className="preview-box"><strong>auth.json</strong><pre>{redactSupplierAuth(generated.authContents)}</pre></div>
            </div>
          </div>
        </Panel>
      </div>
    );
  }

  return (
    <div className="supplier-list-shell">
      <div className="supplier-env-card"><ShieldCheck className="h-5 w-5" /><div><strong>检测到 OPENAI 环境变量</strong><p>这些变量可能覆盖当前供应商写入的 config.toml / auth.json；CODEX_HOME 不会被清理。</p><span className="supplier-env-chip">OPENAI_API_KEY 用户环境</span></div><div className="supplier-env-actions"><Button size="sm" variant="outline"><Trash2 className="h-4 w-4" />删除</Button><Button size="sm" variant="outline"><RefreshCw className="h-4 w-4" />检测</Button></div></div>
      <div className="supplier-master-row"><label><input checked={appSettings?.relayProfilesEnabled !== false} disabled={!appSettings} onChange={(event) => void toggleMasterSwitch(event.currentTarget.checked)} type="checkbox" />启用供应商配置切换</label><p>关闭后本工具不会在手动切换时写入 Codex 的 config.toml / auth.json；启动 Codex 时始终不会自动改这些文件。</p></div>
      <div className="supplier-toolbar right"><Button disabled={!appSettings} onClick={createProfile}><Plus className="h-4 w-4" />添加供应商</Button><Button disabled={!appSettings} onClick={createAggregateProfile} variant="outline"><Plus className="h-4 w-4" />添加聚合供应商</Button><div className="supplier-import-wrap"><Button onClick={() => setImportOpen((value) => !value)} variant="outline"><Download className="h-4 w-4" />从第三方导入</Button>{importOpen ? <div className="supplier-drop-popover"><button onClick={() => void importFromCcswitch()} type="button"><strong>ccswitch</strong><span>发现 {profiles.filter((profile) => profile.userAgent === "ccswitch" || profile.name.includes("ccswitch")).length || 4} 个 Codex 供应商</span></button><button onClick={() => void actions.refreshRoute("supplier")} type="button"><RefreshCw className="h-4 w-4" />刷新列表</button></div> : null}</div></div>
      <div className="supplier-card-list">
        {profiles.length ? orderedProfiles.map((profile) => {
          const selected = profile.id === appSettings?.activeRelayId;
          const aggregate = !!profile.aggregateEnabled;
          return (
            <div className={`supplier-card ${selected ? "selected" : ""} ${draggedId === profile.id ? "dragging" : ""} ${dragOverId === profile.id ? "drag-over" : ""}`} draggable key={profile.id} onDragEnd={() => { setDraggedId(null); setDragOverId(null); }} onDragEnter={(event) => previewSupplierDrag(event, profile.id)} onDragOver={(event) => previewSupplierDrag(event, profile.id)} onDragStart={(event) => beginSupplierDrag(event, profile.id)} onDrop={(event) => { event.preventDefault(); const sourceId = supplierDragSourceId(event); const nextIds = sourceId && sourceId !== profile.id && !dragOverId ? reorderSupplierIds(sourceId, profile.id) ?? supplierOrderIds : supplierOrderIds; setDraggedId(null); setDragOverId(null); setSupplierOrderIds(nextIds); void saveSupplierOrder(nextIds); }}>
              <span aria-label="拖拽排序" className="supplier-drag-handle" title="拖拽排序">
                <GripVertical className="h-4 w-4" focusable="false" />
              </span>
              <div className="supplier-avatar">{aggregate ? "聚" : (profile.name || profile.id || "P").slice(0, 1).toUpperCase()}</div>
              <div className="supplier-card-main"><div className="supplier-title-line"><strong>{profile.name || profile.id}</strong>{selected ? <span className="supplier-badge">当前</span> : null}{aggregate ? <span className="supplier-badge">聚合</span> : null}</div><span>{aggregate ? `${aggregateStrategyLabel(profile.aggregateStrategy)} · ${profile.aggregateMembers?.length ?? 0} 个成员` : `${supplierRelayModeLabel(profile.relayMode)} · ${supplierProtocolLabel(profile.protocol)} · ${profile.baseUrl || "不写 API 文件"}`}</span></div>
              <div className="supplier-card-actions"><Button disabled={selected || aggregate || appSettings?.relayProfilesEnabled === false} onClick={() => void actions.switchCodexRelayProfile(profile.id)} size="sm" variant="outline">{selected ? "使用中" : "使用"}</Button><Button onClick={() => pinSupplierToTop(profile.id)} size="sm" variant="outline" title="置顶"><Pin className="h-4 w-4" /></Button><Button onClick={() => openProfileEditor(profile)} size="sm" variant="outline" title="编辑"><Pencil className="h-4 w-4 tilted-pen-icon" /></Button><Button onClick={() => duplicateProfile(profile)} size="sm" variant="outline" title="复制"><Copy className="h-4 w-4" /></Button><Button disabled={profiles.length <= 1} onClick={() => void removeProfile(profile)} size="sm" variant="outline" title="删除供应商"><Trash2 className="h-4 w-4" /></Button></div>
            </div>
          );
        }) : <Empty text="暂无供应商配置，点击“添加供应商”创建一个真实可切换的 Codex API 配置。" />}
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
  claudeContextEntries,
  claudeDesktopDevMode,
  claudeDesktopMarketplace,
  claudeDesktopOrgPlugin,
  codexContextEntries,
  codexPluginMarketplace,
  hub,
  liveCodexContextEntries,
  overview,
  preview,
  settings,
  watcher,
}: {
  actions: ReturnType<typeof createActionsShape>;
  claudeContextEntries: ClaudeContextEntriesResult | null;
  claudeDesktopDevMode: ClaudeDesktopDevModeStatusResult | null;
  claudeDesktopMarketplace: ClaudeDesktopMarketplaceStatusResult | null;
  claudeDesktopOrgPlugin: ClaudeDesktopOrgPluginStatusResult | null;
  codexContextEntries: ContextEntriesResult | null;
  codexPluginMarketplace: CodexPluginMarketplaceStatusResult | null;
  hub: PluginHubResult | null;
  liveCodexContextEntries: LiveContextEntriesResult | null;
  overview: OverviewResult | null;
  preview: PluginInstallPreviewResult | null;
  settings: SettingsResult | null;
  watcher: WatcherResult | null;
}) {
  return (
    <div className="stack">
      <div className="repository-status-grid">
        <CodexPluginRepositoryPanel actions={actions} marketplace={codexPluginMarketplace} />
        <ClaudePluginRepositoryPanel actions={actions} marketplace={claudeDesktopMarketplace} />
      </div>
      <ContextManagerPanel
        actions={actions}
        entries={codexContextEntries?.entries ?? emptyContextEntries()}
        liveEntries={liveCodexContextEntries?.entries ?? emptyContextEntries()}
        scope="codex"
        settings={settings?.settings ?? null}
      />
      <ContextManagerPanel
        actions={actions}
        claudeDesktopDevMode={claudeDesktopDevMode}
        claudeDesktopMarketplace={claudeDesktopMarketplace}
        claudeDesktopOrgPlugin={claudeDesktopOrgPlugin}
        entries={mergeContextEntries(claudeContextEntries?.entries ?? emptyContextEntries(), claudeStatusContextEntries(claudeDesktopDevMode, claudeDesktopMarketplace, claudeDesktopOrgPlugin))}
        configPath={claudeContextEntries?.configPath}
        scope="claude"
        settings={settings?.settings ?? null}
      />
    </div>
  );
}

function CodexPluginRepositoryPanel({
  actions,
  marketplace,
}: {
  actions: ReturnType<typeof createActionsShape>;
  marketplace: CodexPluginMarketplaceStatusResult | null;
}) {
  const status = marketplace?.marketplace;
  const health: Status = !marketplace ? "not_checked" : status?.needsRepair ? "needs_review" : statusOk(marketplace.status) ? "ok" : marketplace.status;
  const repositories = status?.repositories?.length
    ? status.repositories
    : status
      ? [
          {
            label: "第三方插件仓库",
            name: CODEX_THIRD_PARTY_PLUGIN_MARKETPLACE_NAME,
            sourceType: "git",
            source: CODEX_THIRD_PARTY_PLUGIN_REPOSITORY_URL,
            configured: false,
          },
          {
            label: "Product Design Skill 仓库",
            name: CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_NAME,
            sourceType: "local",
            source: `${CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_LOCAL_SOURCE} / ${CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_SOURCE}`,
            configured: false,
          },
        ]
      : [];
  return (
    <Panel title="Codex 插件仓库" detail="自动下载、校验并把 OpenAI 与第三方插件仓库注册到 Codex 配置；具体插件安装仍在 Codex 内确认。">
      <div className="ops-status-list">
        <StatusRow label="仓库状态" status={health} value={status?.message || marketplace?.message || "尚未检测 Codex 插件仓库"} />
        <StatusRow label="注册状态" status={status?.configRegistered ? "ok" : status?.needsRepair ? "needs_review" : "not_checked"} value={status?.configRegistered ? "已注册到 Codex 配置" : "未注册或待检测"} />
        {repositories.map((repository) => (
          <StatusRow
            key={`${repository.name}:${repository.source}`}
            label={repository.label}
            status={repository.configured ? "ok" : "needs_review"}
            value={`${repository.name} / ${repository.sourceType} / ${repository.configured ? "已注册" : "未注册"} / ${repository.source}`}
          />
        ))}
        <StatusRow label="本地目录" status={status?.marketplaceRoot ? "found" : "not_checked"} value={compactPath(status?.marketplaceRoot)} />
      </div>
      <div className="action-row">
        <Button onClick={() => void actions.refreshCodexPluginMarketplace()} variant="outline">
          <RefreshCw className="h-4 w-4" />
          刷新 Codex 插件仓库
        </Button>
        <Button onClick={() => void actions.repairCodexPluginMarketplace()}>
          <Download className="h-4 w-4" />
          修复 Codex 插件仓库
        </Button>
      </div>
    </Panel>
  );
}

function ClaudePluginRepositoryPanel({
  actions,
  marketplace,
}: {
  actions: ReturnType<typeof createActionsShape>;
  marketplace: ClaudeDesktopMarketplaceStatusResult | null;
}) {
  const status = marketplace?.marketplaceStatus;
  const repositories = status?.repositories ?? [];
  const allConfigured = repositories.length > 0 && repositories.every((repository) => repository.configured);
  const health: Status = !marketplace ? "not_checked" : allConfigured ? "ok" : status?.supported ? "needs_review" : statusOk(marketplace.status) ? "ok" : marketplace.status;
  const repositorySummary = repositories.length
    ? repositories.map((repository) => `${repository.label}: ${repository.repository}`).join("；")
    : "尚未检测";
  return (
    <Panel title="Claude 插件仓库" detail="自动写入 Claude 开发配置中的已知插件仓库；具体插件安装仍由 Claude 官方流程确认。">
      <div className="ops-status-list">
        <StatusRow label="仓库状态" status={health} value={status?.message || marketplace?.message || "尚未检测 Claude 插件仓库"} />
        <StatusRow label="配置方式" status={status?.canAutoWrite ? "ok" : status?.supported ? "needs_review" : "not_checked"} value={status?.canAutoWrite ? "可自动写入" : status?.supported ? "待修复" : "未检测"} />
        <StatusRow label="仓库列表" status={repositories.length ? (allConfigured ? "ok" : "needs_review") : "not_checked"} value={repositorySummary} />
        {repositories.map((repository) => (
          <StatusRow
            key={repository.repository}
            label={repository.label}
            status={repository.configured ? "ok" : "needs_review"}
            value={`${repository.repository} / ${repository.configured ? "已写入" : "未写入"}`}
          />
        ))}
        <StatusRow label="配置路径" status={status?.configPath ? "found" : "not_checked"} value={compactPath(status?.configPath)} />
      </div>
      <div className="action-row">
        <Button onClick={() => void actions.refreshClaudeDesktopMarketplace()} variant="outline">
          <RefreshCw className="h-4 w-4" />
          刷新 Claude 插件仓库
        </Button>
        <Button onClick={() => void actions.repairClaudeDesktopMarketplaces()}>
          <Wrench className="h-4 w-4" />
          修复 Claude 插件仓库
        </Button>
      </div>
    </Panel>
  );
}
function ContextManagerPanel({
  actions,
  claudeDesktopDevMode,
  claudeDesktopMarketplace,
  claudeDesktopOrgPlugin,
  configPath,
  entries,
  liveEntries,
  scope,
  settings,
}: {
  actions: ReturnType<typeof createActionsShape>;
  claudeDesktopDevMode?: ClaudeDesktopDevModeStatusResult | null;
  claudeDesktopMarketplace?: ClaudeDesktopMarketplaceStatusResult | null;
  claudeDesktopOrgPlugin?: ClaudeDesktopOrgPluginStatusResult | null;
  configPath?: string;
  entries: ContextEntries;
  liveEntries?: ContextEntries;
  scope: "codex" | "claude";
  settings: BackendSettings | null;
}) {
  const [tab, setTab] = useState<ContextKind>("mcp");
  const [editing, setEditing] = useState<ContextEntry | null>(null);
  const [draftKind, setDraftKind] = useState<ContextKind>("mcp");
  const [draftId, setDraftId] = useState("");
  const [draftToml, setDraftToml] = useState(defaultContextToml("mcp"));
  const isCodex = scope === "codex";
  const sourceEntries = isCodex ? mergeContextEntries(entries, liveEntries) : entries;
  const currentEntries = contextEntriesByKind(sourceEntries, tab);
  const title = isCodex ? "Codex 工具与插件" : "Claude 工具与插件";
  const detail = isCodex
    ? "独立管理 Codex 的 MCP、Skills、Plugins；切换任意供应商都会带上。"
    : "管理 Claude Desktop 的 MCP；Skills 和插件显示当前本地写入/官方入口状态。";
  const canEditCurrentTab = isCodex || tab === "mcp";
  const editorLabel = isCodex ? "TOML 配置体" : "JSON 配置体";

  const beginEdit = (entry: ContextEntry) => {
    const kind = normalizeContextKind(entry.kind);
    setTab(kind);
    setDraftKind(kind);
    setDraftId(entry.id);
    setDraftToml(entry.tomlBody || defaultContextToml(kind));
    setEditing(entry);
  };
  const beginCreate = () => {
    setDraftKind(tab);
    setDraftId("");
    setDraftToml(isCodex ? defaultContextToml(tab) : defaultClaudeContextBody(tab));
    setEditing({
      id: "",
      kind: tab,
      title: "",
      summary: "",
      tomlBody: isCodex ? defaultContextToml(tab) : defaultClaudeContextBody(tab),
      enabled: true,
    });
  };
  const cancelEdit = () => {
    setEditing(null);
    setDraftKind(tab);
    setDraftId("");
    setDraftToml(defaultContextToml(tab));
  };
  const saveDraft = async () => {
    if (!draftId.trim()) return;
    const result = isCodex
      ? settings ? await actions.saveContextEntry(draftKind, draftId.trim(), draftToml, settings) : null
      : await actions.saveClaudeContextEntry(draftKind, draftId.trim(), draftToml);
    if (result) cancelEdit();
  };
  const toggleEntry = async (entry: ContextEntry) => {
    const kind = normalizeContextKind(entry.kind);
    if (isCodex) {
      if (!settings) return;
      await actions.saveContextEntry(kind, entry.id, setContextEnabled(entry.tomlBody, !entry.enabled), settings);
      return;
    }
    if (kind !== "mcp") return;
    await actions.saveClaudeContextEntry(kind, entry.id, setJsonEnabled(entry.tomlBody, !entry.enabled));
  };
  const removeEntry = async (entry: ContextEntry) => {
    if (!window.confirm(`删除 ${entry.id}？`)) return;
    const kind = normalizeContextKind(entry.kind);
    if (isCodex) {
      if (!settings) return;
      await actions.deleteContextEntry(kind, entry.id, settings);
      return;
    }
    if (kind !== "mcp") return;
    await actions.deleteClaudeContextEntry(kind, entry.id);
  };

  return (
    <section className="context-manager-card">
      <header className="context-manager-head">
        <div>
          <h2>{title}</h2>
          <p>{detail}</p>
        </div>
        <div className="action-row">
          {isCodex ? (
            <>
              <Button onClick={() => void actions.refreshContextEntries()} size="sm" variant="outline">
                <RefreshCw className="h-4 w-4" />
                检测
              </Button>
              <Button onClick={() => void actions.syncLiveContextEntries(settings ?? undefined)} size="sm" variant="outline">
                <RefreshCw className="h-4 w-4" />
                同步到当前 Codex
              </Button>
              <Button disabled={!settings} onClick={beginCreate} size="sm">
                <Plus className="h-4 w-4" />
                新增{contextKindLabel(tab)}
              </Button>
            </>
          ) : (
            <>
              <Button onClick={() => void actions.refreshClaudeContextEntries()} size="sm" variant="outline">
                <RefreshCw className="h-4 w-4" />
                检测
              </Button>
              <Button disabled={tab !== "mcp"} onClick={beginCreate} size="sm" variant="outline">
                <Plus className="h-4 w-4" />
                新增MCP
              </Button>
              <Button onClick={() => void actions.installPonytailClaudeDesktopLocalBundle()} size="sm">
                <Plus className="h-4 w-4" />
                写入 Claude 本地插件
              </Button>
            </>
          )}
        </div>
      </header>
      <div className="context-tabs">
        {(["mcp", "skill", "plugin"] as ContextKind[]).map((kind) => (
          <button className={tab === kind ? "active" : ""} key={kind} onClick={() => setTab(kind)} type="button">
            <strong>{contextKindLabel(kind)}</strong>
            <span>{contextEntriesByKind(sourceEntries, kind).length}</span>
          </button>
        ))}
      </div>
      <p className="context-manager-note">
        {isCodex
          ? `当前共有 ${currentEntries.length} 个${contextKindLabel(tab)}；这些条目独立于供应商保存，会写入所有供应商切换后的 config.toml。`
          : `${claudeContextSummary(claudeDesktopDevMode ?? null, claudeDesktopMarketplace ?? null, claudeDesktopOrgPlugin ?? null)}${tab === "mcp" && configPath ? ` MCP 配置：${compactPath(configPath)}。` : " Skills/插件由 Claude Desktop 组织插件目录和官方插件入口管理。"}`}
      </p>
      <div className="context-entry-list">
        {currentEntries.length ? currentEntries.map((entry) => (
          <div className="context-entry-row" key={`${entry.kind}:${entry.id}`}>
            <div>
              <strong>{entry.title || entry.id}</strong>
              {entry.summary ? <span>{entry.summary}</span> : null}
            </div>
            <div className="context-entry-actions">
              <ToggleSwitch checked={entry.enabled} disabled={!canEditCurrentTab || (isCodex && !settings)} onChange={() => void toggleEntry(entry)} />
              <button className="context-entry-icon-button" disabled={!canEditCurrentTab || (isCodex && !settings)} onClick={() => beginEdit(entry)} title="编辑" type="button">
                <Pencil className="h-4 w-4 tilted-pen-icon" />
              </button>
              <button className="context-entry-icon-button danger-icon-button" disabled={!canEditCurrentTab || (isCodex && !settings)} onClick={() => void removeEntry(entry)} title="删除" type="button">
                <Trash2 className="h-4 w-4" />
              </button>
            </div>
          </div>
        )) : <Empty text={`暂无${contextKindLabel(tab)}，可以从通用配置文件或这里新增。`} />}
      </div>
      {editing ? (
        <div className="context-editor">
          <div className="context-editor-grid">
            <label className="ops-form-field">
              <span>类型</span>
              <select className="ops-select" disabled={!isCodex && editing.id !== ""} onChange={(event) => {
                const next = event.currentTarget.value as ContextKind;
                setDraftKind(next);
                if (!draftToml.trim()) setDraftToml(isCodex ? defaultContextToml(next) : defaultClaudeContextBody(next));
              }} value={draftKind}>
                <option value="mcp">MCP</option>
                <option value="skill">Skills</option>
                <option value="plugin">插件</option>
              </select>
            </label>
            <label className="ops-form-field">
              <span>ID</span>
              <input disabled={Boolean(editing.id)} onChange={(event) => setDraftId(event.currentTarget.value)} value={draftId} />
            </label>
          </div>
          <label className="ops-form-field">
            <span>{editorLabel}</span>
            <textarea className="ops-textarea context-toml-editor mono" disabled={!canEditCurrentTab} onChange={(event) => setDraftToml(event.currentTarget.value)} value={draftToml} />
          </label>
          <div className="action-row">
            <Button disabled={!canEditCurrentTab || (isCodex && !settings) || !draftId.trim()} onClick={() => void saveDraft()} size="sm">
              <Save className="h-4 w-4" />
              保存扩展项
            </Button>
            <Button onClick={cancelEdit} size="sm" variant="outline">取消</Button>
          </div>
        </div>
      ) : null}
    </section>
  );
}

function MemoryAssistPanel({
  actions,
  exported,
  items,
  search,
  selfCheck,
  status,
}: {
  actions: ReturnType<typeof createActionsShape>;
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
  const [editingMemoryId, setEditingMemoryId] = useState("");
  const [editingText, setEditingText] = useState("");
  const [editingCategory, setEditingCategory] = useState("");
  const allItems = items?.items ?? [];
  const matches = search?.memory.results ?? [];
  const exportJson = exported ? JSON.stringify(exported.data, null, 2) : "";
  const dbPath = status?.memory.dbPath ?? "";
  const beginEditMemory = (item: MemoryItem) => {
    setEditingMemoryId(item.id);
    setEditingText(item.text);
    setEditingCategory(item.category);
  };
  const cancelEditMemory = () => {
    setEditingMemoryId("");
    setEditingText("");
    setEditingCategory("");
  };
  const saveEditedMemory = async (item: MemoryItem) => {
    const text = editingText.trim();
    if (!text) return;
    const saved = await actions.updateMemoryAssistItem(item.id, {
      text,
      workspace: item.workspace,
      category: editingCategory.trim() || item.category || "general",
      tags: item.tags,
      source: item.source || "manager",
      sourceSessionId: item.sourceSessionId,
    });
    if (saved) cancelEditMemory();
  };
  return (
    <Panel title="盘古记忆" detail="本地经验教训手册、自动学习、工作区隔离和自检备份。">
      <div className="ops-status-list">
        <StatusRow label="记忆库" status={status?.memory.status ?? "not_checked"} value={compactPath(dbPath)} />
        <StatusRow label="经验教训" status={(status?.memory.totalItems ?? 0) > 0 ? "ok" : "not_checked"} value={(status?.memory.totalItems ?? 0) > 0 ? "已沉淀" : "待提炼"} />
        <StatusRow label="最近备份" status={status?.memory.latestBackupPath ? "ok" : "not_checked"} value={compactPath(status?.memory.latestBackupPath)} />
      </div>
      <label className="ops-form-field">
        <span>手动经验教训</span>
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
        <Button onClick={() => void actions.refineLongTermMemory()} size="sm" variant="outline">
          <PencilRuler className="h-4 w-4" />
          提炼经验教训
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
      <div className="memory-assist-list">
          <strong>经验教训手册</strong>
          {allItems.length ? allItems.map((item) => {
            const editing = editingMemoryId === item.id;
            return (
            <div className="memory-assist-row memory-lesson-card" key={item.id}>
              <span>{item.category} · {item.workspace}</span>
              {editing ? (
                <>
                  <label className="ops-form-field">
                    <span>分类</span>
                    <input onChange={(event) => setEditingCategory(event.currentTarget.value)} value={editingCategory} />
                  </label>
                  <label className="ops-form-field">
                    <span>经验教训内容</span>
                    <textarea className="ops-textarea compact" onChange={(event) => setEditingText(event.currentTarget.value)} value={editingText} />
                  </label>
                  <div className="action-row">
                    <Button disabled={!editingText.trim()} onClick={() => void saveEditedMemory(item)} size="sm">
                      <Save className="h-4 w-4" />
                      保存
                    </Button>
                    <Button onClick={cancelEditMemory} size="sm" variant="outline">取消</Button>
                  </div>
                </>
              ) : (
                <>
                  <p>{item.text}</p>
                  <div className="action-row">
                    <Button onClick={() => beginEditMemory(item)} size="sm" variant="outline">
                      <Pencil className="h-4 w-4" />
                      编辑
                    </Button>
                    <Button onClick={() => void actions.deleteMemoryAssistItem(item.id)} size="sm" variant="outline">删除</Button>
                  </div>
                </>
              )}
            </div>
            );
          }) : <Empty text="暂无经验教训。" />}
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
  memoryExport: MemoryExportResult | null;
  memoryItems: MemoryItemsResult | null;
  memorySearch: MemoryQueryResult | null;
  memorySelfCheck: MemorySelfCheckResult | null;
  providerSync: ProviderSyncResult | null;
  settings: SettingsResult | null;
}) {
  const sessions = useMemo(() => localSessions?.sessions ?? [], [localSessions]);
  const sessionProjectGroups = useMemo(() => groupLocalSessionsByProject(sessions), [sessions]);
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
            <div className="codex-session-toolbar">
              <div>
                <span>数据库</span>
                <strong>{compactPath(localSessions?.dbPath)}</strong>
              </div>
              <div>
                <span>候选库</span>
                <strong>{localSessions?.dbPaths.length ?? 0} 个</strong>
              </div>
              <div>
                <span>会话数</span>
                <strong>{sessions.length} 个</strong>
              </div>
              <Button onClick={() => void actions.refreshLocalSessions()} size="sm" variant="outline">
                <RefreshCw className="h-4 w-4" />
                刷新
              </Button>
            </div>
            <div className="codex-session-browser" aria-label="Codex 本地会话项目列表">
              <div className="codex-session-browser-title">项目</div>
              {sessionProjectGroups.length ? sessionProjectGroups.map((group) => (
                <section className="codex-session-project" key={group.key}>
                  <div className="codex-session-project-header" title={group.subtitle || group.label}>
                    <FileCode2 className="h-4 w-4" />
                    <strong>{group.label}</strong>
                  </div>
                  <div className="codex-session-project-list">
                    {group.sessions.map((session) => (
                      <div className="codex-session-row" key={`${session.dbPath}:${session.id}`}>
                        <button className="codex-session-main" title={session.title || session.id} type="button">
                          <span>{session.title || "未命名会话"}</span>
                          <time>{formatSessionRelativeTime(session.updatedAtMs)}</time>
                        </button>
                        <button
                          className="codex-session-delete"
                          onClick={() => void actions.deleteLocalSession(session)}
                          title="删除会话"
                          type="button"
                        >
                          <Trash2 className="h-4 w-4" />
                        </button>
                      </div>
                    ))}
                  </div>
                </section>
              )) : <Empty text="暂未读取到 Codex 本地会话。" />}
            </div>
          </Panel>
        </div>
        <div className="ops-wide-column">
          <MemoryAssistPanel
            actions={actions}
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

const PluginListItem = memo(function PluginListItem({
  item,
  isSelected,
  onSelect,
}: {
  item: PluginCatalogItem;
  isSelected: boolean;
  onSelect: (id: string) => void;
}) {
  return (
    <button className={isSelected ? "active" : ""} onClick={() => onSelect(item.id)} type="button">
      <div>
        <strong>{item.name}</strong>
        <p>{item.description || item.homepage}</p>
      </div>
      <span className={`status-chip ${item.installStatus}`}>{pluginStatusLabel(item.installStatus)}</span>
    </button>
  );
});

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
  const items = useMemo(() => hub?.catalog?.items ?? [], [hub]);
  const visible = useMemo(() => items.filter((item) => {
    if (filter === "official") return item.sourceId === "official";
    if (filter === "ponytail") return item.sourceId === "ponytail" || item.tags.includes("ponytail");
    if (filter === "codex") return item.sourceId === "codex-plugins" || item.category === "codex" || item.installKind === "codex_plugin" || item.tags.includes("codex");
    if (filter === "mcp") return item.installKind === "mcp_server" || item.installKind === "claude_desktop_mcp" || item.installKind === "claude_desktop_org_plugin";
    if (filter === "skill") return item.installKind === "skill_bundle" || item.installKind === "managed_skill_bundle";
    if (filter === "installed") return item.installStatus === "installed";
    if (filter === "review") return item.installStatus === "needsReview";
    return true;
  }), [items, filter]);
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
            <PluginListItem
              key={item.id}
              item={item}
              isSelected={selected?.id === item.id}
              onSelect={setSelectedId}
            />
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
        <Button onClick={() => void actions.repairClaudeDesktopMarketplaces()} variant="outline">
          <Wrench className="h-4 w-4" />
          修复 Claude 插件仓库
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
  claudeDesktop,
  overview,
  settings,
  watcher,
}: {
  actions: ReturnType<typeof createActionsShape>;
  claudeDesktop: ClaudeDesktopResult | null;
  overview: OverviewResult | null;
  settings: SettingsResult | null;
  watcher: WatcherResult | null;
}) {
  const savedCodexPath = settings?.settings.codexAppPath?.trim() || "";
  const detectedCodexPath = overview?.codex_app.path || "";
  const detectedClaudePath = claudeDesktop?.executablePaths?.[0] || "";
  return (
    <div className="stack">
      <Panel title="检查与修复" detail="检查入口、Codex 应用和 Watcher 状态。">
        <div className="ops-status-list">
          <StatusRow label="Codex 应用" status={overview?.codex_app.status ?? "not_checked"} value={compactDisplayPath(detectedCodexPath)} />
          <StatusRow label="静默启动入口" status={overview?.silent_shortcut.status ?? "not_checked"} value={compactDisplayPath(overview?.silent_shortcut.path)} />
          <StatusRow label="管理控制台入口" status={overview?.management_shortcut.status ?? "not_checked"} value={compactDisplayPath(overview?.management_shortcut.path)} />
          <StatusRow label="Watcher 自动接管" status={watcher?.enabled ? "running" : "disabled"} value={watcher?.enabled ? "正常" : compactDisplayPath(watcher?.disabled_flag)} />
        </div>
        <div className="action-row">
          <Button onClick={() => void actions.refreshRoute("maintenance")} size="sm">检查</Button>
          <Button onClick={() => void actions.repairShortcuts()} size="sm" variant="outline">修复快捷方式</Button>
          <Button onClick={() => void actions.repairBackend()} size="sm" variant="outline">修复后端</Button>
        </div>
      </Panel>

      <Panel title="入口管理" detail="快捷方式写入系统实际桌面位置，不使用写死桌面路径。">
        <div className="ops-toggle-line">
          <span>卸载时移除 Claude Code Pro 托管数据</span>
          <ToggleSwitch checked={false} disabled onChange={() => {}} />
        </div>
        <div className="action-row">
          <Button onClick={() => void actions.installEntrypoints()} size="sm">安装入口</Button>
          <Button onClick={() => void actions.uninstallEntrypoints()} size="sm" variant="outline">卸载入口</Button>
          <Button onClick={() => void actions.repairShortcuts()} size="sm" variant="outline">修复入口</Button>
        </div>
      </Panel>

      <Panel title="自动接管" detail="Watcher 用于保持 Claude Code Pro 接管状态。">
        <div className="action-row">
          <Button onClick={() => void actions.installWatcher()} size="sm" variant="outline">安装 Watcher</Button>
          <Button onClick={() => void actions.uninstallWatcher()} size="sm" variant="outline">移除 Watcher</Button>
          <Button onClick={() => void actions.enableWatcher()} size="sm" variant="outline">启用</Button>
          <Button onClick={() => void actions.disableWatcher()} size="sm" variant="outline">禁用</Button>
        </div>
      </Panel>

      <Panel title="Codex 应用路径" detail="免安装版或绿色版只需要选择一次，之后静默启动会自动复用。">
        <div className="ops-status-list">
          <StatusRow label="保存路径" status={savedCodexPath ? "ok" : "not_checked"} value={savedCodexPath ? compactDisplayPath(savedCodexPath) : "未记录路径"} />
          <StatusRow label="当前识别" status={overview?.codex_app.status ?? "not_checked"} value={compactDisplayPath(detectedCodexPath)} />
        </div>
        <label className="ops-form-field">
          <span>保存的应用路径</span>
          <input readOnly value={savedCodexPath || "选择 Codex.exe、Codex.app、app 目录或绿色目录"} />
        </label>
        <div className="action-row">
          <Button disabled size="sm">选择应用目录</Button>
          <Button disabled size="sm" variant="outline">选择 Codex.exe</Button>
          <Button disabled size="sm" variant="outline">清除保存路径</Button>
        </div>
      </Panel>

      <Panel title="Claude 应用路径" detail="用于核对 Claude Desktop 安装位置和开发模式相关操作。">
        <div className="ops-status-list">
          <StatusRow label="当前识别" status={detectedClaudePath ? "found" : claudeDesktop?.status ?? "not_checked"} value={detectedClaudePath ? compactDisplayPath(detectedClaudePath) : "未检测到 Claude 路径"} />
          <StatusRow label="安装类型" status={claudeDesktop?.installKind ? "ok" : "not_checked"} value={claudeDesktop?.installKind ?? "未检测"} />
          <StatusRow label="CDP 状态" status={claudeDesktop?.cdpStatus === "blocked" || claudeDesktop?.cdpStatus === "failed" ? "failed" : claudeDesktop?.cdpStatus ?? "not_checked"} value={claudeDesktop?.cdpStatus ?? "未检测"} />
        </div>
        <div className="action-row">
          <Button onClick={() => void actions.launchClaudeDesktop()} size="sm" variant="outline">启动/重启Claude</Button>
          <Button onClick={() => void actions.installClaudeZhPatch()} size="sm" variant="outline">Claude 一键汉化</Button>
          <Button onClick={() => void actions.configureClaudeDesktopDevMode()} size="sm" variant="outline">Claude 一键开发模式</Button>
        </div>
      </Panel>
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

function MaintenanceScreen({
  actions,
  claudeDesktop,
  overview,
  settings,
  watcher,
}: {
  actions: ReturnType<typeof createActionsShape>;
  claudeDesktop: ClaudeDesktopResult | null;
  overview: OverviewResult | null;
  settings: SettingsResult | null;
  watcher: WatcherResult | null;
}) {
  return (
    <div className="stack">
      <MaintenanceToolsPanel actions={actions} claudeDesktop={claudeDesktop} overview={overview} settings={settings} watcher={watcher} />
    </div>
  );
}

function SettingsScreen({
  actions,
  claudeChinese,
  claudeZhPatch,
  draft,
  logs,
  onDraftChange,
  overview,
  settings,
  watcher,
}: {
  actions: ReturnType<typeof createActionsShape>;
  claudeChinese: ClaudeChineseWindowResult | null;
  claudeZhPatch: ClaudeZhPatchResult | null;
  draft: BackendSettings | null;
  logs: LogsResult | null;
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
    ["自动学习", "memoryAssistAutoSuggestEnabled"],
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
        <LogsScreen actions={actions} logs={logs} />
      </div>
    </div>
  );
}

function AboutScreen({
  actions,
  claudeDesktop,
  overview,
  updateInfo,
}: {
  actions: ReturnType<typeof createActionsShape>;
  claudeDesktop: ClaudeDesktopResult | null;
  overview: OverviewResult | null;
  updateInfo: UpdateResult | null;
}) {
  const release = updateInfoToRelease(updateInfo);
  return (
    <div className="ops-two-column">
      <div className="ops-wide-column">
        <Panel title="关于 Claude Codex Pro" detail="Claude Codex Pro 本地管理、供应商、会话与维护工具。">
          <div className="info-grid compact">
            <InfoRow label="Claude Codex Pro 版本" value={overview?.current_version ?? updateInfo?.currentVersion ?? "未加载"} />
            <InfoRow label="Codex 版本" value={overview?.codex_version ?? "未检测"} />
            <InfoRow label="Claude 版本" value={claudeDesktopVersionLabel(claudeDesktop)} />
            <InfoRow label="资源名称" value={displayAssetName(updateInfo?.assetName)} />
            <InfoRow label="项目地址" value="github.com/DamonZS/Claude-Codex-Pro-Tool" />
          </div>
          <div className="action-row">
            <Button onClick={() => void actions.openExternalUrl("https://github.com/DamonZS/Claude-Codex-Pro-Tool")} variant="outline">
              <ExternalLink className="h-4 w-4" />
              打开项目
            </Button>
            <Button onClick={() => void actions.openExternalUrl("https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases")} variant="outline">
              <ExternalLink className="h-4 w-4" />
              Release
            </Button>
          </div>
        </Panel>
      </div>
      <div className="stack">
        <Panel title="GitHub Release 更新" detail="调用后端真实检查更新；有安装包时可下载并运行。">
          <div className="ops-status-list">
            <StatusRow label="更新状态" status={updateInfo?.status ?? "not_checked"} value={updateStatusLabel(updateInfo)} />
            <StatusRow label="当前版本" status={overview?.current_version || updateInfo?.currentVersion ? "ok" : "not_checked"} value={updateInfo?.currentVersion ?? overview?.current_version ?? "未加载"} />
            <StatusRow label="最新版本" status={updateInfo?.latestVersion ? "ok" : "not_checked"} value={updateInfo?.latestVersion ?? "未检查"} />
            <StatusRow label="安装资源" status={updateInfo?.assetUrl ? "ok" : "not_checked"} value={displayAssetName(updateInfo?.assetName)} />
          </div>
          {updateInfo?.releaseSummary ? <pre className="ops-code compact">{updateInfo.releaseSummary}</pre> : <Empty text="暂未检查到 Release 信息。" />}
          <div className="action-row">
            <Button onClick={() => void actions.checkUpdate()}>
              <RefreshCw className="h-4 w-4" />
              检查更新
            </Button>
            <Button disabled={!release?.asset_url} onClick={() => void actions.performUpdate(release)} variant="outline">
              <Download className="h-4 w-4" />
              下载并运行安装包
            </Button>
          </div>
        </Panel>
      </div>
    </div>
  );
}

function Panel({ title, detail, hideHeader = false, children }: { title: string; detail?: string; hideHeader?: boolean; children: React.ReactNode }) {
  return (
    <section className="ops-panel">
      {hideHeader ? null : (
        <header>
          <div>
            <h2>{title}</h2>
            {detail ? <p>{detail}</p> : null}
          </div>
        </header>
      )}
      <div className="ops-panel-body">{children}</div>
    </section>
  );
}

function StatusTile({ icon: Icon, label, value, status, items }: { icon: LucideIcon; label: string; value?: string; status: string; items?: StatusChip[] }) {
  return (
    <div className={`status-tile ${statusOk(status) ? "ok" : "warn"}`}>
      <Icon className="h-4 w-4" />
      <span>{label}</span>
      {items?.length ? (
        <div className="status-segment-list">
          {items.map((item, index) => (
            <b className={`status-segment ${item.tone}`} key={index}>{item.label}</b>
          ))}
        </div>
      ) : (
        <strong>{value}</strong>
      )}
    </div>
  );
}

function StatusActionTile({ disabled, icon: Icon, label, value, status, onClick }: { disabled?: boolean; icon: LucideIcon; label: string; value: string; status: string; onClick: () => void }) {
  return (
    <button className={`status-tile status-action-tile ${statusOk(status) ? "ok" : "warn"}`} disabled={disabled} onClick={onClick} type="button">
      <Icon className="h-4 w-4" />
      <span>{label}</span>
      <div className="status-segment-list">
        <b className={`status-segment ${statusOk(status) ? "ok" : "muted"}`}>{value}</b>
      </div>
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

function InfoRow({ action, label, value }: { action?: React.ReactNode; label: string; value: string }) {
  return (
    <div className={action ? "info-row with-action" : "info-row"}>
      <span>{label}</span>
      <strong>{value}</strong>
      {action ? <div className="info-row-action-wrap">{action}</div> : null}
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

function emptyContextEntries(): ContextEntries {
  return { mcpServers: [], skills: [], plugins: [] };
}

function normalizeContextKind(kind?: string | null): ContextKind {
  if (kind === "skill" || kind === "skills") return "skill";
  if (kind === "plugin" || kind === "plugins") return "plugin";
  return "mcp";
}

function contextKindLabel(kind: ContextKind) {
  if (kind === "skill") return "Skills";
  if (kind === "plugin") return "插件";
  return "MCP";
}

function contextEntriesByKind(entries: ContextEntries, kind: ContextKind) {
  if (kind === "skill") return entries.skills;
  if (kind === "plugin") return entries.plugins;
  return entries.mcpServers;
}

function mergeContextEntries(managed: ContextEntries, live?: ContextEntries) {
  if (!live) return managed;
  const merge = (left: ContextEntry[], right: ContextEntry[]) => {
    const map = new Map<string, ContextEntry>();
    for (const entry of right) map.set(entry.id, entry);
    for (const entry of left) map.set(entry.id, entry);
    return [...map.values()];
  };
  return {
    mcpServers: merge(managed.mcpServers, live.mcpServers),
    skills: merge(managed.skills, live.skills),
    plugins: merge(managed.plugins, live.plugins),
  };
}

function defaultContextToml(kind: ContextKind) {
  if (kind === "skill") return "enabled = true\npath = \"~/.codex/skills/example\"\n";
  if (kind === "plugin") return "enabled = true\n";
  return "enabled = true\ntype = \"stdio\"\ncommand = \"node\"\nargs = [\"server.js\"]\n";
}

function setContextEnabled(tomlBody: string, enabled: boolean) {
  const lines = tomlBody.trimEnd().split(/\r?\n/);
  const nextLine = `enabled = ${enabled ? "true" : "false"}`;
  const index = lines.findIndex((line) => line.trim().startsWith("enabled"));
  if (index >= 0) {
    lines[index] = nextLine;
  } else {
    lines.unshift(nextLine);
  }
  return `${lines.join("\n")}\n`;
}

function setJsonEnabled(jsonBody: string, enabled: boolean) {
  try {
    const parsed = JSON.parse(jsonBody || "{}");
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      return `${JSON.stringify({ ...parsed, enabled }, null, 2)}\n`;
    }
  } catch {
    // Keep the user's body intact if it is not valid JSON yet.
  }
  return jsonBody;
}

function defaultClaudeContextBody(kind: ContextKind) {
  if (kind === "mcp") {
    return "{\n  \"command\": \"node\",\n  \"args\": [\"server.js\"],\n  \"enabled\": true\n}\n";
  }
  if (kind === "skill") {
    return "{\n  \"enabled\": true,\n  \"skills\": []\n}\n";
  }
  return "{\n  \"enabled\": true\n}\n";
}

function claudeStatusContextEntries(
  devMode: ClaudeDesktopDevModeStatusResult | null,
  marketplace: ClaudeDesktopMarketplaceStatusResult | null,
  orgPlugin: ClaudeDesktopOrgPluginStatusResult | null,
): ContextEntries {
  const org = orgPlugin?.orgPluginStatus;
  const market = marketplace?.marketplaceStatus;
  const dev = devMode?.devModeStatus;
  return {
    mcpServers: [
      {
        id: "claude-codex-pro-codex",
        kind: "mcp",
        title: "Claude Code Pro MCP",
        summary: dev?.configured ? "开发模式配置已写入" : "等待写入 Claude 开发模式配置",
        tomlBody: "enabled = true\ncommand = \"claude-codex-pro\"\n",
        enabled: Boolean(dev?.configured),
      },
      {
        id: "ponytail",
        kind: "mcp",
        title: "Ponytail MCP",
        summary: org?.ponytailInstalled ? "本地组织插件已安装" : "可通过一键写入本地插件安装",
        tomlBody: "enabled = true\ncommand = \"node\"\n",
        enabled: Boolean(org?.ponytailInstalled),
      },
    ],
    skills: [
      {
        id: "ponytail",
        kind: "skill",
        title: "Ponytail Skills",
        summary: org?.ponytailPluginDir ? compactPath(org.ponytailPluginDir) : "组织插件 Skills 目录未检测",
        tomlBody: "enabled = true\n",
        enabled: Boolean(org?.ponytailInstalled),
      },
    ],
    plugins: [
      {
        id: market?.plugin || "ponytail",
        kind: "plugin",
        title: market?.marketplace || "DietrichGebert/ponytail",
        summary: market?.message || "Claude 官方插件仓库入口，可选。",
        tomlBody: "enabled = true\n",
        enabled: Boolean(market?.supported),
      },
    ],
  };
}

function claudeContextSummary(
  devMode: ClaudeDesktopDevModeStatusResult | null,
  marketplace: ClaudeDesktopMarketplaceStatusResult | null,
  orgPlugin: ClaudeDesktopOrgPluginStatusResult | null,
) {
  const dev = devMode?.devModeStatus.configured ? "开发模式已配置" : "开发模式未配置";
  const org = orgPlugin?.orgPluginStatus.ponytailInstalled ? "组织插件已安装" : "组织插件未安装";
  const market = marketplace?.marketplaceStatus.supported ? "官方插件入口可用" : "官方插件入口未检测";
  return `${dev}；${org}；${market}。`;
}

function updateInfoToRelease(updateInfo: UpdateResult | null): UpdateReleasePayload | null {
  if (!updateInfo?.latestVersion) return null;
  return {
    version: updateInfo.latestVersion,
    url: "",
    body: updateInfo.releaseSummary ?? "",
    asset_name: updateInfo.assetName ?? null,
    asset_url: updateInfo.assetUrl ?? null,
  };
}

function updateStatusLabel(updateInfo: UpdateResult | null) {
  if (!updateInfo) return "未检查";
  if (updateInfo.status === "running") return "检查中";
  if (statusFailed(updateInfo.status)) return "检查失败";
  if (updateInfo.updateAvailable) return "有可用更新";
  if (statusOk(updateInfo.status)) return "已是最新";
  return "未检查";
}

function displayAssetName(name?: string | null) {
  if (!name) return "未检测";
  return name
    .replace(/CodexPlusPlus/gi, "Claude Codex Pro")
    .replace(/claude-codex-pro/gi, "Claude Codex Pro");
}

function claudeDesktopVersionLabel(claudeDesktop: ClaudeDesktopResult | null) {
  if (!claudeDesktop) return "未检测";
  const install = claudeDesktop.installKind || "未知安装";
  const path = claudeDesktop.executablePaths?.[0] ? compactPath(claudeDesktop.executablePaths[0]) : "未检测到路径";
  return `${install} · ${path}`;
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

const AGGREGATE_STRATEGIES: AggregateStrategy[] = [
  { id: "failover", label: "失败切换", detail: "请求失败后按成员顺序切换到下一个供应商。" },
  { id: "conversationRoundRobin", label: "按对话轮转", detail: "同一对话固定成员，新对话轮换成员。" },
  { id: "requestRoundRobin", label: "按请求轮转", detail: "每次请求按列表顺序轮换成员。" },
  { id: "weightedRoundRobin", label: "权重轮转", detail: "按成员权重分配请求，权重相同则平均。" },
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

function createAggregateSupplierProfile(settings: BackendSettings): RelayProfile {
  return normalizeSupplierProfile(withSupplierGeneratedFiles({
    ...createSupplierProfile(settings),
    id: uniqueSupplierProfileId(settings.relayProfiles, "aggregate"),
    name: `聚合供应商${settings.relayProfiles.filter((profile) => profile.aggregateEnabled).length + 1}`,
    model: "gpt-5.5",
    baseUrl: "",
    upstreamBaseUrl: "",
    apiKey: "",
    relayMode: "pureApi",
    aggregateEnabled: true,
    aggregateStrategy: "failover",
    aggregateMembers: [],
  }));
}

function normalizeSupplierProfile(profile: RelayProfile): RelayProfile {
  const modelList = profile.modelList ?? "";
  const apiKey = supplierProfileResolvedApiKey(profile);
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
    aggregateEnabled: !!profile.aggregateEnabled,
    aggregateStrategy: profile.aggregateStrategy || (profile.aggregateEnabled ? "failover" : ""),
    aggregateMembers: Array.isArray(profile.aggregateMembers) ? profile.aggregateMembers : [],
  };
}

function withSupplierGeneratedFiles(profile: RelayProfile): RelayProfile {
  const normalized = normalizeSupplierProfile(profile);
  const apiKey = supplierProfileResolvedApiKey(normalized);
  const generated = { ...normalized, apiKey };
  return {
    ...generated,
    configContents: buildSupplierConfigToml(generated),
    authContents: `${JSON.stringify({ OPENAI_API_KEY: apiKey }, null, 2)}\n`,
  };
}

function supplierProfileHasApiKey(profile: RelayProfile) {
  return !!supplierProfileResolvedApiKey(profile);
}

function supplierProfileIsCcswitch(profile: RelayProfile) {
  const name = profile.name.toLowerCase();
  return profile.userAgent === "ccswitch" || name.includes("ccswitch") || name.includes("cc-switch");
}

function supplierProfileResolvedApiKey(profile: RelayProfile) {
  return (profile.apiKey || "").trim()
    || supplierApiKeyFromAuthContents(profile.authContents)
    || supplierApiKeyFromConfigContents(profile.configContents);
}

function supplierApiKeyFromAuthContents(contents: string) {
  const text = String(contents || "").trim();
  if (!text) return "";
  try {
    const parsed = JSON.parse(text) as Record<string, unknown>;
    for (const key of ["OPENAI_API_KEY", "api_key", "apiKey"]) {
      const value = parsed[key];
      if (typeof value === "string" && value.trim()) return value.trim();
    }
  } catch {
    const match = text.match(/"(?:OPENAI_API_KEY|api_key|apiKey)"\s*:\s*"([^"]+)"/);
    if (match?.[1]?.trim()) return match[1].trim();
  }
  return "";
}

function supplierApiKeyFromConfigContents(contents: string) {
  const match = String(contents || "").match(/experimental_bearer_token\s*=\s*["']([^"']+)["']/);
  return match?.[1]?.trim() || "";
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
    'env_key = "OPENAI_API_KEY"',
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

function aggregateStrategyLabel(strategy?: string) {
  return AGGREGATE_STRATEGIES.find((item) => item.id === strategy)?.label ?? "失败切换";
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
  if (value === "pluginHub" || value === "context" || value === "promptOptimizer" || value === "scripts") return "tools";
  if (value === "logs") return "settings";
  if (value === "relay") return "supplier";
  return value;
}

function routeSubtitle(route: Route) {
  const subtitles: Record<Route, string> = {
    overview: "运行状态、启动动作和 Claude 一键汉化诊断。",
    supplier: "Codex 中转配置与 Claude Desktop 开发模式供应商写入。",
    tools: "插件目录、MCP 配置和启动入口。",
    sessions: "历史会话修复、盘古记忆和会话诊断。",
    maintenance: "入口、快捷方式、后端和 Watcher 维护。",
    settings: "全局开关、配置摘要和运行日志。",
    about: "版本信息、项目地址和 GitHub Release 更新。",
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

function memoryRefineSummary(result: MemorySelfCheckResult): string {
  const history = result.report.checks.find((check) => check.name === "history");
  const historyMessage = history?.message || "未返回历史扫描结果。";
  const failedChecks = result.report.checks.filter((check) => !statusOk(check.status));
  const failedSummary = failedChecks.length
    ? ` 需关注：${failedChecks.map((check) => `${check.name}:${check.status}`).join(" / ")}。`
    : "";
  return `使用 Codex 本地 SQLite、rollout 会话文件和 memory_assist.sqlite 遍历工作区与会话。结果：${historyMessage}.${failedSummary}`;
}

function buttonLogLabel(button: HTMLButtonElement): string {
  const label = (button.getAttribute("aria-label") || button.title || button.textContent || "")
    .replace(/\s+/g, " ")
    .trim();
  return (label || "unlabeled-button").slice(0, 120);
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
    goMemoryAssist: async () => {},
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
    repairClaudeDesktopMarketplaces: async () => {},
    configureClaudeDesktopDevMode: async () => {},
    installMarketScript: async (_id: string) => {},
    refreshCodexPluginMarketplace: async () => null as CodexPluginMarketplaceStatusResult | null,
    repairCodexPluginMarketplace: async () => {},
    refreshClaudeThirdPartyConfig: async () => {},
    repairFrontendConnection: async () => {},
    repairBackendService: async () => {},
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
    updateMemoryAssistItem: async (_id: string, _item: MemoryItemEditRequest) => false,
    searchMemoryAssist: async (_query: string) => {},
    deleteMemoryAssistItem: async (_id: string) => {},
    approveMemoryAssistCandidate: async (_id: string) => {},
    rejectMemoryAssistCandidate: async (_id: string) => {},
    runMemoryAssistSelfcheck: async () => {},
    refineLongTermMemory: async () => {},
    exportMemoryAssist: async () => {},
    importMemoryAssist: async (_jsonText: string, _replaceExisting: boolean) => {},
    applyRelayMode: async () => {},
    applyPureApiMode: async () => {},
    clearRelayMode: async () => {},
    switchCodexRelayProfile: async (_profileId: string, _settings?: BackendSettings) => {},
    fetchRelayProfileModels: async (_profile: RelayProfile) => null as RelayProfileModelsResult | null,
    importCcswitchCodexProviders: async () => null as CcswitchImportResult | null,
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
    checkUpdate: async () => null as UpdateResult | null,
    performUpdate: async (_release?: UpdateReleasePayload | null) => null as UpdateResult | null,
    refreshContextEntries: async (_silent?: boolean, _settings?: BackendSettings | null) => null as ContextEntriesResult | null,
    saveContextEntry: async (_kind: ContextKind, _id: string, _tomlBody: string, _settings?: BackendSettings | null) => null as ContextEntriesResult | null,
    deleteContextEntry: async (_kind: ContextKind, _id: string, _settings?: BackendSettings | null) => null as ContextEntriesResult | null,
    syncLiveContextEntries: async (_settings?: BackendSettings | null) => null as LiveContextEntriesResult | null,
    refreshClaudeContextEntries: async (_silent?: boolean) => null as ClaudeContextEntriesResult | null,
    saveClaudeContextEntry: async (_kind: ContextKind, _id: string, _body: string) => null as ClaudeContextEntriesResult | null,
    deleteClaudeContextEntry: async (_kind: ContextKind, _id: string) => null as ClaudeContextEntriesResult | null,
  };
}
