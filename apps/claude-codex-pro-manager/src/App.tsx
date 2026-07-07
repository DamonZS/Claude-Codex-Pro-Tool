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
import {
  AGGREGATE_STRATEGIES,
  CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_LOCAL_SOURCE,
  CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_NAME,
  CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_SOURCE,
  CODEX_THIRD_PARTY_PLUGIN_MARKETPLACE_NAME,
  CODEX_THIRD_PARTY_PLUGIN_REPOSITORY_URL,
  MEMORY_ALL_WORKSPACES,
  MEMORY_GLOBAL_WORKSPACE,
  PLUGIN_REPOSITORY_REPAIR_PROMPT_KEY_PREFIX,
  PONYTAIL_REPOSITORY_URL,
  PROMPT_OPTIMIZER_URL,
  SUPPLIER_DRAG_MIME_TYPE,
  SUPPLIER_PRESETS,
} from "@/constants";
import {
  afterFirstPaint,
  buttonLogLabel,
  claudeDesktopMarketplaceNeedsRepair,
  claudeOverviewStatus,
  codexLaunchRequestFromOverview,
  codexOverviewStatus,
  codexPluginMarketplaceNeedsRepair,
  compactDisplayPath,
  compactPath,
  displayProductPath,
  formatSessionRelativeTime,
  groupLocalSessionsByProject,
  localSessionProjectLabel,
  memoryOverviewStatus,
  memoryRefineSummary,
  pathTail,
  pluginRepositoryRepairPromptKey,
  pluginRepositoryRepairPromptMessage,
  statusFailed,
  statusOk,
  stringifyError,
  waitForPaint,
  zhPatchNoticeMessage,
} from "@/lib/helpers";
import {
  initialRoute,
  isRoute,
  normalizeRoute,
  routeDocumentTitle,
  routeLabel,
  routes,
  routeSubtitle,
} from "@/lib/routes";
import {
  aggregateStrategyLabel,
  buildSupplierConfigToml,
  createAggregateSupplierProfile,
  createSupplierProfile,
  firstSupplierModel,
  normalizeSupplierProfile,
  redactSupplierAuth,
  supplierApiKeyFromAuthContents,
  supplierApiKeyFromConfigContents,
  supplierCategoryLabel,
  supplierIdFromName,
  supplierProfileHasApiKey,
  supplierProfileIsCcswitch,
  supplierProfileResolvedApiKey,
  supplierProtocolLabel,
  supplierRelayModeLabel,
  tomlString,
  uniqueSupplierProfileId,
  withSupplierGeneratedFiles,
} from "@/lib/supplier";
import {
  claudeContextSummary,
  claudeStatusContextEntries,
  contextEntriesByKind,
  contextKindLabel,
  defaultClaudeContextBody,
  defaultContextToml,
  emptyContextEntries,
  mergeContextEntries,
  normalizeContextKind,
  setContextEnabled,
  setJsonEnabled,
} from "@/lib/context";
import {
  pluginCanInstall,
  pluginInstallButtonLabel,
  pluginKindLabel,
  pluginStatusLabel,
} from "@/lib/plugin";
import {
  claudeDesktopVersionLabel,
  displayAssetName,
  updateInfoToRelease,
  updateStatusLabel,
} from "@/lib/update";
import {
  ActionButton,
  Empty,
  InfoRow,
  Notice,
  Panel,
  StatusActionTile,
  StatusRow,
  StatusTile,
  ToggleSwitch,
} from "@/components/ui/ops";
import {
  AboutScreen,
  MaintenanceScreen,
  OverviewScreen,
  SessionManagementScreen,
  SettingsScreen,
  SupplierScreen,
  ToolsAndPluginsScreen,
} from "@/screens";
import type { AppActions } from "@/lib/actions";
import type {
  AggregateStrategy,
  BackendSettings,
  CcswitchImportResult,
  ClaudeChineseWindowResult,
  ClaudeContextEntriesResult,
  ClaudeDesktopDevModeConfigureResult,
  ClaudeDesktopDevModeStatusResult,
  ClaudeDesktopLocalBundleResult,
  ClaudeDesktopMarketplaceOpenResult,
  ClaudeDesktopMarketplaceRepairResult,
  ClaudeDesktopMarketplaceStatusResult,
  ClaudeDesktopOrgPluginInstallResult,
  ClaudeDesktopOrgPluginStatusResult,
  ClaudeDesktopProviderApplyResult,
  ClaudeDesktopProviderPreviewResult,
  ClaudeDesktopResult,
  ClaudeZhPatchResult,
  ClaudeZhPatchStatus,
  CodexHookTrustResult,
  CodexPluginMarketplaceRepairResult,
  CodexPluginMarketplaceStatus,
  CodexPluginMarketplaceStatusResult,
  CommandResult,
  ContextEntries,
  ContextEntriesResult,
  ContextEntry,
  ContextKind,
  DeleteLocalSessionResult,
  InstallEntrypointsResult,
  LaunchStatus,
  LegacyRoute,
  LiveContextEntriesResult,
  LocalSession,
  LocalSessionProjectGroup,
  LocalSessionsResult,
  LogsResult,
  McpbPackageResult,
  MemoryCandidate,
  MemoryCandidateResult,
  MemoryExport,
  MemoryExportResult,
  MemoryItem,
  MemoryItemEditRequest,
  MemoryItemResult,
  MemoryItemsResult,
  MemoryMcpRegisterPayload,
  MemoryQueryResult,
  MemorySelfCheckResult,
  MemoryStatusResult,
  OverviewResult,
  PathState,
  PluginCatalogItem,
  PluginCatalogSource,
  PluginHubResult,
  PluginInstallKind,
  PluginInstallOutcomeResult,
  PluginInstallPreviewResult,
  PluginInstallStatus,
  ProviderSyncResult,
  RelayProfile,
  RelayProfileModelsResult,
  RepairConnectionResult,
  Route,
  ScriptMarketResult,
  SettingsResult,
  Status,
  StatusChip,
  SupplierPreset,
  SupplierSaveResult,
  UpdateReleasePayload,
  UpdateResult,
  UserScriptInventory,
  WatcherResult,
} from "@/types";

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

  const refreshMemoryAssist = async (silent = false, includeArchived = false) => {
    const [status, items] = await Promise.all([
      refreshMemoryAssistStatus(silent),
      run(() => call<MemoryItemsResult>("list_memory_assist_items", { request: { workspace: MEMORY_ALL_WORKSPACES, includeGlobal: true, limit: 80, includeArchived } }), "记忆列表", { trackBusy: !silent, notify: !silent }),
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
      setNotice({ title: "Ponytail Codex Hooks", message: "未发现未信任的 Ponytail hook。", status: "ok" });
      return;
    }
    const details = pending.map((hook) => `${hook.eventName}: ${hook.command}`).join("\n\n");
    if (!window.confirm(`是否信任以下 Ponytail Codex hooks？\n\n${details}`)) return;
    const result = await run(() => call<CodexHookTrustResult>("trust_ponytail_codex_hooks"), "信任 Ponytail Hooks");
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

  const searchMemoryAssist = async (query: string, includeArchived = false) => {
    const result = await run(
      () => call<MemoryQueryResult>("query_memory_assist", { request: { query, workspace: MEMORY_ALL_WORKSPACES, includeGlobal: true, limit: 12, includeArchived } }),
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

  const archiveMemoryAssistItem = async (id: string) => {
    const result = await run(() => call<MemoryItemResult>("archive_memory_assist_item", { request: { id } }), "归档记忆");
    if (result) {
      notifyResult({ title: "盘古记忆", message: result.message, status: result.status });
      await refreshMemoryAssist(true);
    }
  };

  const restoreMemoryAssistItem = async (id: string) => {
    const result = await run(() => call<MemoryItemResult>("restore_memory_assist_item", { request: { id } }), "恢复记忆");
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

  const registerMemoryMcpServer = async () => {
    setNotice({
      title: "注册盘古记忆 MCP",
      message: "正在把盘古记忆 MCP server 写入 Claude Desktop 与 Codex 配置...",
      status: "running",
    });
    await waitForPaint();
    void writeUiEvent("memory.register_mcp.click", { targets: ["claude_desktop", "codex"] });
    const result = await run(
      () => call<CommandResult<MemoryMcpRegisterPayload>>("register_memory_mcp_server"),
      "注册盘古记忆 MCP",
    );
    if (result) {
      notifyResult({ title: "注册盘古记忆 MCP", message: result.message, status: result.status });
      await refreshSettings(true);
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

  const refreshRoute = async (target = route, options: { notify?: boolean } = {}) => {
    const shouldNotify = options.notify === true;
    const refreshTitle = `刷新${routeLabel(target)}`;
    if (shouldNotify) {
      setNotice({ title: refreshTitle, message: `正在刷新${routeLabel(target)}状态...`, status: "running" });
      await waitForPaint();
    }
    // Capture this load's epoch so trailing work can bail if the user has
    // navigated to another route (or re-triggered this one) in the meantime.
    const loadEpoch = routeLoadEpochRef.current + 1;
    routeLoadEpochRef.current = loadEpoch;
    const isStaleRouteLoad = () => routeLoadEpochRef.current !== loadEpoch;
    const afterFirstPaintIfFresh = (work: () => Promise<void> | void, delay?: number) => {
      afterFirstPaint(() => {
        if (isStaleRouteLoad()) return;
        void work();
      }, delay);
    };
    if (target === "overview") {
      await Promise.all([refreshOverview(true), refreshClaudeLight(true), refreshClaudeDesktopDevMode(true), refreshSettings(true)]);
      afterFirstPaintIfFresh(() => {
        void refreshMemoryAssistStatus(true);
      }, 250);
      afterFirstPaintIfFresh(() => {
        void refreshClaudeZhPatch(true);
      }, 650);
    } else if (target === "settings") {
      await refreshSettings(true);
      afterFirstPaintIfFresh(() => {
        void refreshLogs(true);
      }, 250);
    } else if (target === "supplier") {
      await Promise.all([refreshSettings(true), refreshClaudeDesktopDevMode(true)]);
    } else if (target === "tools") {
      const loadedSettings = await refreshSettings(true);
      await Promise.all([
        refreshPluginHub(true),
        refreshClaudeDesktopOrgPlugin(true),
        refreshClaudeDesktopDevMode(true),
        refreshClaudeContextEntries(true),
      ]);
      if (isStaleRouteLoad()) return;
      const sourceSettings = loadedSettings?.settings ?? settings?.settings ?? settingsDraft;
      if (sourceSettings) await refreshContextEntries(true, sourceSettings);
      afterFirstPaintIfFresh(async () => {
        const [codexMarketplaceStatus, claudeMarketplaceStatus] = await Promise.all([
          refreshCodexPluginMarketplace(true),
          refreshClaudeDesktopMarketplace(true),
        ]);
        if (isStaleRouteLoad()) return;
        await promptAndRepairPluginRepositories(codexMarketplaceStatus, claudeMarketplaceStatus);
      }, 250);
      afterFirstPaintIfFresh(() => {
        void Promise.all([refreshOverview(true), refreshClaude(true), refreshWatcher(true)]);
      }, 650);
    } else if (target === "sessions") {
      await Promise.all([
        refreshLocalSessions(true),
        refreshMemoryAssist(true),
        refreshSettings(true),
      ]);
      afterFirstPaintIfFresh(() => {
        void Promise.all([refreshOverview(true), refreshClaude(true)]);
      }, 250);
    } else if (target === "maintenance") {
      await Promise.all([refreshSettings(true), refreshClaudeLight(true)]);
      afterFirstPaintIfFresh(() => {
        void Promise.all([refreshOverview(true), refreshWatcher(true)]);
      }, 250);
    } else if (target === "about") {
      await Promise.all([refreshOverview(true), refreshClaudeLight(true)]);
      afterFirstPaintIfFresh(() => {
        void checkUpdate(true);
      }, 250);
    }
    if (shouldNotify && !isStaleRouteLoad()) {
      setNotice({ title: refreshTitle, message: `${routeLabel(target)}已刷新。`, status: "ok" });
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

  const actionsRef = useRef<AppActions | null>(null);
  actionsRef.current = {
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
      archiveMemoryAssistItem,
      restoreMemoryAssistItem,
      approveMemoryAssistCandidate,
      rejectMemoryAssistCandidate,
      runMemoryAssistSelfcheck,
      refineLongTermMemory,
      registerMemoryMcpServer,
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

  const actions = useMemo<AppActions>(() => ({
      refreshRoute: (...args) => actionsRef.current!.refreshRoute(...args),
      showNotice: (...args) => actionsRef.current!.showNotice(...args),
      openClaudeChinese: (...args) => actionsRef.current!.openClaudeChinese(...args),
      installClaudeZhPatch: (...args) => actionsRef.current!.installClaudeZhPatch(...args),
      installClaudeZhPatchFromDirectory: (...args) => actionsRef.current!.installClaudeZhPatchFromDirectory(...args),
      restoreClaudeZhPatch: (...args) => actionsRef.current!.restoreClaudeZhPatch(...args),
      launchClaudeDesktop: (...args) => actionsRef.current!.launchClaudeDesktop(...args),
      launchCodex: (...args) => actionsRef.current!.launchCodex(...args),
      restartCodex: (...args) => actionsRef.current!.restartCodex(...args),
      openExternalUrl: (...args) => actionsRef.current!.openExternalUrl(...args),
      goPluginHub: (...args) => actionsRef.current!.goPluginHub(...args),
      goMemoryAssist: (...args) => actionsRef.current!.goMemoryAssist(...args),
      goPromptOptimizer: (...args) => actionsRef.current!.goPromptOptimizer(...args),
      previewPlugin: (...args) => actionsRef.current!.previewPlugin(...args),
      installPlugin: (...args) => actionsRef.current!.installPlugin(...args),
      uninstallPlugin: (...args) => actionsRef.current!.uninstallPlugin(...args),
      previewPonytailCodexHooks: (...args) => actionsRef.current!.previewPonytailCodexHooks(...args),
      trustPonytailCodexHooks: (...args) => actionsRef.current!.trustPonytailCodexHooks(...args),
      generatePonytailMcpbInstaller: (...args) => actionsRef.current!.generatePonytailMcpbInstaller(...args),
      installPonytailClaudeDesktopOrgPlugin: (...args) => actionsRef.current!.installPonytailClaudeDesktopOrgPlugin(...args),
      installPonytailClaudeDesktopLocalBundle: (...args) => actionsRef.current!.installPonytailClaudeDesktopLocalBundle(...args),
      openClaudeDesktopOrgPluginsDir: (...args) => actionsRef.current!.openClaudeDesktopOrgPluginsDir(...args),
      openPonytailClaudeDesktopMarketplaceSetup: (...args) => actionsRef.current!.openPonytailClaudeDesktopMarketplaceSetup(...args),
      repairClaudeDesktopMarketplaces: (...args) => actionsRef.current!.repairClaudeDesktopMarketplaces(...args),
      configureClaudeDesktopDevMode: (...args) => actionsRef.current!.configureClaudeDesktopDevMode(...args),
      installMarketScript: (...args) => actionsRef.current!.installMarketScript(...args),
      refreshCodexPluginMarketplace: (...args) => actionsRef.current!.refreshCodexPluginMarketplace(...args),
      repairCodexPluginMarketplace: (...args) => actionsRef.current!.repairCodexPluginMarketplace(...args),
      refreshClaudeThirdPartyConfig: (...args) => actionsRef.current!.refreshClaudeThirdPartyConfig(...args),
      repairFrontendConnection: (...args) => actionsRef.current!.repairFrontendConnection(...args),
      repairBackendService: (...args) => actionsRef.current!.repairBackendService(...args),
      refreshPluginHub: (...args) => actionsRef.current!.refreshPluginHub(...args),
      refreshClaudeDesktopOrgPlugin: (...args) => actionsRef.current!.refreshClaudeDesktopOrgPlugin(...args),
      refreshClaudeDesktopMarketplace: (...args) => actionsRef.current!.refreshClaudeDesktopMarketplace(...args),
      refreshClaudeDesktopDevMode: (...args) => actionsRef.current!.refreshClaudeDesktopDevMode(...args),
      refreshScripts: (...args) => actionsRef.current!.refreshScripts(...args),
      repairEntrypoints: (...args) => actionsRef.current!.repairEntrypoints(...args),
      repairBackend: (...args) => actionsRef.current!.repairBackend(...args),
      repairHistorySessions: (...args) => actionsRef.current!.repairHistorySessions(...args),
      refreshLocalSessions: (...args) => actionsRef.current!.refreshLocalSessions(...args),
      deleteLocalSession: (...args) => actionsRef.current!.deleteLocalSession(...args),
      refreshMemoryAssist: (...args) => actionsRef.current!.refreshMemoryAssist(...args),
      learnMemoryAssistItem: (...args) => actionsRef.current!.learnMemoryAssistItem(...args),
      updateMemoryAssistItem: (...args) => actionsRef.current!.updateMemoryAssistItem(...args),
      searchMemoryAssist: (...args) => actionsRef.current!.searchMemoryAssist(...args),
      deleteMemoryAssistItem: (...args) => actionsRef.current!.deleteMemoryAssistItem(...args),
      archiveMemoryAssistItem: (...args) => actionsRef.current!.archiveMemoryAssistItem(...args),
      restoreMemoryAssistItem: (...args) => actionsRef.current!.restoreMemoryAssistItem(...args),
      approveMemoryAssistCandidate: (...args) => actionsRef.current!.approveMemoryAssistCandidate(...args),
      rejectMemoryAssistCandidate: (...args) => actionsRef.current!.rejectMemoryAssistCandidate(...args),
      runMemoryAssistSelfcheck: (...args) => actionsRef.current!.runMemoryAssistSelfcheck(...args),
      refineLongTermMemory: (...args) => actionsRef.current!.refineLongTermMemory(...args),
      registerMemoryMcpServer: (...args) => actionsRef.current!.registerMemoryMcpServer(...args),
      exportMemoryAssist: (...args) => actionsRef.current!.exportMemoryAssist(...args),
      importMemoryAssist: (...args) => actionsRef.current!.importMemoryAssist(...args),
      applyRelayMode: (...args) => actionsRef.current!.applyRelayMode(...args),
      applyPureApiMode: (...args) => actionsRef.current!.applyPureApiMode(...args),
      clearRelayMode: (...args) => actionsRef.current!.clearRelayMode(...args),
      switchCodexRelayProfile: (...args) => actionsRef.current!.switchCodexRelayProfile(...args),
      fetchRelayProfileModels: (...args) => actionsRef.current!.fetchRelayProfileModels(...args),
      importCcswitchCodexProviders: (...args) => actionsRef.current!.importCcswitchCodexProviders(...args),
      previewClaudeDesktopProvider: (...args) => actionsRef.current!.previewClaudeDesktopProvider(...args),
      applyClaudeDesktopProvider: (...args) => actionsRef.current!.applyClaudeDesktopProvider(...args),
      restoreClaudeDesktopProviderOfficial: (...args) => actionsRef.current!.restoreClaudeDesktopProviderOfficial(...args),
      saveSettings: (...args) => actionsRef.current!.saveSettings(...args),
      installEntrypoints: (...args) => actionsRef.current!.installEntrypoints(...args),
      uninstallEntrypoints: (...args) => actionsRef.current!.uninstallEntrypoints(...args),
      repairShortcuts: (...args) => actionsRef.current!.repairShortcuts(...args),
      installWatcher: (...args) => actionsRef.current!.installWatcher(...args),
      uninstallWatcher: (...args) => actionsRef.current!.uninstallWatcher(...args),
      enableWatcher: (...args) => actionsRef.current!.enableWatcher(...args),
      disableWatcher: (...args) => actionsRef.current!.disableWatcher(...args),
      resetSettings: (...args) => actionsRef.current!.resetSettings(...args),
      resetImageOverlaySettings: (...args) => actionsRef.current!.resetImageOverlaySettings(...args),
      refreshLogs: (...args) => actionsRef.current!.refreshLogs(...args),
      refreshWatcher: (...args) => actionsRef.current!.refreshWatcher(...args),
      checkUpdate: (...args) => actionsRef.current!.checkUpdate(...args),
      performUpdate: (...args) => actionsRef.current!.performUpdate(...args),
      refreshContextEntries: (...args) => actionsRef.current!.refreshContextEntries(...args),
      saveContextEntry: (...args) => actionsRef.current!.saveContextEntry(...args),
      deleteContextEntry: (...args) => actionsRef.current!.deleteContextEntry(...args),
      syncLiveContextEntries: (...args) => actionsRef.current!.syncLiveContextEntries(...args),
      refreshClaudeContextEntries: (...args) => actionsRef.current!.refreshClaudeContextEntries(...args),
      saveClaudeContextEntry: (...args) => actionsRef.current!.saveClaudeContextEntry(...args),
      deleteClaudeContextEntry: (...args) => actionsRef.current!.deleteClaudeContextEntry(...args),
  }), []);

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
