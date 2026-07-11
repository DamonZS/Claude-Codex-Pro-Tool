// 全部前端共享类型定义。从 App.tsx 抽出（任务#3 组件拆分）。
// 纯类型模块，无运行时依赖。

export type Status = "ok" | "failed" | "not_implemented" | "not_checked" | string;

export type CommandResult<T> = T & {
  status: Status;
  message: string;
};

export type StatusChipTone = "ok" | "warn" | "muted";
export type StatusChip = {
  label: string;
  tone: StatusChipTone;
};

export type PathState = {
  status: string;
  path: string | null;
};

export type LaunchStatus = {
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

export type OverviewResult = CommandResult<{
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

export type AnnouncementItem = {
  id: string;
  type: string;
  badge?: string;
  title: string;
  description: string;
  buttonLabel?: string;
  url: string;
  highlights?: string[];
};

export type AdsResult = CommandResult<{
  version: number;
  ads: AnnouncementItem[];
}>;

export type ClaudeDesktopResult = CommandResult<{
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

export type RepairConnectionResult = CommandResult<{
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

export type ClaudeChineseWindowResult = CommandResult<{
  open: boolean;
  label: string;
  defaultUrl: string;
  injectionMode: string;
  cdpStatus: string;
  cdpBlocker: string;
  officialInstallKind: string;
}>;

export type ClaudeZhPatchStatus = {
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

export type ClaudeZhPatchResult = CommandResult<{
  status: ClaudeZhPatchStatus;
  changedFiles: string[];
  backupDir: string;
  logsPath: string;
}>;

export type BackendSettings = {
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
  memoryAssistLlmSummaryEnabled: boolean;
  memoryAssistMcpEnabled: boolean;
  memoryAssistMaxInjectedItems: number;
  memoryAssistWorkspaceMode: string;
  launchMode: "patch" | "relay";
  relayBaseUrl: string;
  relayApiKey: string;
  relayProfiles: RelayProfile[];
  relayCommonConfigContents: string;
  relayContextConfigContents: string;
  activeRelayId: string;
  activeClaudeRelayId: string;
  activeClaudeDesktopRelayId: string;
  relayTestModel: string;
  cliWrapperEnabled: boolean;
  cliWrapperBaseUrl: string;
  cliWrapperApiKey: string;
  cliWrapperApiKeyEnv: string;
};

export type RelayProfile = {
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
  codexCatalogJson?: string;
  userAgent: string;
  notes?: string;
  websiteUrl?: string;
  authField?: string;
  headerOverride?: string;
  bodyOverride?: string;
  hideAiSignature?: boolean;
  teammatesMode?: boolean;
  toolSearchEnabled?: boolean;
  maxThinkingEnabled?: boolean;
  disableAutoUpdate?: boolean;
  importSource?: string;
  targetApp?: SupplierTargetApp;
  apiFormat?: string;
  claudeDesktopMode?: string;
  routeEnabled?: boolean;
  routeMode?: string;
  modelMapping?: string;
  modelMappingEnabled?: boolean;
  modelMappingJson?: string;
  aggregateEnabled?: boolean;
  aggregateStrategy?: string;
  aggregateMembers?: string[];
};

export type SupplierSaveResult = {
  settings: BackendSettings;
  profile: RelayProfile;
};

export type SettingsResult = CommandResult<{
  settings: BackendSettings;
  settings_path: string;
  user_scripts: UserScriptInventory;
}>;

export type RelayProfileModelsResult = CommandResult<{
  models: string[];
  endpoint: string;
}>;

export type CcswitchImportResult = CommandResult<{
  dbPath: string;
  profiles: RelayProfile[];
  scanned: number;
}>;

export type ContextKind = "mcp" | "skill" | "plugin";

export type ContextEntry = {
  id: string;
  kind: string;
  title: string;
  summary: string;
  tomlBody: string;
  enabled: boolean;
};

export type ContextEntries = {
  mcpServers: ContextEntry[];
  skills: ContextEntry[];
  plugins: ContextEntry[];
};

export type ContextEntriesResult = CommandResult<{
  settings: BackendSettings;
  entries: ContextEntries;
}>;

export type LiveContextEntriesResult = CommandResult<{
  entries: ContextEntries;
}>;

export type ClaudeContextEntriesResult = CommandResult<{
  configPath: string;
  entries: ContextEntries;
}>;

export type UnifiedToolAppState = {
  enabled: boolean;
  available: boolean;
  toggleSupported: boolean;
  sourcePath: string;
};

export type UnifiedToolAsset = {
  id: string;
  kind: ContextKind;
  title: string;
  summary: string;
  source: string;
  claude: UnifiedToolAppState;
  codex: UnifiedToolAppState;
};

export type UnifiedToolInventory = {
  assets: UnifiedToolAsset[];
  counts: {
    total: number;
    rawDiscoveries: number;
    deduplicated: number;
    mcp: number;
    skills: number;
    plugins: number;
    codexEnabled: number;
    claudeEnabled: number;
  };
  scannedSources: string[];
  diagnostics: string[];
};

export type UnifiedToolInventoryResult = CommandResult<{
  inventory: UnifiedToolInventory;
}>;

// 阶段4 模块D：一键注册盘古记忆 MCP 到 Claude Desktop 与 Codex 两端的返回。
export type MemoryMcpRegisterPayload = {
  mcpBinaryPath: string;
  mcpBinaryExists: boolean;
  claudeDesktopConfigPath: string;
  claudeDesktopRegistered: boolean;
  codexConfigPath: string;
  codexRegistered: boolean;
  mcpEnabled: boolean;
  errors: string[];
};

export type SupplierPreset = {
  id: string;
  name: string;
  category: "official" | "cn_official" | "aggregator" | "third_party";
  baseUrl: string;
  protocol: "responses" | "chatCompletions";
  model: string;
  modelList?: string[];
  websiteUrl?: string;
  apiKeyUrl?: string;
  targetApp?: SupplierTargetApp;
  apiFormat?: string;
  claudeDesktopMode?: string;
  routeEnabled?: boolean;
  routeMode?: string;
  modelMappingEnabled?: boolean;
  modelMappingJson?: string;
};

export type SupplierTargetApp = "codex" | "claude" | "claude-desktop";

export type AggregateStrategy = {
  id: string;
  label: string;
  detail: string;
};

export type UserScriptInventory = {
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

export type ScriptMarketItem = {
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

export type ScriptMarketResult = CommandResult<{
  market: {
    status: string;
    message: string;
    indexUrl: string;
    updatedAt: string;
    scripts: ScriptMarketItem[];
  };
  user_scripts: UserScriptInventory;
}>;

export type UpdateReleasePayload = {
  version: string;
  url: string;
  body: string;
  asset_name: string | null;
  asset_url: string | null;
};

export type UpdateResult = CommandResult<{
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

export type CodexPluginMarketplaceStatus = {
  codexHome: string;
  marketplaceRoot: string | null;
  configRegistered: boolean;
  needsRepair: boolean;
  message: string;
  localSourcesReady: boolean;
  runtimeConfirmation: string;
  repositories?: Array<{
    label: string;
    name: string;
    sourceType: string;
    source: string;
    configured: boolean;
  }>;
};

export type CodexPluginMarketplaceStatusResult = CommandResult<{
  marketplace: CodexPluginMarketplaceStatus;
}>;

export type CodexPluginMarketplaceRepairResult = CommandResult<{
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

export type LocalSession = {
  id: string;
  title: string;
  cwd: string;
  modelProvider: string;
  archived: boolean;
  updatedAtMs: number | null;
  rolloutPath: string;
  dbPath: string;
};

export type LocalSessionsResult = CommandResult<{
  dbPath: string;
  dbPaths: string[];
  sessions: LocalSession[];
}>;

export type ClaudeSession = {
  id: string;
  title: string;
  cwd: string;
  modelProvider: string;
  archived: boolean;
  updatedAtMs: number | null;
  sourcePath: string;
  sourceKind: string;
  messageCount: number;
};

export type ClaudeSessionsResult = CommandResult<{
  sourceRoot: string;
  sourcePaths: string[];
  sessions: ClaudeSession[];
  warnings: string[];
}>;

export type LocalSessionProjectGroup = {
  key: string;
  label: string;
  subtitle: string;
  sessions: LocalSession[];
};

export type ClaudeSessionProjectGroup = {
  key: string;
  label: string;
  subtitle: string;
  sessions: ClaudeSession[];
};

export type MemoryItem = {
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
  // Tiering (phase 2): "active" or "archived"; archived is a soft, recoverable state.
  tier: string;
  // Stored base strength, boosted on each access.
  strength: number;
  // Unix seconds when archived (0 = active).
  archivedAt: number;
  // Read-time Ebbinghaus-decayed retention in 0..1 (exempt items report 1.0).
  retention: number;
  // Read-time flag: exempt from decay (manual / safety-rule / project-rule).
  exempt: boolean;
};

export type MemoryItemEditRequest = Pick<MemoryItem, "text" | "workspace" | "category" | "tags" | "source" | "sourceSessionId">;

export type MemoryCandidate = {
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

export type MemoryStatusResult = CommandResult<{
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
    injectSummaryCachePath?: string | null;
    totalItems: number;
    pendingCandidates: number;
    totalCaptures: number;
    captureProgress: {
      firstBaselineAt: number;
      lastScanAt: number;
      totalSources: number;
      codexSources: number;
      claudeSources: number;
      totalContextCount: number;
      newContextCount: number;
      skippedUnchangedSessions: number;
    };
    workspaces: Array<{ workspace: string; itemCount: number; pendingCount: number; captureCount: number; sessionCount: number; latestCaptureAt: number }>;
    latestBackupPath: string | null;
  };
}>;

export type MemoryItemsResult = CommandResult<{ items: MemoryItem[] }>;
export type MemoryItemResult = CommandResult<{ item: MemoryItem }>;
export type MemoryCandidateResult = CommandResult<{ candidate: MemoryCandidate }>;
export type MemoryQueryResult = CommandResult<{
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
export type MemoryExport = {
  schemaVersion: string;
  exportedAt: number;
  items: MemoryItem[];
  candidates: MemoryCandidate[];
};
export type MemoryExportResult = CommandResult<{ data: MemoryExport }>;
export type MemorySelfCheckResult = CommandResult<{
  report: {
    status: string;
    repaired: boolean;
    backupPath: string | null;
    checks: Array<{ name: string; status: string; message: string }>;
  };
}>;

export type ProviderSyncResult = CommandResult<{
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

export type DeleteLocalSessionResult = CommandResult<{
  session_id?: string;
  sessionId?: string;
  undo_token?: string | null;
  undoToken?: string | null;
  backup_path?: string | null;
  backupPath?: string | null;
}>;

export type DeleteClaudeSessionResult = CommandResult<{
  sessionId: string;
  backupPath: string | null;
}>;

export type PluginInstallKind =
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
export type PluginInstallStatus = "notInstalled" | "installed" | "needsReview" | "unsupported";

export type PluginCatalogItem = {
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

export type PluginCatalogSource = {
  id: string;
  label: string;
  url: string;
  status: string;
  message: string;
  itemCount: number;
};

export type PluginHubResult = CommandResult<{
  catalog: {
    updatedAt: string;
    sources: PluginCatalogSource[];
    items: PluginCatalogItem[];
  };
}>;

export type PluginInstallPreviewResult = CommandResult<{
  item: PluginCatalogItem;
  canInstall: boolean;
  action: string;
  command: string[];
  configDiff: string;
  message: string;
}>;

export type PluginInstallOutcomeResult = CommandResult<{
  item: PluginCatalogItem;
  preview: unknown;
  installed: boolean;
  installMessage?: string;
  stdout: string;
  stderr: string;
  backupPath: string | null;
}>;

export type PluginInstallOutcomePayload = Omit<PluginInstallOutcomeResult, "status" | "message">;

export type CodexHookTrustResult = CommandResult<{
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

export type McpbPackageResult = CommandResult<{
  package: {
    mcpbPath: string;
    manifestPath: string;
    opened: boolean;
    message: string;
  };
}>;

export type ClaudeDesktopOrgPluginStatusResult = CommandResult<{
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

export type ClaudeDesktopOrgPluginInstallResult = CommandResult<{
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

export type ClaudeDesktopMarketplaceStatusResult = CommandResult<{
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

export type ClaudeDesktopMarketplaceOpenResult = CommandResult<{
  outcome: {
    repaired: boolean;
    configPath: string;
    repositories: ClaudeDesktopMarketplaceStatusResult["marketplaceStatus"]["repositories"];
    message: string;
  };
  marketplaceStatus: ClaudeDesktopMarketplaceStatusResult["marketplaceStatus"];
}>;

export type ClaudeDesktopMarketplaceRepairResult = ClaudeDesktopMarketplaceOpenResult;

export type ClaudeDesktopDevModeStatusResult = CommandResult<{
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

export type ClaudeDesktopDevModeConfigureResult = CommandResult<{
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

export type ClaudeDesktopProviderPreviewResult = CommandResult<{
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

export type ClaudeDesktopProviderApplyResult = CommandResult<{
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

export type ClaudeDesktopLocalBundleResult = CommandResult<{
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

export type LogsResult = CommandResult<{
  path: string;
  text: string;
  lines: number;
}>;

export type WatcherPayload = {
  enabled: boolean;
  disabled_flag: string;
};

export type WatcherResult = CommandResult<WatcherPayload>;

export type InstallEntrypointsResult = CommandResult<{
  silent_shortcut: {
    installed: boolean;
    path: string | null;
  };
  management_shortcut: {
    installed: boolean;
    path: string | null;
  };
}>;

export type Route =
  | "overview"
  | "supplier"
  | "tools"
  | "sessions"
  | "memory"
  | "maintenance"
  | "settings"
  | "about";
export type LegacyRoute = "promptOptimizer" | "relay";

declare global {
  interface Window {
    __CLAUDE_CODEX_PRO_INITIAL_ROUTE?: Route | LegacyRoute;
  }
}
