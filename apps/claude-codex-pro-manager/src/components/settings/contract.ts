/**
 * Settings & routing contract shared with the Rust backend.
 * Field names are the camelCase JSON the Tauri commands exchange; the backend
 * tasks implement exactly these shapes. Layout and defaults follow CC Switch
 * 3.20.4 (MIT); WebDAV/S3 cloud sync is intentionally excluded.
 */
import type { CommandResult } from "@/types";

export type UiLanguage = "zh" | "en";
export type ThemeChoice = "light" | "dark" | "system";
export type TerminalChoice = "cmd" | "powershell" | "wt";
export type SkillStorageLocation = "ccp" | "unified";
export type SkillSyncMethod = "symlink" | "copy";
export type LogLevel = "error" | "warn" | "info" | "debug" | "trace";
/**
 * Agents with provider management (CC Switch AppType parity). Routing takeover
 * only applies to HTTP-proxied agents (ProxyAppId); the rest are config-file
 * ("additive") providers written directly to the agent's own config.
 */
export type AgentAppId =
  | "codex"
  | "claude"
  | "claude-desktop"
  | "gemini"
  | "grok"
  | "opencode"
  | "openclaw"
  | "hermes"
  | "pi"
  | "mcode"
  | "workbuddy"
  | "cursor";
export type ProxyAppId = "codex" | "claude" | "claude-desktop" | "gemini";

export const AGENT_APPS: ReadonlyArray<{ id: AgentAppId; label: string; routable: boolean }> = [
  { id: "claude", label: "Claude Code", routable: true },
  { id: "claude-desktop", label: "Claude Desktop", routable: true },
  { id: "codex", label: "Codex", routable: true },
  { id: "gemini", label: "Gemini", routable: true },
  { id: "grok", label: "Grok Build", routable: false },
  { id: "opencode", label: "OpenCode", routable: false },
  { id: "openclaw", label: "OpenClaw", routable: false },
  { id: "hermes", label: "Hermes", routable: false },
  { id: "pi", label: "Pi", routable: false },
  { id: "mcode", label: "MiniMax Code", routable: false },
  { id: "workbuddy", label: "WorkBuddy", routable: false },
  { id: "cursor", label: "Cursor", routable: false },
];

/** Device-level preferences (`load_app_preferences` / `save_app_preferences`). */
export type AppPreferences = {
  language: UiLanguage;
  visibleApps: AgentAppId[];
  showProjectSwitcher: boolean;
  skillStorageLocation: SkillStorageLocation;
  skillSyncMethod: SkillSyncMethod;
  preserveCodexOfficialAuthOnSwitch: boolean;
  unifyCodexSessionHistory: boolean;
  launchOnStartup: boolean;
  silentStartup: boolean;
  applyToClaudeCodePlugin: boolean;
  skipClaudeOnboarding: boolean;
  minimizeToTrayOnClose: boolean;
  preferredTerminal: TerminalChoice;
  /** Empty string = default directory. */
  /** Per-agent config directory overrides; empty or missing = default. */
  configDirs: Partial<Record<AgentAppId, string>>;
  backupIntervalHours: number;
  backupRetainCount: number;
  logEnabled: boolean;
  logLevel: LogLevel;
};

export const DEFAULT_APP_PREFERENCES: AppPreferences = {
  language: "zh",
  visibleApps: ["claude", "claude-desktop", "codex", "gemini", "grok", "opencode", "openclaw", "hermes", "pi", "mcode", "workbuddy", "cursor"],
  showProjectSwitcher: true,
  skillStorageLocation: "ccp",
  skillSyncMethod: "symlink",
  preserveCodexOfficialAuthOnSwitch: false,
  unifyCodexSessionHistory: false,
  launchOnStartup: false,
  silentStartup: false,
  applyToClaudeCodePlugin: false,
  skipClaudeOnboarding: false,
  minimizeToTrayOnClose: true,
  preferredTerminal: "cmd",
  configDirs: {},
  backupIntervalHours: 24,
  backupRetainCount: 10,
  logEnabled: true,
  logLevel: "info",
};

export type AppPreferencesResult = CommandResult<{
  preferences: AppPreferences;
  /** Resolved directories shown as placeholders when overrides are empty. */
  resolvedDirs: { app: string } & Partial<Record<AgentAppId, string>>;
}>;

/** Per-app routing parameters (`proxy_config` table row). */
export type AppProxyConfig = {
  appId: ProxyAppId;
  takeover: boolean;
  autoFailoverEnabled: boolean;
  maxRetries: number;
  streamingFirstByteTimeout: number;
  streamingIdleTimeout: number;
  nonStreamingTimeout: number;
  circuitFailureThreshold: number;
  circuitSuccessThreshold: number;
  circuitTimeoutSeconds: number;
  circuitErrorRateThreshold: number;
  circuitMinRequests: number;
};

export type RectifierConfig = {
  enabled: boolean;
  thinkingSignature: boolean;
  thinkingBudget: boolean;
  mediaFallback: boolean;
  mediaHeuristic: boolean;
};

export type RoutingConfig = {
  enabled: boolean;
  listenAddress: string;
  listenPort: number;
  enableLogging: boolean;
  showRoutingToggleOnMain: boolean;
  showFailoverToggleOnMain: boolean;
  apps: AppProxyConfig[];
  rectifier: RectifierConfig;
  globalProxyUrl: string;
  globalProxyUsername: string;
  /** Never returned by the backend; only sent when the user types a new one. */
  globalProxyPassword?: string;
};

export type RoutingStatus = {
  running: boolean;
  address: string;
  port: number;
  activeConnections: number;
  totalRequests: number;
  successRate: number;
  uptimeSeconds: number;
  currentProviders: Partial<Record<ProxyAppId, string>>;
};

export type FailoverQueueEntry = {
  providerId: string;
  name: string;
  priority: number;
  circuitState: "closed" | "open" | "half_open";
};

export type RoutingResult = CommandResult<{
  config: RoutingConfig;
  /** Named `runtime` so it never collides with CommandResult.status. */
  runtime: RoutingStatus;
  queues: Partial<Record<ProxyAppId, FailoverQueueEntry[]>>;
}>;

export type ProxyTestResult = CommandResult<{ ok: boolean; latencyMs?: number | null }>;
export type ProxyScanResult = CommandResult<{ candidates: string[] }>;

export type BackupEntry = { id: string; name: string; createdAt: string; sizeBytes: number };
export type BackupListResult = CommandResult<{ backups: BackupEntry[]; dir: string }>;
export type DataTransferResult = CommandResult<{ path?: string; safetyBackupId?: string }>;

/** Tauri command names (single source for the frontend). */
export const SETTINGS_COMMANDS = {
  loadPreferences: "load_app_preferences",
  savePreferences: "save_app_preferences",
  loadRouting: "load_routing_config",
  saveRouting: "save_routing_config",
  setRoutingEnabled: "set_routing_enabled",
  setAppTakeover: "set_routing_app_takeover",
  addFailoverQueue: "add_failover_queue_provider",
  removeFailoverQueue: "remove_failover_queue_provider",
  resetCircuitBreaker: "reset_circuit_breaker",
  testGlobalProxy: "test_global_proxy",
  scanLocalProxies: "scan_local_proxies",
  listBackups: "list_database_backups",
  createBackup: "create_database_backup",
  restoreBackup: "restore_database_backup",
  renameBackup: "rename_database_backup",
  deleteBackup: "delete_database_backup",
  exportData: "export_ccp_data",
  importData: "import_ccp_data",
} as const;
