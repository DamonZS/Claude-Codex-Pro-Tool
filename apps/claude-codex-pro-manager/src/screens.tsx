import { type CSSProperties, type Dispatch, type PointerEvent as ReactPointerEvent, type ReactNode, type SetStateAction, memo, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import {
  Activity,
  ArrowLeft,
  AlertTriangle,
  Archive,
  ArchiveRestore,
  BarChart3,
  CheckCircle2,
  Copy,
  Download,
  Edit,
  ExternalLink,
  Eye,
  EyeOff,
  FileCode2,
  FileDown,
  FileUp,
  FolderOpen,
  GripVertical,
  Info,
  KeyRound,
  Languages,
  MessageCircle,
  Network,
  Pencil,
  PencilRuler,
  Pin,
  Play,
  Plus,
  Power,
  RefreshCw,
  Save,
  ShieldCheck,
  Trash2,
  Wrench,
  X,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import contactWechatQr from "@/assets/contact-wechat-qr.jpg";
import claudeLogo from "@/assets/claude.svg";
import codexLogo from "@/assets/openai.svg";
import { MemoryActivityWave } from "@/components/MemoryActivityWave";
import {
  AGGREGATE_STRATEGIES,
  CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_LOCAL_SOURCE,
  CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_NAME,
  CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_SOURCE,
  CODEX_THIRD_PARTY_PLUGIN_MARKETPLACE_NAME,
  CODEX_THIRD_PARTY_PLUGIN_REPOSITORY_URL,
  SUPPLIER_PRESETS,
} from "@/constants";
import type { AppActions } from "@/lib/actions";
import {
  ActionButton,
  Empty,
  InfoRow,
  Panel,
  StatusActionTile,
  StatusRow,
  StatusTile,
  ToggleSwitch,
} from "@/components/ui/ops";
import {
  claudeOverviewStatus,
  codexOverviewStatus,
  compactDisplayPath,
  compactPath,
  formatSessionRelativeTime,
  groupClaudeSessionsByProject,
  groupLocalSessionsByProject,
  memoryOverviewStatus,
  statusFailed,
  statusOk,
} from "@/lib/helpers";
import {
  aggregateStrategyLabel,
  createAggregateSupplierProfile,
  createSupplierProfile,
  normalizeSupplierProfile,
  redactSupplierConfig,
  supplierIdFromName,
  supplierProfileHasApiKey,
  supplierProfileIsCcswitch,
  supplierProtocolLabel,
  supplierRelayModeLabel,
  supplierTargetAppLabel,
  supplierApiFormatLabel,
  supplierApiFormatOption,
  supplierApiFormatRequiresRoute,
  SUPPLIER_API_FORMAT_OPTIONS,
  supplierCodexCatalogJson,
  supplierCodexCatalogModelList,
  supplierCodexCatalogRows,
  type SupplierCodexCatalogRow,
  supplierDirectModelIsClaudeDesktopSafe,
  supplierDirectModelList,
  supplierDirectModelRows,
  type SupplierDirectModelRow,
  supplierModelMappingJson,
  supplierModelMappingRows,
  supplierModelMappingText,
  uniqueSupplierProfileId,
  withSupplierGeneratedFiles,
  withSupplierPreservedImportedFiles,
} from "@/lib/supplier";
import { contextKindLabel, defaultClaudeContextBody, defaultContextToml } from "@/lib/context";
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
import type {
  AdsResult,
  BackendSettings,
  ClaudeChineseWindowResult,
  ClaudeDesktopDevModeStatusResult,
  ClaudeDesktopMarketplaceStatusResult,
  ClaudeDesktopOrgPluginStatusResult,
  ClaudeDesktopProviderApplyResult,
  ClaudeDesktopProviderPreviewResult,
  CredentialEnvironmentResult,
  ClaudeDesktopResult,
  ClaudeSession,
  ClaudeSessionContextPage,
  ClaudeSessionsResult,
  ClaudeZhPatchResult,
  CodexPluginMarketplaceStatusResult,
  CodexSessionContextPage,
  ContextKind,
  LocalSession,
  LocalSessionsResult,
  LogsResult,
  MemoryCandidatesResult,
  MemoryExportResult,
  MemoryItem,
  MemoryItemsResult,
  MemoryAssistMigrationResult,
  MemoryNewProjectExperience,
  MemoryNewProjectGuideResult,
  MemoryOutcomeDashboardResult,
  MemoryQueryResult,
  MemorySelfCheckResult,
  MemoryStatusResult,
  OverviewResult,
  PluginCatalogItem,
  PluginHubResult,
  PluginInstallPreviewResult,
  ProviderSyncResult,
  RelayProfile,
  RelayProfileModelsResult,
  SettingsResult,
  Status,
  SupplierPreset,
  SupplierSaveResult,
  SupplierTargetApp,
  UpdateResult,
  UnifiedToolAsset,
  UnifiedToolInventoryResult,
  WatcherResult,
} from "@/types";

type SupplierDirectModelDraftRow = SupplierDirectModelRow & {
  rowId: string;
};

type SupplierCodexCatalogDraftRow = SupplierCodexCatalogRow & {
  rowId: string;
};

const SUPPLIER_USER_AGENT_PRESETS = [
  "claude-cli/2.1.161 (external, cli)",
  "claude-cli/2.1.161",
  "claude-code/1.0.0",
  "claude-code/0.1.0",
  "Kilo-Code/1.0",
] as const;

export function OverviewScreen({
  actions,
  ads,
  overview,
  claudeDesktop,
  claudeZhPatch,
  claudeDesktopDevMode,
  claudeDevModeBusy,
  memoryAssist,
  memoryItems,
  settings,
}: {
  actions: AppActions;
  ads: AdsResult | null;
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
  const announcement = ads?.ads.find((item) => item.id === "official-toporeduce-api") ?? ads?.ads[0] ?? null;
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
  // 运行状态行只显示短标签，不再把后端 runtimeMessage（可能是整份经验教训全文）灌进 value。
  const memoryRuntimeValue = ({
    ok: "运行中",
    waiting: "等待 Codex 注入",
    disabled: "未开启",
    failed: "未运行",
    loading: "正在检测",
    not_checked: "尚未检测",
  } as Record<string, string>)[memoryRuntimeStatus] ?? "尚未检测";
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
      {announcement ? (
        <section className="overview-announcement-card" aria-label={announcement.badge?.trim() || "公告"}>
          <div className="overview-announcement-copy">
            <span className="overview-announcement-kicker">{announcement.badge?.trim() || "公告"}</span>
            <div>
              <h2>{announcement.title}</h2>
              <p>{announcement.description}</p>
            </div>
          </div>
          <Button onClick={() => void actions.openExternalUrl(announcement.url)} variant="outline">
            <ExternalLink className="h-4 w-4" />
            {announcement.buttonLabel?.trim() || "查看详情"}
          </Button>
        </section>
      ) : null}
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
              <div className="memory-title-row">
                <strong>盘古记忆开关</strong>
                <span className="memory-info" tabIndex={0}>
                  <Info className="h-3.5 w-3.5" />
                  <span className="memory-info-popover" role="tooltip">
                    <strong>盘古记忆是什么</strong>
                    <p>一份保存在本机的 agent 长期记忆库（<code>memory_assist.sqlite</code>）。它把你和 Codex / Claude 的每次协作里沉淀的经验教训、约定和结论记下来，下次自动喂回给 agent，让它记得住上下文、不再重复踩同一个坑。</p>
                    <strong>有多好</strong>
                    <ul>
                      <li>语义检索：按意思找记忆，不只是关键词匹配。</li>
                      <li>遗忘曲线：长期没用到的记忆自动淡出，常用的越记越牢，注入的上下文始终聚焦。</li>
                      <li>跨 agent 共享：通过 MCP，Claude Code / Cursor / Codex CLI 接入同一份大脑。</li>
                    </ul>
                    <strong>怎么用</strong>
                    <ul>
                      <li>打开上方开关，允许 Codex 读写本地记忆。</li>
                      <li>「提炼经验教训」把历史会话沉淀成手册；「查看/编辑」可手动增删。</li>
                      <li>在设置里开「盘古记忆 MCP」后，用一键注册把它接到 Claude Desktop / Codex。</li>
                    </ul>
                    <strong>怎么让 agent 越用越聪明</strong>
                    <p>用得越多，记忆越丰富、命中越频繁，高价值经验被强化、低价值的自动归档——大脑随使用持续自我打磨。手动固化的规则（约定、安全红线）常驻不衰减，始终生效。</p>
                  </span>
                </span>
              </div>
              <p>{memoryEnabled ? "已允许 Codex 使用本地经验教训与会话摘要。" : "当前不会向 Codex 注入盘古记忆。可在这里直接开启。"}</p>
            </div>
            <ToggleSwitch checked={memoryEnabled} disabled={!settings} onChange={(value) => void toggleMemoryAssistEnabled(value)} />
          </div>
          <div className="ops-status-list">
            <StatusRow label="运行状态" status={memoryRuntimeStatus} value={memoryRuntimeValue} />
            <StatusRow label="Codex 注入" status={memoryInjectStatus} value={memoryInjectValue} />
            <StatusRow label="对话监控" status={memoryMonitorStatus} value={memoryMonitorValue} />
          </div>
          <div className="ops-note">
            <Activity className="h-4 w-4" />
            <span>对话监控</span>
            <MemoryActivityWave active={memoryMonitorActive} />
          </div>
        </Panel>
        <div className="overview-side-stack">
          <Panel title="诊断与修复" detail="检查和修复入口集中在这里；修复动作会先显示运行反馈，再调用后端命令。">
            <ActionButton icon={RefreshCw} label="刷新概览" onClick={() => void actions.refreshRoute("overview", { notify: true })} />
            <ActionButton icon={RefreshCw} label="刷新 Claude 第三方配置" onClick={() => void actions.refreshClaudeThirdPartyConfig()} />
            <ActionButton icon={Wrench} label="修复前端连接" onClick={() => void actions.repairFrontendConnection()} />
            <ActionButton icon={Wrench} label="修复后端服务" onClick={() => void actions.repairBackendService()} />
            <ActionButton icon={Wrench} label="修复 Claude" onClick={() => void actions.restoreClaudeZhPatch()} />
          </Panel>
        </div>
      </div>
    </div>
  );
}

// Phase 2 tiering UI: a decay strength bar (exempt items read "常驻" and show
// full), plus an archive/restore action. Shared by the overview detail list and
// the memory management screen so both surfaces stay consistent.
const CONTACT_QQ_GROUP_PRIMARY_URL = "https://qm.qq.com/cgi-bin/qm/qr?k=uwNon9opx0Arfovyo5qJQQ2jUvlxSpmf&jump_from=webapi&authKey=El8Xwz9ZqefrpE4BhW9xWQsEAUFvptw74MBsRKRJTw5x5QiEPiG0fmdVIf9VuMWg";
const CONTACT_QQ_GROUP_SECONDARY_URL = "https://qm.qq.com/cgi-bin/qm/qr?k=cIeUYUFyy0ypTWMqo8CfgRwq8jU_OrXy&jump_from=webapi&authKey=njT7ceHMggvpptkiy9xD6FbBubVGCDof0cnX0adhLgUvi9kKZP4OY51M1xWZBy68";

function MemoryTierControls({ actions, item }: { actions: AppActions; item: MemoryItem }) {
  const archived = item.tier === "archived";
  const exempt = Boolean(item.exempt);
  // retention is 0..1; exempt items always report 1. Clamp for safety.
  const retention = exempt ? 1 : Math.max(0, Math.min(1, item.retention ?? 1));
  const pct = Math.round(retention * 100);
  return (
    <div className="memory-tier-controls">
      {archived ? (
        <span className="memory-tier-badge archived">已归档</span>
      ) : exempt ? (
        <span className="memory-tier-badge resident" title="常驻记忆，豁免遗忘衰减">
          <Pin className="h-3 w-3" />
          常驻
        </span>
      ) : (
        <span
          className="memory-strength-bar"
          title={`记忆强度 ${pct}%（越低越接近归档）`}
        >
          <span className="memory-strength-fill" style={{ width: `${pct}%` }} />
        </span>
      )}
      {archived ? (
        <Button onClick={() => void actions.restoreMemoryAssistItem(item.id)} size="sm" variant="outline">
          恢复
        </Button>
      ) : (
        <Button onClick={() => void actions.archiveMemoryAssistItem(item.id)} size="sm" variant="outline">
          <Archive className="h-4 w-4" />
          归档
        </Button>
      )}
    </div>
  );
}

export function OverviewMemoryDetails({
  actions,
  items,
  onClose,
}: {
  actions: AppActions;
  items: MemoryItemsResult | null;
  onClose: () => void;
}) {
  const [editingMemoryId, setEditingMemoryId] = useState("");
  const [editingText, setEditingText] = useState("");
  const [editingCategory, setEditingCategory] = useState("");
  const [showArchived, setShowArchived] = useState(false);
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
        <label className="memory-archive-toggle">
          <input
            checked={showArchived}
            onChange={(event) => {
              const next = event.currentTarget.checked;
              setShowArchived(next);
              void actions.refreshMemoryAssist(false, next);
            }}
            type="checkbox"
          />
          <span>显示归档</span>
        </label>
        <Button onClick={onClose} size="sm" variant="outline">收起</Button>
      </div>
      <div className="overview-memory-list">
        {allItems.length ? allItems.map((item) => {
          const editing = editingMemoryId === item.id;
          const archived = item.tier === "archived";
          return (
            <div className={`memory-assist-row memory-lesson-card${archived ? " memory-archived" : ""}`} key={item.id}>
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
                  <MemoryTierControls actions={actions} item={item} />
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

function SupplierModelDropdown({
  options,
  value,
  placeholder,
  onChange,
  compact = false,
  iconOnly = false,
  showAvailabilityWarning = true,
  triggerLabel,
}: {
  options: string[];
  value: string;
  placeholder: string;
  onChange: (value: string) => void;
  compact?: boolean;
  iconOnly?: boolean;
  showAvailabilityWarning?: boolean;
  triggerLabel?: string;
}) {
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState<CSSProperties>({});
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const menuRef = useRef<HTMLDivElement | null>(null);
  const valueAvailable = !showAvailabilityWarning || !value || options.includes(value);

  useEffect(() => {
    if (!open) return;
    const updatePosition = () => {
      const trigger = triggerRef.current;
      if (!trigger) return;
      const rect = trigger.getBoundingClientRect();
      const viewportWidth = window.innerWidth;
      const viewportHeight = window.innerHeight;
      const gap = 8;
      const spaceBelow = Math.max(0, viewportHeight - rect.bottom - gap);
      const spaceAbove = Math.max(0, rect.top - gap);
      const opensUp = spaceBelow < 180 && spaceAbove > spaceBelow;
      const availableSpace = opensUp ? spaceAbove : spaceBelow;
      const width = Math.min(Math.max(rect.width, 220), Math.max(120, viewportWidth - 16));
      const left = Math.min(Math.max(8, rect.left), Math.max(8, viewportWidth - width - 8));
      setPosition({
        left,
        width,
        maxHeight: Math.max(24, Math.min(320, availableSpace)),
        ...(opensUp
          ? { bottom: Math.max(8, viewportHeight - rect.top + gap) }
          : { top: Math.min(viewportHeight - 8, rect.bottom + gap) }),
      });
    };
    const closeOnOutsidePointer = (event: PointerEvent) => {
      const target = event.target as Node;
      if (!triggerRef.current?.contains(target) && !menuRef.current?.contains(target)) setOpen(false);
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        setOpen(false);
        triggerRef.current?.focus();
      }
    };
    updatePosition();
    document.addEventListener("pointerdown", closeOnOutsidePointer, true);
    document.addEventListener("keydown", closeOnEscape, true);
    window.addEventListener("resize", updatePosition);
    window.addEventListener("scroll", updatePosition, true);
    return () => {
      document.removeEventListener("pointerdown", closeOnOutsidePointer, true);
      document.removeEventListener("keydown", closeOnEscape, true);
      window.removeEventListener("resize", updatePosition);
      window.removeEventListener("scroll", updatePosition, true);
    };
  }, [open]);

  const menu = open ? createPortal(
    <div className="supplier-model-dropdown-menu" ref={menuRef} style={{ ...position, position: "fixed" }} role="listbox">
      {!valueAvailable && value ? <div className="supplier-model-dropdown-warning">当前配置不可用：{value}</div> : null}
      {options.length ? options.map((option) => (
        <button
          aria-selected={option === value}
          className={option === value ? "selected" : ""}
          key={option}
          onClick={() => {
            onChange(option);
            setOpen(false);
          }}
          role="option"
          type="button"
        >{option}</button>
      )) : <div className="supplier-model-dropdown-empty">暂无可用模型</div>}
    </div>,
    document.body,
  ) : null;

  return (
    <div className={`supplier-model-dropdown ${compact ? "compact" : ""} ${iconOnly ? "icon-only" : ""}`}>
      <button
        aria-label={triggerLabel || placeholder}
        aria-expanded={open}
        aria-haspopup="listbox"
        className={`supplier-model-dropdown-trigger ${!valueAvailable ? "unavailable" : ""}`}
        onClick={() => setOpen((current) => !current)}
        ref={triggerRef}
        type="button"
      >
        {iconOnly ? <span aria-hidden="true">▾</span> : <><span>{triggerLabel || value || placeholder}</span><span aria-hidden="true">▾</span></>}
      </button>
      {!valueAvailable && value ? <small className="supplier-model-dropdown-warning-inline">当前配置不可用</small> : null}
      {menu}
    </div>
  );
}

export function SupplierScreen({
  // 供应商主列表 / 编辑 / 聚合配置
  actions,
  settings,
  claudeDesktopDevMode,
  claudeDesktopProviderPreview,
  claudeDesktopProviderApply,
  claudeDesktopProviderDraft,
  credentialEnvironment,
  onClaudeDesktopProviderDraftChange,
}: {
  actions: AppActions;
  settings: SettingsResult | null;
  claudeDesktopDevMode: ClaudeDesktopDevModeStatusResult | null;
  claudeDesktopProviderPreview: ClaudeDesktopProviderPreviewResult | null;
  claudeDesktopProviderApply: ClaudeDesktopProviderApplyResult | null;
  credentialEnvironment: CredentialEnvironmentResult | null;
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
  const [supplierRefreshBusy, setSupplierRefreshBusy] = useState(false);
  const [credentialEnvironmentBusy, setCredentialEnvironmentBusy] = useState(false);
  const [importOpen, setImportOpen] = useState(false);
  const [showSupplierApiKey, setShowSupplierApiKey] = useState(false);
  const [supplierTestConfigOpen, setSupplierTestConfigOpen] = useState(false);
  const [supplierPricingConfigOpen, setSupplierPricingConfigOpen] = useState(false);
  const [supplierDirectModelsOpen, setSupplierDirectModelsOpen] = useState(true);
  const [supplierDirectModels, setSupplierDirectModels] = useState<SupplierDirectModelDraftRow[]>([]);
  const [supplierCodexCatalogModels, setSupplierCodexCatalogModels] = useState<SupplierCodexCatalogDraftRow[]>([]);
  const [supplierTargetFilter, setSupplierTargetFilter] = useState<SupplierTargetApp>("codex");
  const [draggedId, setDraggedId] = useState<string | null>(null);
  const [dragOverId, setDragOverId] = useState<string | null>(null);
  const [supplierOrderIds, setSupplierOrderIds] = useState<string[]>([]);
  const [supplierDragOverlay, setSupplierDragOverlay] = useState<{
    profileId: string;
    top: number;
    left: number;
    width: number;
    height: number;
    offsetY: number;
  } | null>(null);
  const supplierCardRefs = useRef<Map<string, HTMLDivElement>>(new Map());
  const supplierModelFetchRequestRef = useRef(0);
  const supplierDirectModelRowIdRef = useRef(0);
  const supplierCodexCatalogRowIdRef = useRef(0);
  const supplierPointerDragRef = useRef<{
    sourceId: string;
    latestIds: string[];
    lastTargetId: string | null;
  } | null>(null);
  const appSettings = settings?.settings ?? null;
  const profiles = useMemo(() => appSettings?.relayProfiles ?? [], [appSettings]);
  const profileIdsKey = profiles.map((profile) => profile.id).join("\u001f");
  const editingExisting = draft && editingId ? profiles.find((profile) => profile.id === editingId) : null;
  const isNewDraft = !!draft && !editingExisting;
  const aggregateProfiles = useMemo(() => profiles.filter((profile) => profile.aggregateEnabled), [profiles]);
  const apiProfiles = useMemo(() => profiles.filter((profile) => !profile.aggregateEnabled && profile.relayMode !== "official"), [profiles]);
  const supplierTargetForProfile = (profile: RelayProfile): SupplierTargetApp => profile.targetApp || "codex";
  const activeSupplierIdForTarget = (targetApp: SupplierTargetApp) => {
    if (!appSettings) return "";
    return targetApp === "claude"
      ? appSettings.activeClaudeRelayId
      : targetApp === "claude-desktop"
        ? appSettings.activeClaudeDesktopRelayId
        : appSettings.activeRelayId;
  };
  const supplierRoutingEnabledForTarget = (targetApp: SupplierTargetApp, sourceProfiles = profiles) => {
    return sourceProfiles.some((profile) => {
      const target = supplierTargetForProfile(profile);
      return (targetApp === "codex" ? target === "codex" : target === "claude" || target === "claude-desktop") && !!profile.routeEnabled;
    });
  };
  const withSupplierRoutingState = (profile: RelayProfile, targetApp: SupplierTargetApp, enabled: boolean) => normalizeSupplierProfile({
    ...profile,
    targetApp,
    routeEnabled: enabled,
    claudeDesktopMode: targetApp === "codex" ? "" : enabled ? "proxy" : "direct",
    routeMode: targetApp === "codex"
      ? (enabled ? "Codex Proxy" : "Codex Direct")
      : (enabled ? "Claude Desktop Proxy" : "Claude Desktop Direct"),
  });
  const withActiveSupplierId = (current: BackendSettings, targetApp: SupplierTargetApp, profileId: string): BackendSettings => {
    if (targetApp === "claude") return { ...current, activeClaudeRelayId: profileId };
    if (targetApp === "claude-desktop") return { ...current, activeClaudeDesktopRelayId: profileId };
    return { ...current, activeRelayId: profileId };
  };
  const updateClaudeDraft = (field: keyof typeof claudeDesktopProviderDraft, value: string) => {
    onClaudeDesktopProviderDraftChange((current) => ({ ...current, [field]: value }));
  };
  const createSupplierDirectModelRows = (rows: SupplierDirectModelRow[]): SupplierDirectModelDraftRow[] => rows.map((row) => ({
    ...row,
    rowId: `direct-model-${supplierDirectModelRowIdRef.current += 1}`,
  }));
  const createSupplierCodexCatalogModelRows = (rows: SupplierCodexCatalogRow[]): SupplierCodexCatalogDraftRow[] => rows.map((row) => ({
    ...row,
    rowId: `codex-catalog-${supplierCodexCatalogRowIdRef.current += 1}`,
  }));
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
    supplierModelFetchRequestRef.current += 1;
    setModelFetch(null);
    setShowSupplierApiKey(false);
    setSupplierDirectModels(createSupplierDirectModelRows(supplierDirectModelRows(profile.modelList)));
    setSupplierCodexCatalogModels(createSupplierCodexCatalogModelRows(supplierCodexCatalogRows(profile)));
    setSupplierDirectModelsOpen(true);
    setEditingId(profile.id);
    const targetApp = supplierTargetForProfile(profile);
    setDraft(withSupplierRoutingState(profile, targetApp, supplierRoutingEnabledForTarget(targetApp)));
  };
  const createProfile = () => {
    if (!appSettings) return;
    supplierModelFetchRequestRef.current += 1;
    setModelFetch(null);
    setShowSupplierApiKey(false);
    setEditingId(null);
    const targetApp = supplierTargetFilter;
    const profile = withSupplierRoutingState({ ...createSupplierProfile(appSettings), targetApp }, targetApp, supplierRoutingEnabledForTarget(targetApp));
    setSupplierDirectModels(createSupplierDirectModelRows(supplierDirectModelRows(profile.modelList)));
    setSupplierCodexCatalogModels(createSupplierCodexCatalogModelRows(supplierCodexCatalogRows(profile)));
    setSupplierDirectModelsOpen(true);
    setDraft(profile);
  };
  const createAggregateProfile = () => {
    if (!appSettings) return;
    supplierModelFetchRequestRef.current += 1;
    const profile = createAggregateSupplierProfile(appSettings);
    setModelFetch(null);
    setShowSupplierApiKey(false);
    setEditingId(null);
    setDraft(profile);
    if (!apiProfiles.length) {
      actions.showNotice({ title: "添加聚合供应商", message: "已打开聚合供应商详情；请先添加或选择至少 1 个普通 API 供应商的 Base URL / Key，再勾选为成员。", status: "failed" });
    }
  };
  const duplicateProfile = (profile: RelayProfile) => {
    if (!appSettings) return;
    supplierModelFetchRequestRef.current += 1;
    setShowSupplierApiKey(false);
    const targetApp = supplierTargetForProfile(profile);
    const copy = {
      ...withSupplierRoutingState(profile, targetApp, supplierRoutingEnabledForTarget(targetApp)),
      id: uniqueSupplierProfileId(appSettings.relayProfiles, `${profile.id || "provider"}-copy`),
      name: `${profile.name || profile.id || "供应商"} 副本`,
    };
    setModelFetch(null);
    setSupplierDirectModels(createSupplierDirectModelRows(supplierDirectModelRows(copy.modelList)));
    setSupplierCodexCatalogModels(createSupplierCodexCatalogModelRows(supplierCodexCatalogRows(copy)));
    setSupplierDirectModelsOpen(true);
    setEditingId(null);
    setDraft(copy);
  };
  const normalizeDraftProfile = (profile: RelayProfile) => supplierProfileIsCcswitch(profile)
    ? normalizeSupplierProfile(profile)
    : normalizeSupplierProfile(withSupplierGeneratedFiles(profile));
  const updateDraft = (patch: Partial<RelayProfile>) => {
    supplierModelFetchRequestRef.current += 1;
    setDraft((current) => current ? normalizeDraftProfile({ ...current, ...patch }) : current);
  };
  const closeSupplierEditor = () => {
    supplierModelFetchRequestRef.current += 1;
    setModelFetch(null);
    setDraft(null);
    setEditingId(null);
    setShowSupplierApiKey(false);
    setSupplierCodexCatalogModels([]);
  };
  const refreshCredentialEnvironment = async () => {
    if (credentialEnvironmentBusy) return;
    setCredentialEnvironmentBusy(true);
    try {
      await actions.diagnoseCodexCredentialEnvironment(false);
    } finally {
      setCredentialEnvironmentBusy(false);
    }
  };
  const clearCredentialEnvironment = async () => {
    if (!credentialEnvironment?.canClearUser || credentialEnvironmentBusy) return;
    const variableName = credentialEnvironment.variableName;
    if (!window.confirm(`确认删除用户环境变量「${variableName}」？不会修改 CODEX_HOME 或系统级环境变量。`)) return;
    setCredentialEnvironmentBusy(true);
    try {
      await actions.clearCodexUserCredentialEnvironment(variableName);
    } finally {
      setCredentialEnvironmentBusy(false);
    }
  };
  const updateNewDraftIdFromName = (value: string) => {
    if (!isNewDraft) return;
    supplierModelFetchRequestRef.current += 1;
    setDraft((current) => {
      if (!current) return current;
      const nextId = uniqueSupplierProfileId(profiles, value || current.name);
      const next = normalizeDraftProfile({ ...current, id: nextId });
      return normalizeSupplierProfile(next);
    });
  };
  const updateSupplierModelMapping = (role: string, field: "routeId" | "displayName" | "requestModel" | "supports1m", value: string | boolean) => {
    if (!draft) return;
    const rows = supplierModelMappingRows(draft).map((row) => row.role === role ? { ...row, [field]: value } : row);
    updateDraft({
      modelMappingEnabled: true,
      modelMappingJson: supplierModelMappingJson(rows),
      modelMapping: supplierModelMappingText(rows),
    });
  };
  const writeSupplierDirectModels = (rows: SupplierDirectModelDraftRow[]) => {
    setSupplierDirectModels(rows);
    const modelList = supplierDirectModelList(rows);
    const firstModel = rows.find((row) => row.model.trim())?.model.trim() || "";
    updateDraft({
      modelList,
      ...(firstModel && (!draft?.model.trim() || !rows.some((row) => row.model.trim() === draft.model.trim()))
        ? { model: firstModel, testModel: firstModel }
        : {}),
    });
  };
  const addSupplierDirectModel = () => {
    writeSupplierDirectModels([...supplierDirectModels, {
      model: "",
      rowId: `direct-model-${supplierDirectModelRowIdRef.current += 1}`,
      supports1m: false,
    }]);
  };
  const updateSupplierDirectModel = (rowId: string, patch: Partial<SupplierDirectModelRow>) => {
    const nextRows = supplierDirectModels.map((row) => row.rowId === rowId ? { ...row, ...patch } : row);
    writeSupplierDirectModels(nextRows);
  };
  const removeSupplierDirectModel = (rowId: string) => {
    writeSupplierDirectModels(supplierDirectModels.filter((row) => row.rowId !== rowId));
  };
  const writeSupplierCodexCatalogModels = (rows: SupplierCodexCatalogDraftRow[]) => {
    setSupplierCodexCatalogModels(rows);
    const codexCatalogJson = supplierCodexCatalogJson(rows);
    const modelList = supplierCodexCatalogModelList(rows);
    const firstRow = rows.find((row) => row.model.trim());
    updateDraft({
      codexCatalogJson,
      modelList,
      ...(firstRow ? {
        model: firstRow.model.trim(),
        testModel: firstRow.model.trim(),
        contextWindow: firstRow.contextWindow,
      } : {}),
    });
  };
  const addSupplierCodexCatalogModel = () => {
    writeSupplierCodexCatalogModels([...supplierCodexCatalogModels, {
      displayName: "",
      model: "",
      contextWindow: "",
      rowId: `codex-catalog-${supplierCodexCatalogRowIdRef.current += 1}`,
    }]);
  };
  const updateSupplierCodexCatalogModel = (rowId: string, patch: Partial<SupplierCodexCatalogRow>) => {
    writeSupplierCodexCatalogModels(supplierCodexCatalogModels.map((row) => row.rowId === rowId ? { ...row, ...patch } : row));
  };
  const removeSupplierCodexCatalogModel = (rowId: string) => {
    writeSupplierCodexCatalogModels(supplierCodexCatalogModels.filter((row) => row.rowId !== rowId));
  };

  const saveDraft = async (options: { stayInEditor?: boolean; applySupplier?: boolean } = {}): Promise<SupplierSaveResult | null> => {
    if (!appSettings || !draft || supplierSaveBusy) return null;
    const aggregateDraft = !!draft.aggregateEnabled;
    const requestedId = draft.id.trim();
    const normalizedId = supplierIdFromName(requestedId || draft.name);
    const idWasNormalized = requestedId !== normalizedId;
    const targetApp = supplierTargetForProfile(draft);
    const inheritedRouting = supplierRoutingEnabledForTarget(targetApp);
    const routedDraft = withSupplierRoutingState({ ...draft, id: normalizedId }, targetApp, inheritedRouting);
    const normalized = supplierProfileIsCcswitch(routedDraft)
      ? withSupplierPreservedImportedFiles(routedDraft)
      : normalizeSupplierProfile(withSupplierGeneratedFiles(routedDraft));
    if (!normalized.name.trim() || (!aggregateDraft && !normalized.baseUrl.trim())) {
      window.alert(aggregateDraft ? "请填写聚合供应商名称后再保存。" : "请填写供应商名称和 Base URL 后再保存。API Key 可以后续补入。");
      return null;
    }
    if (aggregateDraft && !(normalized.aggregateMembers ?? []).length) {
      actions.showNotice({ title: "添加聚合供应商", message: "请先添加或选择至少 1 个普通 API 供应商的 Base URL / Key，再勾选为成员。", status: "failed" });
      return null;
    }
    if (targetApp === "claude-desktop" && !normalized.modelMappingEnabled) {
      const invalidModel = supplierDirectModelRows(normalized.modelList)
        .find((row) => !supplierDirectModelIsClaudeDesktopSafe(row.model));
      if (invalidModel) {
        actions.showNotice({
          title: "供应商保存",
          message: `Claude Desktop 直连模型 ID 无效：${invalidModel.model}。请使用 claude-/anthropic/claude- 的 Sonnet、Opus、Haiku 或 Fable 模型，或开启模型映射。`,
          status: "failed",
        });
        return null;
      }
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
    const currentActiveId = activeSupplierIdForTarget(targetApp);
    const nextActiveRelayId = !aggregateDraft && originalId && currentActiveId === originalId
      ? normalized.id
      : currentActiveId;
    const nextSettings = withActiveSupplierId({
      ...appSettings,
      relayProfilesEnabled: true,
      relayProfiles: nextProfiles,
    }, targetApp, nextActiveRelayId);
    const shouldApplySupplier = !aggregateDraft && (
      options.applySupplier === true
      || (targetApp === "claude-desktop" && !!originalId && currentActiveId === originalId)
    );
    if (shouldApplySupplier && !supplierProfileHasApiKey(normalized)) {
      actions.showNotice({
        title: "供应商应用",
        message: "该供应商缺少 API Key，未修改当前生效配置。可取消“保存并使用”，先作为非活动配置保存。",
        status: "failed",
      });
      return null;
    }
    setSupplierSaveBusy(true);
    try {
      actions.showNotice({
        title: shouldApplySupplier ? "供应商保存并应用" : "供应商保存",
        message: shouldApplySupplier
          ? `正在保存并应用供应商「${normalized.name || normalized.id}」...`
          : `正在保存供应商「${normalized.name || normalized.id}」...`,
        status: "running",
      });
      const applied = shouldApplySupplier
        ? await actions.switchSupplierProfile(targetApp, normalized.id, nextSettings)
        : null;
      const saved = shouldApplySupplier
        ? applied && !statusFailed(applied.status) ? applied.settings : null
        : await saveSupplierSettings(nextSettings);
      if (saved) {
        const savedProfile = saved.relayProfiles.find((profile) => profile.id === normalized.id) ?? normalized;
        if (options.stayInEditor) {
          setEditingId(savedProfile.id);
          setDraft(normalizeDraftProfile(savedProfile));
          setSupplierCodexCatalogModels(createSupplierCodexCatalogModelRows(supplierCodexCatalogRows(savedProfile)));
        } else {
          closeSupplierEditor();
        }
        actions.showNotice({
          title: shouldApplySupplier ? "供应商保存并应用" : "供应商保存",
          message: shouldApplySupplier
            ? `已保存并应用供应商「${savedProfile.name || savedProfile.id}」。`
            : `已保存供应商「${savedProfile.name || savedProfile.id}」。`,
          status: "ok",
        });
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
    await saveDraft({ stayInEditor: true, applySupplier: true });
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
    const nextForTarget = (_targetApp: SupplierTargetApp, currentId: string) => currentId === profile.id ? "" : currentId;
    const saved = await saveSupplierSettings({
      ...appSettings,
      relayProfiles: nextProfiles,
      activeRelayId: nextForTarget("codex", appSettings.activeRelayId),
      activeClaudeRelayId: nextForTarget("claude", appSettings.activeClaudeRelayId || ""),
      activeClaudeDesktopRelayId: nextForTarget("claude-desktop", appSettings.activeClaudeDesktopRelayId || ""),
    });
    if (saved && editingId === profile.id) {
      setEditingId(null);
      setDraft(null);
    }
  };
  const applyPreset = (preset: SupplierPreset) => {
    if (!draft) return;
    setModelFetch(null);
    const targetApp = preset.targetApp ?? "codex";
    const modelList = preset.modelList?.join("\n") ?? preset.model;
    const codexCatalogJson = targetApp === "codex" ? "" : draft.codexCatalogJson ?? "";
    setSupplierDirectModels(createSupplierDirectModelRows(supplierDirectModelRows(modelList)));
    setSupplierCodexCatalogModels(targetApp === "codex"
      ? createSupplierCodexCatalogModelRows(supplierCodexCatalogRows({
        ...draft,
        targetApp,
        model: preset.model,
        testModel: preset.model,
        modelList,
        codexCatalogJson,
      }))
      : []);
    updateDraft({
      id: isNewDraft ? uniqueSupplierProfileId(profiles, preset.id) : draft.id,
      name: preset.name,
      baseUrl: preset.baseUrl,
      upstreamBaseUrl: preset.baseUrl,
      protocol: preset.protocol,
      targetApp,
      apiFormat: preset.apiFormat ?? "",
      routeEnabled: supplierRoutingEnabledForTarget(targetApp),
      claudeDesktopMode: targetApp === "codex" ? "" : supplierRoutingEnabledForTarget(targetApp) ? "proxy" : "direct",
      routeMode: targetApp === "codex"
        ? (supplierRoutingEnabledForTarget(targetApp) ? "Codex Proxy" : "Codex Direct")
        : (supplierRoutingEnabledForTarget(targetApp) ? "Claude Desktop Proxy" : "Claude Desktop Direct"),
      modelMappingEnabled: preset.modelMappingEnabled ?? false,
      modelMappingJson: preset.modelMappingJson ?? "",
      modelMapping: preset.modelMappingJson ? supplierModelMappingText(supplierModelMappingRows({ ...draft, modelMappingJson: preset.modelMappingJson })) : "",
      relayMode: "pureApi",
      aggregateEnabled: false,
      aggregateMembers: [],
      aggregateStrategy: "",
      model: preset.model,
      testModel: preset.model,
      modelList,
      codexCatalogJson,
    });
  };
  const fetchModels = async () => {
    if (!draft) return;
    const requestId = ++supplierModelFetchRequestRef.current;
    const normalized = normalizeSupplierProfile(withSupplierGeneratedFiles(draft));
    const result = await actions.fetchRelayProfileModels(normalized);
    if (requestId !== supplierModelFetchRequestRef.current) return;
    if (result) {
      setModelFetch(result);
      if (result.models.length) {
        if (draft.targetApp === "codex") return;
        const isClaudeTarget = draft.targetApp === "claude" || draft.targetApp === "claude-desktop";
        if (isClaudeTarget && draft.modelMappingEnabled) return;
        const existingRows = supplierDirectModels.length
          ? supplierDirectModels
          : createSupplierDirectModelRows(supplierDirectModelRows(draft.modelList));
        const existingModels = new Set(existingRows.map((row) => row.model.trim().toLowerCase()).filter(Boolean));
        const mergedRows = [
          ...existingRows,
          ...result.models
            .map((model) => model.trim())
            .filter((model) => model && !existingModels.has(model.toLowerCase()))
            .map((model) => ({
              model,
              rowId: `direct-model-${supplierDirectModelRowIdRef.current += 1}`,
              supports1m: false,
            })),
        ];
        setSupplierDirectModels(mergedRows);
        updateDraft({
          modelList: supplierDirectModelList(mergedRows),
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
  const toggleVisibleSupplierRouting = async (enabled: boolean) => {
    if (!appSettings || !routableSupplierProfiles.length) return;
    const visibleIds = new Set(routableSupplierProfiles.map((profile) => profile.id));
    const nextProfiles = appSettings.relayProfiles.map((profile) => {
      if (!visibleIds.has(profile.id)) return profile;
      if (supplierRouteGroup === "codex") {
        return withSupplierRoutingState(profile, "codex", enabled);
      }
      return withSupplierRoutingState(profile, supplierTargetForProfile(profile), enabled);
    });
    actions.showNotice({ title: "供应商路由", message: enabled ? `正在开启 ${supplierRouteGroupLabel} 供应商路由...` : `正在关闭 ${supplierRouteGroupLabel} 供应商路由...`, status: "running" });
    const saved = await saveSupplierSettings({ ...appSettings, relayProfiles: nextProfiles });
    if (saved) {
      actions.showNotice({ title: "供应商路由", message: enabled ? `已开启 ${supplierRouteGroupLabel} 供应商路由。` : `已关闭 ${supplierRouteGroupLabel} 供应商路由。`, status: "ok" });
    }
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
  const filteredOrderedProfiles = useMemo(() => orderedProfiles.filter((profile) => supplierTargetForProfile(profile) === supplierTargetFilter), [orderedProfiles, supplierTargetFilter]);
  const visibleSupplierOrderIds = useMemo(() => filteredOrderedProfiles.map((profile) => profile.id), [filteredOrderedProfiles]);
  const supplierRouteGroup = supplierTargetFilter === "codex" ? "codex" : "claude";
  const supplierRouteGroupLabel = supplierRouteGroup === "codex" ? "Codex" : "Claude";
  const routableSupplierProfiles = useMemo(() => profiles.filter((profile) => {
    const target = supplierTargetForProfile(profile);
    return supplierRouteGroup === "codex" ? target === "codex" : target === "claude" || target === "claude-desktop";
  }), [profiles, supplierRouteGroup]);
  const supplierRouteSwitchEnabled = routableSupplierProfiles.some((profile) => !!profile.routeEnabled);
  const supplierRouteSwitchDisabled = !appSettings || !routableSupplierProfiles.length;
  const setSupplierCardRef = (profileId: string) => (node: HTMLDivElement | null) => {
    if (node) {
      supplierCardRefs.current.set(profileId, node);
    } else {
      supplierCardRefs.current.delete(profileId);
    }
  };
  // 目标应用过滤后渲染卡片；保持全量 supplierOrderIds 用于跨过滤视图稳定排序。
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
  const supplierTargetIdFromPointer = (clientY: number) => {
    if (!visibleSupplierOrderIds.length) return null;
    for (const profileId of visibleSupplierOrderIds) {
      const node = supplierCardRefs.current.get(profileId);
      if (!node) continue;
      const rect = node.getBoundingClientRect();
      if (clientY < rect.top + rect.height / 2) return profileId;
    }
    return visibleSupplierOrderIds[visibleSupplierOrderIds.length - 1] ?? null;
  };
  const beginSupplierPointerDrag = (event: ReactPointerEvent<HTMLElement>, profileId: string) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    try {
      event.currentTarget.setPointerCapture(event.pointerId);
    } catch {
      // Pointer capture may be unavailable in some WebView states; window listeners still finish sorting.
    }
    const baselineIds = supplierOrderFromIds(supplierOrderIds.length ? supplierOrderIds : profiles.map((profile) => profile.id))
      .map((profile) => profile.id);
    const dragHandle = event.currentTarget;
    const sourceNode = supplierCardRefs.current.get(profileId);
    const sourceRect = sourceNode?.getBoundingClientRect();
    supplierPointerDragRef.current = {
      sourceId: profileId,
      latestIds: baselineIds,
      lastTargetId: profileId,
    };
    if (sourceRect) {
      setSupplierDragOverlay({
        profileId,
        top: sourceRect.top,
        left: sourceRect.left,
        width: sourceRect.width,
        height: sourceRect.height,
        offsetY: event.clientY - sourceRect.top,
      });
    }
    setDraggedId(profileId);
    setDragOverId(profileId);

    const handlePointerMove = (moveEvent: PointerEvent) => {
      const current = supplierPointerDragRef.current;
      if (!current) return;
      moveEvent.preventDefault();
      setSupplierDragOverlay((overlay) => overlay && overlay.profileId === current.sourceId
        ? { ...overlay, top: moveEvent.clientY - overlay.offsetY }
        : overlay);
      const targetId = supplierTargetIdFromPointer(moveEvent.clientY);
      if (!targetId || targetId === current.lastTargetId) return;
      const nextIds = reorderSupplierIds(current.sourceId, targetId, current.latestIds) ?? current.latestIds;
      current.latestIds = nextIds;
      current.lastTargetId = targetId;
      setDragOverId(targetId);
      setSupplierOrderIds(nextIds);
    };

    const finishPointerDrag = () => {
      const current = supplierPointerDragRef.current;
      supplierPointerDragRef.current = null;
      window.removeEventListener("pointermove", handlePointerMove, true);
      window.removeEventListener("pointerup", finishPointerDrag, true);
      window.removeEventListener("pointercancel", finishPointerDrag, true);
      try {
        dragHandle.releasePointerCapture(event.pointerId);
      } catch {
        // Pointer capture may already be released by the WebView.
      }
      setDraggedId(null);
      setDragOverId(null);
      setSupplierDragOverlay(null);
      if (current) {
        setSupplierOrderIds(current.latestIds);
        void saveSupplierOrder(current.latestIds);
      }
    };

    window.addEventListener("pointermove", handlePointerMove, true);
    window.addEventListener("pointerup", finishPointerDrag, { capture: true, once: true });
    window.addEventListener("pointercancel", finishPointerDrag, { capture: true, once: true });
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
    const imported = result.profiles.map((profile) => normalizeSupplierProfile(profile));
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
        ? normalizeSupplierProfile({ ...profile, id: uniqueSupplierProfileId(nextProfiles, profile.id) })
        : profile;
      existingIds.add(nextProfile.id);
      nextProfiles.push(nextProfile);
      addedCount += 1;
    }
    await saveSupplierSettings({ ...appSettings, relayProfiles: nextProfiles });
    actions.showNotice({ title: "CC-switch 导入", message: `已从 cc-switch 更新 ${updatedCount} 个、新增 ${addedCount} 个供应商配置。`, status: "ok" });
  };

  const refreshSupplierList = async () => {
    if (supplierRefreshBusy) return;
    setSupplierRefreshBusy(true);
    actions.showNotice({ title: "刷新供应商列表", message: "正在刷新供应商配置和路由状态...", status: "running" });
    try {
      await actions.refreshRoute("supplier", { notify: true });
      actions.showNotice({ title: "刷新供应商列表", message: "供应商列表已刷新。", status: "ok" });
      setImportOpen(false);
    } catch (error) {
      actions.showNotice({
        title: "刷新供应商列表失败",
        message: error instanceof Error ? error.message : String(error),
        status: "failed",
      });
    } finally {
      setSupplierRefreshBusy(false);
    }
  };

  const supplierDisplayUrl = (profile: RelayProfile) => {
    const configBaseUrl = profile.configContents.match(/\bbase_url\s*=\s*["']([^"']+)["']/i)?.[1]?.trim() ?? "";
    const rawUrl = profile.upstreamBaseUrl || profile.baseUrl || configBaseUrl;
    if (!rawUrl.trim()) return "未配置接口地址";
    return rawUrl.trim().replace(/\/v1\/?$/i, "");
  };

  const renderSupplierCard = (profile: RelayProfile, options: { overlay?: boolean; style?: CSSProperties } = {}) => {
    const targetApp = supplierTargetForProfile(profile);
    const selected = profile.id === activeSupplierIdForTarget(targetApp);
    const aggregate = !!profile.aggregateEnabled;
    const imported = supplierProfileIsCcswitch(profile);
    const appLabel = imported ? supplierTargetAppLabel(profile.targetApp) : supplierRelayModeLabel(profile.relayMode);
    const protocolLabel = imported ? supplierApiFormatLabel(profile) : supplierProtocolLabel(profile.protocol);
    const summary = aggregate
      ? `${aggregateStrategyLabel(profile.aggregateStrategy)} / ${profile.aggregateMembers?.length ?? 0} \u4e2a\u6210\u5458`
      : `${appLabel} / ${protocolLabel}`;
    const displayUrl = supplierDisplayUrl(profile);
    const dragSource = draggedId === profile.id && !options.overlay;
    return (
      <div
        className={`supplier-card ${selected ? "selected" : ""} ${draggedId === profile.id ? "dragging" : ""} ${dragOverId === profile.id ? "drag-over" : ""} ${dragSource ? "drag-source" : ""} ${options.overlay ? "drag-overlay-card" : ""}`}
        key={options.overlay ? `${profile.id}-overlay` : profile.id}
        ref={options.overlay ? undefined : setSupplierCardRef(profile.id)}
        style={options.style}
      >
        <button aria-label={"\u62d6\u62fd\u6392\u5e8f"} className="supplier-drag-handle" disabled={options.overlay} onPointerDown={options.overlay ? undefined : (event) => beginSupplierPointerDrag(event, profile.id)} title={"\u6309\u4f4f\u62d6\u62fd\u6392\u5e8f"} type="button">
          <GripVertical className="h-4 w-4" focusable="false" />
        </button>
        <div className="supplier-avatar">{aggregate ? "\u805a" : (profile.name || profile.id || "P").slice(0, 1).toUpperCase()}</div>
        <div className="supplier-card-main">
          <div className="supplier-title-line">
            <strong>{profile.name || profile.id}</strong>
            {aggregate ? <span className="supplier-badge">\u805a\u5408</span> : null}
            {imported ? <span className="supplier-badge">cc-switch</span> : null}
          </div>
          {aggregate ? <span className="supplier-card-subtitle">{summary}</span> : null}
          <button className="supplier-url-link" disabled type="button">{displayUrl}</button>
        </div>
        <div className="supplier-card-actions">
          <button className={`supplier-card-action-button supplier-card-use-button ${selected ? "current" : ""}`} disabled={selected || aggregate || appSettings?.relayProfilesEnabled === false || options.overlay} onClick={() => void actions.switchSupplierProfile(targetApp, profile.id)} type="button">
            <Play className="h-4 w-4" />
            {selected ? "\u4f7f\u7528\u4e2d" : "\u4f7f\u7528"}
          </button>
          <button className="supplier-card-action-button" disabled={options.overlay} onClick={() => openProfileEditor(profile)} title="\u7f16\u8f91" type="button"><Edit className="h-4 w-4" /></button>
          <button className="supplier-card-action-button" disabled={options.overlay} onClick={() => duplicateProfile(profile)} title="\u590d\u5236" type="button"><Copy className="h-4 w-4" /></button>
          <button className="supplier-card-action-button" disabled title="\u68c0\u6d4b\u8fde\u901a" type="button"><Activity className="h-4 w-4" /></button>
          <button className="supplier-card-action-button" disabled title="\u7528\u91cf\u914d\u7f6e" type="button"><BarChart3 className="h-4 w-4" /></button>
          <button className="supplier-card-action-button" disabled={profiles.length <= 1 || options.overlay} onClick={() => void removeProfile(profile)} title="\u5220\u9664\u4f9b\u5e94\u5546" type="button"><Trash2 className="h-4 w-4" /></button>
        </div>
      </div>
    );
  };
  const supplierDragOverlayProfile = supplierDragOverlay ? profiles.find((profile) => profile.id === supplierDragOverlay.profileId) : null;


  if (draft?.aggregateEnabled) {
    const generated = normalizeSupplierProfile(withSupplierGeneratedFiles(draft));
    const members = generated.aggregateMembers ?? [];
    return (
      <div className="supplier-workbench">
        <Panel title={generated.name || "聚合供应商1"} detail="聚合供应商会保存策略和成员关系；当前版本不直接写入 Codex，后续聚合代理会读取这些字段。">
          <div className="supplier-editor-toolbar sticky">
            <Button onClick={closeSupplierEditor} variant="outline">返回列表</Button>
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
    const generated = normalizeDraftProfile(draft);
    const canSwitch = !!editingExisting && appSettings?.relayProfilesEnabled !== false;
    const isClaudeSupplier = generated.targetApp === "claude" || generated.targetApp === "claude-desktop";
    const isCodexSupplier = generated.targetApp === "codex" || !generated.targetApp;
    const apiFormatOption = supplierApiFormatOption(generated.apiFormat || "Anthropic Messages");
    const selectedApiFormat = isCodexSupplier
      ? (generated.apiFormat === "openai_chat" || generated.protocol === "chatCompletions" ? "openai_chat" : "openai_responses")
      : generated.apiFormat;
    const routeRequired = isCodexSupplier
      ? selectedApiFormat === "openai_chat"
      : supplierApiFormatRequiresRoute(selectedApiFormat);
    const routeEnabled = !!generated.routeEnabled;
    const authField = generated.authField || "ANTHROPIC_AUTH_TOKEN";
    const defaultModel = generated.model || generated.testModel || (isCodexSupplier ? "gpt-5.1" : "claude-sonnet");
    const modelRowsForDraft = supplierModelMappingRows(generated);
    const supplierModelOptions = Array.from(new Set((modelFetch !== null
      ? modelFetch.models
      : isCodexSupplier
        ? supplierCodexCatalogModels.map((row) => row.model)
        : supplierDirectModelRows(generated.modelList).map((row) => row.model))
      .map((model) => String(model || "").trim())
      .filter(Boolean)));
    const routePrompt = routeRequired && !routeEnabled
      ? `当前 API 格式需要路由。请返回供应商列表开启${isCodexSupplier ? " Codex" : " Claude / Claude Desktop"} 路由。`
      : "";
    const applyOneClickModelMapping = () => {
      if (!supplierModelOptions.length) {
        actions.showNotice({ title: "一键设置失败", message: "请先获取模型，或在保存的模型列表中配置可用模型。", status: "failed" });
        return;
      }
      const lowerOptions = supplierModelOptions.map((option) => ({ option, lower: option.toLowerCase() }));
      const rows = modelRowsForDraft.map((row) => {
        const current = row.requestModel.trim();
        const selected = supplierModelOptions.includes(current)
          ? current
          : lowerOptions.find(({ lower }) => lower.includes(row.role))?.option
            || (supplierModelOptions.includes(defaultModel) ? defaultModel : supplierModelOptions[0]);
        return { ...row, displayName: row.displayName || selected, requestModel: selected };
      });
      updateDraft({ modelMappingEnabled: true, modelMappingJson: supplierModelMappingJson(rows), modelMapping: supplierModelMappingText(rows) });
      actions.showNotice({ title: "一键设置完成", message: `已为 ${rows.length} 个 Claude 角色设置有效的实际请求模型。`, status: "ok" });
    };
    const cleanName = generated.name.replace(/\s*\(ccswitch\)$/i, "");
    const editorTitle = isNewDraft ? "添加供应商" : "编辑供应商";
    const editorAppLabel = supplierTargetAppLabel(generated.targetApp || "codex");
    const formAvatar = (cleanName || generated.id || "P").slice(0, 1).toUpperCase();
    const baseEndpointLabel = isCodexSupplier ? "API 请求地址" : "请求地址";
    const baseEndpointHint = isCodexSupplier
      ? "填写兼容 OpenAI Responses 或 Chat Completions 格式的服务端点地址；Chat Completions 按 cc-switch 语义启用路由接管。"
      : "填写兼容 Claude API 的服务端点地址，不要以斜杠结尾。";
    const claudeConfigJson = JSON.stringify({
      env: {
        [authField]: generated.apiKey,
        ANTHROPIC_BASE_URL: generated.baseUrl || generated.upstreamBaseUrl,
        ...(generated.modelMappingEnabled ? {
          ANTHROPIC_DEFAULT_HAIKU_MODEL: modelRowsForDraft.find((row) => row.role === "haiku")?.requestModel || defaultModel,
          ANTHROPIC_DEFAULT_OPUS_MODEL: modelRowsForDraft.find((row) => row.role === "opus")?.requestModel || defaultModel,
          ANTHROPIC_DEFAULT_FABLE_MODEL: modelRowsForDraft.find((row) => row.role === "fable")?.requestModel || defaultModel,
          ANTHROPIC_DEFAULT_SONNET_MODEL: modelRowsForDraft.find((row) => row.role === "sonnet")?.requestModel || defaultModel,
          CLAUDE_CODE_SUBAGENT_MODEL: modelRowsForDraft.find((row) => row.role === "subagent")?.requestModel || "",
        } : {}),
        ANTHROPIC_MODEL: defaultModel,
      },
      ...(generated.headerOverride?.trim() || generated.bodyOverride?.trim()
        ? { localProxyOverrides: { headers: generated.headerOverride || "{}", body: generated.bodyOverride || "{}" } }
        : {}),
    }, null, 2);
    const supplierConfigJson = generated.configContents || claudeConfigJson;
    const visibleSupplierConfigJson = showSupplierApiKey
      ? supplierConfigJson
      : redactSupplierConfig(supplierConfigJson);
    const codexAuthJson = generated.authContents || JSON.stringify({ OPENAI_API_KEY: generated.apiKey }, null, 2);
    const codexConfigToml = generated.configContents || `model = "${generated.model || defaultModel}"
model_provider = "${generated.id || "custom"}"

[model_providers.${generated.id || "custom"}]
name = "${cleanName || "Custom Provider"}"
base_url = "${generated.baseUrl || generated.upstreamBaseUrl || "https://api.example.com/v1"}"
wire_api = "${generated.apiFormat === "openai_chat" || generated.protocol === "chatCompletions" ? "chat" : "responses"}"
env_key = "OPENAI_API_KEY"
`;
    const visibleCodexAuthJson = redactSupplierConfig(codexAuthJson);
    const visibleCodexConfigToml = redactSupplierConfig(codexConfigToml);
    const visibleHeaderOverride = showSupplierApiKey
      ? generated.headerOverride || ""
      : redactSupplierConfig(generated.headerOverride || "");
    const visibleBodyOverride = showSupplierApiKey
      ? generated.bodyOverride || ""
      : redactSupplierConfig(generated.bodyOverride || "");
    const renderSourceCollapse = (open: boolean, setOpen: Dispatch<SetStateAction<boolean>>, icon: ReactNode, title: string, children: ReactNode) => (
      <div className={`supplier-ccswitch-collapse-card ${open ? "expanded" : ""}`}>
        <div className="supplier-ccswitch-collapse-head" onClick={() => setOpen((value) => !value)} onKeyDown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); setOpen((value) => !value); } }} role="button" tabIndex={0}>
          <span className="supplier-collapse-title">{icon}{title}</span>
          <span className="supplier-collapse-right"><span>使用单独配置</span><ToggleSwitch checked={false} disabled onChange={() => undefined} /><span className="supplier-collapse-chevron">{open ? "v" : ">"}</span></span>
        </div>
        {open ? <div className="supplier-ccswitch-collapse-body">{children}</div> : null}
      </div>
    );
    return (
      <div className="supplier-ccswitch-editor source-parity">
        <div className="supplier-ccswitch-editor-head"><button className="supplier-back-button" onClick={closeSupplierEditor} type="button" aria-label="返回供应商列表" title="返回"><ArrowLeft className="h-5 w-5" /></button><strong>{editorTitle}</strong></div>
        <div className="supplier-ccswitch-editor-body"><section className="supplier-ccswitch-form-card">
          <div className="supplier-form-avatar-shell"><div className="supplier-form-avatar">{formAvatar}</div></div>
          <div className="supplier-preset-strip">{SUPPLIER_PRESETS.filter((preset) => preset.id === "openai" || preset.id === "anthropic").map((preset) => <button className={preset.id === generated.id ? "active" : ""} key={preset.id} onClick={() => applyPreset(preset)} type="button"><strong>{preset.name}</strong><span>{preset.targetApp === "claude-desktop" ? "Claude Desktop" : preset.targetApp === "claude" ? "Claude" : "Codex"}</span></button>)}</div>
          <label className="ops-form-field"><span>供应商名称</span><input onBlur={(event) => updateNewDraftIdFromName(event.currentTarget.value)} onChange={(event) => updateDraft({ name: event.currentTarget.value })} value={cleanName} /></label>
          <label className="ops-form-field"><span>备注</span><input onChange={(event) => updateDraft({ notes: event.currentTarget.value })} placeholder="例如：公司专用账号" value={generated.notes || ""} /></label>
          <label className="ops-form-field"><span>官网链接</span><input onChange={(event) => updateDraft({ websiteUrl: event.currentTarget.value })} placeholder="https://example.com" value={generated.websiteUrl || ""} /></label>
          <label className="ops-form-field"><span>API Key</span><div className="supplier-secret-input"><input onChange={(event) => updateDraft({ apiKey: event.currentTarget.value })} type={showSupplierApiKey ? "text" : "password"} value={generated.apiKey} /><button aria-label={showSupplierApiKey ? "隐藏密钥" : "显示密钥"} onClick={() => setShowSupplierApiKey((value) => !value)} title={showSupplierApiKey ? "隐藏密钥" : "显示密钥"} type="button">{showSupplierApiKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}</button></div></label>
          <label className="ops-form-field"><span>{baseEndpointLabel} <span className="supplier-url-toggle">完整 URL</span></span><input onChange={(event) => updateDraft({ baseUrl: event.currentTarget.value, upstreamBaseUrl: event.currentTarget.value })} placeholder={isCodexSupplier ? "https://api.example.com/v1" : "https://api.example.com"} value={generated.baseUrl || generated.upstreamBaseUrl} /></label>
          <div className="supplier-route-note">提示：{baseEndpointHint}</div>
          {isClaudeSupplier ? <section className="supplier-mapping-card"><div><strong>需要模型映射</strong><p>关闭时按原始模型 ID 直传；供应商不接受 Claude 安全路由 ID 时请开启映射。</p></div><ToggleSwitch checked={!!generated.modelMappingEnabled} onChange={(value) => updateDraft({ modelMappingEnabled: value })} /></section> : null}
          <details className="supplier-ccswitch-section supplier-advanced-card" open><summary><span>&gt;</span>高级选项</summary>
            {isClaudeSupplier ? (
              generated.modelMappingEnabled ? (
                <>
                  <label className="ops-form-field">
                    <span>API 格式</span>
                    <select className="ops-select" onChange={(event) => updateDraft({ apiFormat: event.currentTarget.value })} value={generated.apiFormat || "Anthropic Messages"}>
                      {SUPPLIER_API_FORMAT_OPTIONS.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
                    </select>
                    <small>{apiFormatOption?.detail || "选择供应商 API 的输入格式"}</small>
                  </label>
                  {routePrompt ? <div className="supplier-route-note">{routePrompt}</div> : null}
                  <div className="supplier-ccswitch-divider" />
                  <div className="supplier-model-map-head">
                    <strong>模型映射</strong>
                    <div className="supplier-toolbar">
                      <Button onClick={applyOneClickModelMapping} type="button" variant="outline"><Wrench className="h-4 w-4" />一键设置</Button>
                      <Button onClick={() => void fetchModels()} type="button" variant="outline"><Download className="h-4 w-4" />获取模型</Button>
                    </div>
                  </div>
                  <p className="supplier-inline-note">显示名称只影响模型菜单；实际请求模型会发送到上游；1M 是本地能力声明。</p>
                  <div className="supplier-model-map-grid header claude"><span>模型角色</span><span>显示名称</span><span>实际请求模型</span><span>声明支持 1M</span></div>
                  {modelRowsForDraft.map((row) => (
                    <div className="supplier-model-map-grid claude" key={row.role}>
                      <input disabled value={row.label} />
                      <input onChange={(event) => updateSupplierModelMapping(row.role, "displayName", event.currentTarget.value)} placeholder={defaultModel} value={row.displayName || ""} />
                      <SupplierModelDropdown
                        onChange={(value) => updateSupplierModelMapping(row.role, "requestModel", value)}
                        options={supplierModelOptions}
                        placeholder="选择实际请求模型"
                        value={row.requestModel || ""}
                      />
                      <label><input checked={row.supports1m} onChange={(event) => updateSupplierModelMapping(row.role, "supports1m", event.currentTarget.checked)} type="checkbox" />1M</label>
                    </div>
                  ))}
                  <label className="ops-form-field"><span>默认兜底模型</span><input onChange={(event) => updateDraft({ model: event.currentTarget.value, testModel: event.currentTarget.value })} value={defaultModel} /></label>
                </>
              ) : (
                <details className="supplier-direct-model-list" onToggle={(event) => setSupplierDirectModelsOpen(event.currentTarget.open)} open={supplierDirectModelsOpen}>
                  <summary><span>{supplierDirectModelsOpen ? "⌄" : ">"}</span>手动指定 Claude Desktop 模型列表（高级，可选）</summary>
                  <div className="supplier-direct-model-list-body">
                    <div className="supplier-direct-model-list-head">
                      <p>仅当供应商的 /v1/models 不可用或没有返回 Claude Desktop 可识别的 Sonnet / Opus / Haiku 模型名时填写；勾选 1M 会向 Claude Desktop 声明支持 1M 上下文。</p>
                      <div className="supplier-toolbar">
                        <Button onClick={() => void fetchModels()} type="button" variant="outline"><Download className="h-4 w-4" />获取模型列表</Button>
                        <Button onClick={addSupplierDirectModel} type="button" variant="outline"><Plus className="h-4 w-4" />添加模型</Button>
                      </div>
                    </div>
                    {supplierDirectModels.length ? <div className="supplier-direct-model-rows">
                      {supplierDirectModels.map((row, index) => (
                        <div className="supplier-direct-model-row" key={row.rowId}>
                          <input aria-label={`Claude Desktop 模型 ${index + 1}`} onChange={(event) => updateSupplierDirectModel(row.rowId, { model: event.currentTarget.value })} placeholder="claude-sonnet-4-6" value={row.model} />
                          <label><input checked={row.supports1m} onChange={(event) => updateSupplierDirectModel(row.rowId, { supports1m: event.currentTarget.checked })} type="checkbox" />1M</label>
                          <button aria-label="删除模型" className="supplier-direct-model-remove" onClick={() => removeSupplierDirectModel(row.rowId)} title="删除模型" type="button"><Trash2 className="h-4 w-4" /></button>
                        </div>
                      ))}
                    </div> : <p className="supplier-direct-model-empty">尚未指定手动模型；Claude Desktop 会优先读取供应商模型目录。</p>}
                  </div>
                </details>
              )
            ) : (
              <>
                <label className="ops-form-field">
                  <span>上游格式</span>
                  <select className="ops-select" onChange={(event) => {
                    const next = event.currentTarget.value;
                    updateDraft({ apiFormat: next, protocol: next === "openai_chat" ? "chatCompletions" : "responses" });
                  }} value={generated.apiFormat === "openai_chat" || generated.protocol === "chatCompletions" ? "openai_chat" : "openai_responses"}>
                    <option value="openai_chat">Chat Completions（需开启路由）</option>
                    <option value="openai_responses">Responses（原生）</option>
                  </select>
                  <small>Responses 可直连；Chat Completions 需要路由接管。</small>
                </label>
                {routePrompt ? <div className="supplier-route-note">{routePrompt}</div> : null}
                <div className="supplier-ccswitch-divider" />
                <div className="supplier-model-map-head">
                  <strong>模型映射</strong>
                  <div className="supplier-toolbar">
                    <Button onClick={() => void fetchModels()} type="button" variant="outline"><Download className="h-4 w-4" />获取模型</Button>
                    <Button onClick={addSupplierCodexCatalogModel} type="button" variant="outline"><Plus className="h-4 w-4" />添加模型</Button>
                  </div>
                </div>
                <p className="supplier-inline-note">菜单显示名用于 Codex 模型选择；实际请求模型发送给上游；上下文窗口用于本地模型能力说明。</p>
                <div className="supplier-codex-catalog-grid header"><span>菜单显示名</span><span>实际请求模型</span><span>上下文窗口</span><span /></div>
                {supplierCodexCatalogModels.length ? supplierCodexCatalogModels.map((row, index) => (
                  <div className="supplier-codex-catalog-grid" key={row.rowId}>
                    <input
                      aria-label={`菜单显示名 ${index + 1}`}
                      onChange={(event) => updateSupplierCodexCatalogModel(row.rowId, { displayName: event.currentTarget.value })}
                      placeholder="例如: DeepSeek V4 Flash"
                      value={row.displayName}
                    />
                    <div className="supplier-model-input-dropdown">
                      <input
                        aria-label={`实际请求模型 ${index + 1}`}
                        onChange={(event) => updateSupplierCodexCatalogModel(row.rowId, { model: event.currentTarget.value })}
                        placeholder="例如: deepseek-v4-flash"
                        value={row.model}
                      />
                      <SupplierModelDropdown
                        compact
                        iconOnly
                        onChange={(value) => updateSupplierCodexCatalogModel(row.rowId, {
                          model: value,
                          ...(row.displayName.trim() ? {} : { displayName: value }),
                        })}
                        options={supplierModelOptions}
                        placeholder="选择已获取模型"
                        showAvailabilityWarning={false}
                        triggerLabel="选择已获取模型"
                        value={row.model}
                      />
                    </div>
                    <input
                      aria-label={`上下文窗口 ${index + 1}`}
                      inputMode="numeric"
                      onChange={(event) => updateSupplierCodexCatalogModel(row.rowId, { contextWindow: event.currentTarget.value.replace(/[^\d]/g, "") })}
                      placeholder="例如: 128000"
                      value={row.contextWindow}
                    />
                    <button aria-label="删除模型" className="supplier-codex-catalog-remove" onClick={() => removeSupplierCodexCatalogModel(row.rowId)} title="删除模型" type="button"><Trash2 className="h-4 w-4" /></button>
                  </div>
                )) : <p className="supplier-codex-catalog-empty">尚未添加模型；可手动添加，或先获取上游模型后从列表选择。</p>}
                <label className="ops-form-field">
                  <span>自定义 User-Agent</span>
                  <div className="supplier-user-agent-control">
                    <input onChange={(event) => updateDraft({ userAgent: event.currentTarget.value })} placeholder="Mozilla/5.0 ..." value={generated.userAgent || ""} />
                    <SupplierModelDropdown
                      compact
                      onChange={(value) => updateDraft({ userAgent: value })}
                      options={[...SUPPLIER_USER_AGENT_PRESETS]}
                      placeholder="选择 User-Agent 预设"
                      showAvailabilityWarning={false}
                      triggerLabel="预设"
                      value={generated.userAgent || ""}
                    />
                  </div>
                  <small>仅在本地路由或代理接管后生效，用于替换发送到供应商 API 的 User-Agent。</small>
                </label>
              </>
            )}
            <div className="supplier-ccswitch-divider" /><strong>本地代理请求覆盖</strong><p className="supplier-inline-note">仅在本地路由 / 代理接管后生效，应用于协议转换后的上游请求。</p><div className="supplier-ccswitch-form-grid two"><label className="ops-form-field"><span>Header 覆盖</span><textarea className="ops-textarea mono" onChange={(event) => updateDraft({ headerOverride: event.currentTarget.value })} readOnly={!showSupplierApiKey} rows={6} value={visibleHeaderOverride} placeholder={'{\n  "X-Provider": "cc-switch"\n}'} /></label><label className="ops-form-field"><span>Body 覆盖</span><textarea className="ops-textarea mono" onChange={(event) => updateDraft({ bodyOverride: event.currentTarget.value })} readOnly={!showSupplierApiKey} rows={6} value={visibleBodyOverride} placeholder={'{\n  "temperature": 0.2\n}'} /></label></div>{isClaudeSupplier ? <label className="ops-form-field"><span>配置 JSON</span><textarea className="ops-textarea mono supplier-config-json" onChange={(event) => updateDraft({ configContents: event.currentTarget.value })} readOnly={!showSupplierApiKey} value={visibleSupplierConfigJson} /></label> : <><label className="ops-form-field"><span>auth.json</span><textarea className="ops-textarea mono supplier-config-json compact" readOnly value={visibleCodexAuthJson} /></label><label className="ops-form-field"><span>config.toml</span><textarea className="ops-textarea mono supplier-config-json" readOnly value={visibleCodexConfigToml} /></label></>}{renderSourceCollapse(supplierTestConfigOpen, setSupplierTestConfigOpen, <Activity className="h-4 w-4" />, "模型 Test Config", <><p>为此供应商配置单独的模型测试参数。</p><div className="supplier-ccswitch-form-grid two"><label className="ops-form-field"><span>超时时间（秒）</span><input disabled placeholder="8" /></label><label className="ops-form-field"><span>降级阈值（毫秒）</span><input disabled placeholder="6000" /></label><label className="ops-form-field"><span>最大重试次数</span><input disabled placeholder="1" /></label></div></>)}{renderSourceCollapse(supplierPricingConfigOpen, setSupplierPricingConfigOpen, <BarChart3 className="h-4 w-4" />, "计费配置", <><p>为此供应商配置单独的计费参数。</p><div className="supplier-ccswitch-form-grid two"><label className="ops-form-field"><span>成本倍率</span><input disabled placeholder="留空使用全局默认" /></label><label className="ops-form-field"><span>计费模式</span><select className="ops-select" disabled value="inherit"><option value="inherit">继承全局默认</option><option value="request">请求模型</option><option value="response">返回模型</option></select></label></div></>)}
          </details></section></div>
        <div className="supplier-ccswitch-savebar"><span>{modelFetch?.models.length ? `已获取 ${modelFetch.models.length} 个模型，来源：${modelFetch.endpoint || "模型接口"}` : "请检查并保存供应商配置"}</span><div className="action-row"><Button onClick={closeSupplierEditor} type="button" variant="outline">取消</Button><Button disabled={supplierSaveBusy} onClick={() => void saveDraft()} type="button"><Save className="h-4 w-4" />{supplierSaveBusy ? "保存中" : "保存"}</Button><Button disabled={!canSwitch || supplierSaveBusy} onClick={() => void saveAndSwitchDraft()} type="button"><KeyRound className="h-4 w-4" />保存并使用</Button></div></div>
      </div>
    );
  }


  return (
    <div className="supplier-list-shell">
      {credentialEnvironment?.present ? <div className="supplier-env-card"><ShieldCheck className="h-5 w-5" /><div><strong>{credentialEnvironment.conflict ? "检测到凭据环境变量冲突" : "检测到凭据环境变量"}</strong><p>{credentialEnvironment.conflict ? `${credentialEnvironment.variableName} 与当前 Codex 供应商凭据不一致，可能覆盖 config.toml / auth.json 并导致 401；不会清理 CODEX_HOME。` : `${credentialEnvironment.variableName} 已存在，当前未发现与活动供应商的值冲突。`}{credentialEnvironment.restartRequired ? " 请完全退出并重新启动 Codex。" : ""}</p><span className="supplier-env-chip">{credentialEnvironment.variableName} {credentialEnvironment.userPresent ? "用户环境" : credentialEnvironment.systemPresent ? "系统环境" : "当前进程"}</span></div><div className="supplier-env-actions"><Button disabled={!credentialEnvironment.canClearUser || credentialEnvironmentBusy} onClick={() => void clearCredentialEnvironment()} size="sm" variant="outline"><Trash2 className="h-4 w-4" />删除</Button><Button disabled={credentialEnvironmentBusy} onClick={() => void refreshCredentialEnvironment()} size="sm" variant="outline"><RefreshCw className={`h-4 w-4 ${credentialEnvironmentBusy ? "spin" : ""}`} />{credentialEnvironmentBusy ? "检测中" : "检测"}</Button></div></div> : null}
      <div className="supplier-master-row"><label><input checked={appSettings?.relayProfilesEnabled !== false} disabled={!appSettings} onChange={(event) => void toggleMasterSwitch(event.currentTarget.checked)} type="checkbox" />启用供应商配置切换</label><p>关闭后本工具不会在手动切换时写入 Codex 的 config.toml / auth.json；启动 Codex 时始终不会自动改这些文件。</p></div>
      <div className="supplier-control-row"><div className="supplier-route-master-toggle"><Network className="h-4 w-4" /><span>开启路由</span><ToggleSwitch checked={supplierRouteSwitchEnabled} disabled={supplierRouteSwitchDisabled} onChange={(value) => void toggleVisibleSupplierRouting(value)} /></div><div className="supplier-toolbar right"><div className="supplier-target-filter" aria-label="供应商目标应用过滤"><button className={supplierTargetFilter === "codex" ? "active" : ""} onClick={() => setSupplierTargetFilter("codex")} type="button">Codex</button><button className={supplierTargetFilter === "claude" ? "active" : ""} onClick={() => setSupplierTargetFilter("claude")} type="button">Claude</button><button className={supplierTargetFilter === "claude-desktop" ? "active" : ""} onClick={() => setSupplierTargetFilter("claude-desktop")} type="button">Claude Desktop</button></div><Button disabled={!appSettings} onClick={createProfile}><Plus className="h-4 w-4" />添加供应商</Button><Button disabled={!appSettings} onClick={createAggregateProfile} variant="outline"><Plus className="h-4 w-4" />添加聚合供应商</Button><div className="supplier-import-wrap"><Button onClick={() => setImportOpen((value) => !value)} variant="outline"><Download className="h-4 w-4" />从第三方导入</Button>{importOpen ? <div className="supplier-drop-popover"><button onClick={() => void importFromCcswitch()} type="button"><strong>ccswitch</strong><span>发现并导入 Codex / Claude / Claude Desktop 配置</span></button><button className={`supplier-menu-action ${supplierRefreshBusy ? "busy" : ""}`} disabled={supplierRefreshBusy} onClick={() => void refreshSupplierList()} type="button"><RefreshCw className={`h-4 w-4 ${supplierRefreshBusy ? "spin" : ""}`} />{supplierRefreshBusy ? "刷新中..." : "刷新列表"}</button></div> : null}</div></div></div>
      <div className="supplier-card-list">
        {filteredOrderedProfiles.length ? filteredOrderedProfiles.map((profile) => renderSupplierCard(profile)) : <Empty text="\u6682\u65e0\u4f9b\u5e94\u5546\u914d\u7f6e\uff0c\u70b9\u51fb\u201c\u6dfb\u52a0\u4f9b\u5e94\u5546\u201d\u521b\u5efa\u4e00\u4e2a\u771f\u5b9e\u53ef\u5207\u6362\u7684 Codex API \u914d\u7f6e\u3002" />}
      </div>
      {supplierDragOverlay && supplierDragOverlayProfile ? renderSupplierCard(supplierDragOverlayProfile, {
        overlay: true,
        style: {
          left: supplierDragOverlay.left,
          minHeight: supplierDragOverlay.height,
          top: supplierDragOverlay.top,
          width: supplierDragOverlay.width,
        },
      }) : null}
    </div>
  );
}
export function LegacySupplierScreen({
  actions,
  settings,
  claudeDesktopDevMode,
  claudeDesktopProviderPreview,
  claudeDesktopProviderApply,
  claudeDesktopProviderDraft,
  onClaudeDesktopProviderDraftChange,
}: {
  actions: AppActions;
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
            <InfoRow label="接口地址" value={active?.baseUrl || settings?.settings.relayBaseUrl || "官方登录"} />
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
            <InfoRow label="Profile 元数据" value={compactPath(claudeDesktopDevMode?.devModeStatus.profileMetaPath)} />
          </div>
        </Panel>
        <Panel title="当前配置摘录" detail="只展示路径和非敏感字段。">
          <div className="info-grid compact">
            <InfoRow label="供应商同步" value={settings?.settings.providerSyncEnabled ? "开启" : "关闭"} />
            <InfoRow label="供应商开关" value={settings?.settings.relayProfilesEnabled ? "开启" : "关闭"} />
            <InfoRow label="协议" value={active?.protocol || "responses"} />
            <InfoRow label="测试模型" value={active?.testModel || settings?.settings.relayTestModel || "默认"} />
          </div>
        </Panel>
      </div>
    </div>
  );
}

export const ToolsAndPluginsScreen = memo(function ToolsAndPluginsScreen({
  actions,
  claudeDesktopMarketplace,
  codexPluginMarketplace,
  settings,
  unifiedInventory,
}: {
  actions: AppActions;
  claudeDesktopMarketplace: ClaudeDesktopMarketplaceStatusResult | null;
  codexPluginMarketplace: CodexPluginMarketplaceStatusResult | null;
  settings: SettingsResult | null;
  unifiedInventory: UnifiedToolInventoryResult | null;
}) {
  return (
    <div className="stack">
      <div className="repository-status-grid">
        <CodexPluginRepositoryPanel actions={actions} marketplace={codexPluginMarketplace} />
        <ClaudePluginRepositoryPanel actions={actions} marketplace={claudeDesktopMarketplace} />
      </div>
      <UnifiedToolInventoryPanel
        actions={actions}
        result={unifiedInventory}
        settings={settings?.settings ?? null}
      />
    </div>
  );
});

function UnifiedToolInventoryPanel({
  actions,
  result,
  settings,
}: {
  actions: AppActions;
  result: UnifiedToolInventoryResult | null;
  settings: BackendSettings | null;
}) {
  const [tab, setTab] = useState<ContextKind>("mcp");
  const [pending, setPending] = useState<string | null>(null);
  const [creatingMcp, setCreatingMcp] = useState(false);
  const [mcpTarget, setMcpTarget] = useState<"claude" | "codex">("codex");
  const [mcpId, setMcpId] = useState("");
  const [mcpBody, setMcpBody] = useState(defaultContextToml("mcp"));
  const inventory = result?.inventory;
  const entries = useMemo(
    () => (inventory?.assets ?? []).filter((asset) => asset.kind === tab),
    [inventory, tab],
  );
  const countFor = (kind: ContextKind) => {
    if (kind === "skill") return inventory?.counts.skills ?? 0;
    if (kind === "plugin") return inventory?.counts.plugins ?? 0;
    return inventory?.counts.mcp ?? 0;
  };
  const toggle = async (asset: UnifiedToolAsset, app: "claude" | "codex") => {
    const state = asset[app];
    const key = `${asset.kind}:${asset.id}:${app}`;
    setPending(key);
    try {
      await actions.toggleUnifiedToolAsset(asset.id, asset.kind, app, !state.enabled);
    } finally {
      setPending(null);
    }
  };
  const resetMcpDraft = () => {
    setCreatingMcp(false);
    setMcpTarget("codex");
    setMcpId("");
    setMcpBody(defaultContextToml("mcp"));
  };
  const beginCreateMcp = () => {
    setTab("mcp");
    setCreatingMcp(true);
    setMcpTarget("codex");
    setMcpId("");
    setMcpBody(defaultContextToml("mcp"));
  };
  const saveMcp = async () => {
    const id = mcpId.trim();
    if (!id || !mcpBody.trim() || pending !== null) return;
    setPending("create:mcp");
    try {
      const saved = mcpTarget === "codex"
        ? settings
          ? await actions.saveContextEntry("mcp", id, mcpBody, settings)
          : null
        : await actions.saveClaudeContextEntry("mcp", id, mcpBody);
      if (saved) {
        await actions.refreshUnifiedToolInventory(true);
        if (statusOk(saved.status)) resetMcpDraft();
      }
    } finally {
      setPending(null);
    }
  };

  return (
    <section className="context-manager-card unified-tool-inventory">
      <header className="context-manager-head">
        <div>
          <h2>Claude、Codex 工具与插件</h2>
          <p>完整检测两端本地资产；同一资产只显示一行，点亮应用图标即启用到对应应用。</p>
        </div>
        <div className="action-row">
          <Button
            aria-controls="unified-mcp-editor"
            aria-expanded={creatingMcp}
            disabled={pending !== null}
            onClick={creatingMcp ? resetMcpDraft : beginCreateMcp}
            size="sm"
          >
            <Plus className="h-4 w-4" />
            {creatingMcp ? "收起新增 MCP" : "新增 MCP"}
          </Button>
          <Button disabled={pending !== null} onClick={async () => {
            setPending("scan");
            try {
              await actions.refreshUnifiedToolInventory(false);
            } finally {
              setPending(null);
            }
          }} size="sm" variant="outline">
            <RefreshCw className={`h-4 w-4${pending === "scan" ? " spin" : ""}`} />
            {pending === "scan" ? "检测中" : "重新检测"}
          </Button>
        </div>
      </header>
      {creatingMcp ? (
        <div aria-label="新增 MCP" className="context-editor" id="unified-mcp-editor" role="region">
          <div className="context-editor-grid">
            <label className="ops-form-field">
              <span>目标应用</span>
              <select
                className="ops-select"
                disabled={pending !== null}
                onChange={(event) => {
                  const target = event.currentTarget.value as "claude" | "codex";
                  setMcpTarget(target);
                  setMcpBody(target === "codex" ? defaultContextToml("mcp") : defaultClaudeContextBody("mcp"));
                }}
                value={mcpTarget}
              >
                <option value="codex">Codex</option>
                <option value="claude">Claude</option>
              </select>
            </label>
            <label className="ops-form-field">
              <span>MCP ID</span>
              <input
                autoComplete="off"
                disabled={pending !== null}
                onChange={(event) => setMcpId(event.currentTarget.value)}
                placeholder="例如：filesystem"
                value={mcpId}
              />
            </label>
          </div>
          <label className="ops-form-field">
            <span>{mcpTarget === "codex" ? "TOML 配置体" : "JSON 配置体"}</span>
            <textarea
              className="ops-textarea context-toml-editor mono"
              disabled={pending !== null}
              onChange={(event) => setMcpBody(event.currentTarget.value)}
              spellCheck={false}
              value={mcpBody}
            />
          </label>
          <p className={`context-manager-note${mcpTarget === "codex" && !settings ? " warning" : ""}`}>
            {mcpTarget === "codex" && !settings
              ? "Codex 设置尚未加载，重新检测后再保存。"
              : `只写入 ${mcpTarget === "codex" ? "Codex" : "Claude"}，不会改变另一端。`}
          </p>
          <div className="action-row">
            <Button
              disabled={pending !== null || !mcpId.trim() || !mcpBody.trim() || (mcpTarget === "codex" && !settings)}
              onClick={() => void saveMcp()}
              size="sm"
            >
              <Save className="h-4 w-4" />
              {pending === "create:mcp" ? "保存中" : "保存 MCP"}
            </Button>
            <Button disabled={pending !== null} onClick={resetMcpDraft} size="sm" variant="outline">
              取消
            </Button>
          </div>
        </div>
      ) : null}
      <div className="unified-tool-countbar">
        <span>共 {inventory?.counts.total ?? 0} 项</span>
        <span>原始发现 {inventory?.counts.rawDiscoveries ?? 0}</span>
        <span>已合并 {inventory?.counts.deduplicated ?? 0}</span>
        <span className="claude-count">Claude {inventory?.counts.claudeEnabled ?? 0}</span>
        <span className="codex-count">Codex {inventory?.counts.codexEnabled ?? 0}</span>
      </div>
      <div className="context-tabs">
        {(["mcp", "skill", "plugin"] as ContextKind[]).map((kind) => (
          <button className={tab === kind ? "active" : ""} key={kind} onClick={() => setTab(kind)} type="button">
            <strong>{contextKindLabel(kind)}</strong>
            <span>{countFor(kind)}</span>
          </button>
        ))}
      </div>
      {inventory?.diagnostics.length ? (
        <p className="context-manager-note warning">检测到 {inventory.diagnostics.length} 项非致命诊断；其余可读来源仍已加载。</p>
      ) : (
        <p className="context-manager-note">已扫描 {inventory?.scannedSources.length ?? 0} 个配置或目录来源。</p>
      )}
      <div className="context-entry-list unified-tool-list">
        {entries.length ? entries.map((asset) => (
          <div className="context-entry-row unified-tool-row" key={`${asset.kind}:${asset.id}`}>
            <div className="unified-tool-copy">
              <strong>{asset.title || asset.id}</strong>
              {asset.summary ? <span title={asset.summary}>{asset.summary}</span> : null}
              {asset.source ? <small title={asset.source}>{compactPath(asset.source)}</small> : null}
            </div>
            <div className="agent-toggle-group" aria-label={`${asset.title} 应用状态`}>
              {(["claude", "codex"] as const).map((app) => {
                const state = asset[app];
                const appName = app === "claude" ? "Claude" : "Codex";
                const key = `${asset.kind}:${asset.id}:${app}`;
                const isPending = pending === key;
                return (
                  <button
                    aria-label={`${state.enabled ? "关闭" : "启用"} ${appName}：${asset.title}`}
                    className={`agent-toggle ${app} ${state.enabled ? "enabled" : "disabled"}${isPending ? " pending" : ""}`}
                    disabled={!state.toggleSupported || pending !== null}
                    key={app}
                    onClick={() => void toggle(asset, app)}
                    title={`${appName}${state.enabled ? " ✓（点击关闭）" : state.available ? "（点击启用）" : "（未发现可用来源）"}`}
                    type="button"
                  >
                    <img alt="" aria-hidden="true" src={app === "claude" ? claudeLogo : codexLogo} />
                  </button>
                );
              })}
            </div>
          </div>
        )) : <Empty text={result ? `未发现${contextKindLabel(tab)}；可点击重新检测查看最新本地状态。` : "尚未检测本地工具与插件。"} />}
      </div>
    </section>
  );
}

export function CodexPluginRepositoryPanel({
  actions,
  marketplace,
}: {
  actions: AppActions;
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
            label: "产品设计技能仓库",
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
        <StatusRow label="配置写入" status={status?.configRegistered ? "ok" : status?.needsRepair ? "needs_review" : "not_checked"} value={status?.configRegistered ? "已写入 Codex 配置" : "未写入或待检测"} />
        <StatusRow label="本地来源" status={status?.localSourcesReady ? "ok" : "needs_review"} value={status?.localSourcesReady ? "仓库快照存在并可读取" : "部分仓库只有配置，缺少本地来源"} />
        <StatusRow label="应用可见" status={status?.configRegistered && status?.localSourcesReady ? "needs_review" : "not_checked"} value={status?.runtimeConfirmation || "尚未确认"} />
        {repositories.map((repository) => (
          <StatusRow
            key={`${repository.name}:${repository.source}`}
            label={repository.label}
            status={repository.configured ? "ok" : "needs_review"}
            value={`${repository.name} / ${repository.sourceType} / ${repository.configured ? "配置已写入" : "配置未写入"} / ${repository.source}`}
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

export function ClaudePluginRepositoryPanel({
  actions,
  marketplace,
}: {
  actions: AppActions;
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
        <StatusRow label="应用可见" status={allConfigured ? "needs_review" : "not_checked"} value={allConfigured ? "配置已写入，待重启 Claude 确认" : "尚未确认"} />
        <StatusRow label="仓库列表" status={repositories.length ? (allConfigured ? "ok" : "needs_review") : "not_checked"} value={repositorySummary} />
        {repositories.map((repository) => (
          <StatusRow
            key={repository.repository}
            label={repository.label}
            status={repository.configured ? "ok" : "needs_review"}
            value={`${repository.repository} / ${repository.configured ? "配置已写入" : "配置未写入"}`}
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
export function MemoryAssistPanel({
  actions,
  exported,
  items,
  search,
  selfCheck,
  status,
}: {
  actions: AppActions;
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
  const [showArchived, setShowArchived] = useState(false);
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
        <Button onClick={() => void actions.registerMemoryMcpServer()} size="sm" variant="outline">
          <Network className="h-4 w-4" />
          注册 MCP 到 Claude/Codex
        </Button>
      </div>
      <div className="memory-assist-search">
        <label className="ops-form-field">
          <span>搜索记忆</span>
          <input
            onChange={(event) => setSearchQuery(event.currentTarget.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && searchQuery.trim()) void actions.searchMemoryAssist(searchQuery, showArchived);
            }}
            placeholder="搜索项目约定、构建命令、历史修复结论"
            value={searchQuery}
          />
        </label>
        <Button disabled={!searchQuery.trim()} onClick={() => void actions.searchMemoryAssist(searchQuery, showArchived)} size="sm" variant="outline">
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
          <div className="memory-list-header">
            <strong>经验教训手册</strong>
            <label className="memory-archive-toggle">
              <input
                checked={showArchived}
                onChange={(event) => {
                  const next = event.currentTarget.checked;
                  setShowArchived(next);
                  void actions.refreshMemoryAssist(false, next);
                }}
                type="checkbox"
              />
              <span>显示归档</span>
            </label>
          </div>
          {allItems.length ? allItems.map((item) => {
            const editing = editingMemoryId === item.id;
            const archived = item.tier === "archived";
            return (
            <div className={`memory-assist-row memory-lesson-card${archived ? " memory-archived" : ""}`} key={item.id}>
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
                  <MemoryTierControls actions={actions} item={item} />
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

function memoryManualHeadings(markdown: string) {
  return markdown
    .split(/\r?\n/)
    .map((line) => {
      const match = /^(#{2,3})\s+(.+?)\s*$/.exec(line);
      if (!match) return null;
      const title = match[2].trim();
      return { depth: match[1].length, title, anchor: title.toLowerCase().replace(/[^\p{L}\p{N}]+/gu, "-").replace(/^-|-$/g, "") };
    })
    .filter((item): item is { depth: number; title: string; anchor: string } => Boolean(item));
}

function fallbackMemoryManual(status: MemoryStatusResult | null) {
  const captureCount = status?.memory.totalCaptures ?? 0;
  const itemCount = status?.memory.totalItems ?? 0;
  const workspaceCount = status?.memory.workspaces?.length ?? 0;
  return [
    "# 会话经验教训注入手册",
    "",
    "> 用途：在新会话启动时注入由盘古核心算法提炼的经验教训。",
    "> 来源：memory_assist.sqlite 与真实 Codex/Claude 会话采集记录。",
    `> 更新时间：${new Date().toLocaleString()}`,
    "> 工作区：global",
    "",
    "## 目录",
    "",
    "- [当前状态](#当前状态)",
    "- [附录：来源摘要](#附录来源摘要)",
    "",
    "## 当前状态",
    "",
    "- 当前可用于生成手册的材料不足，等待核心算法从真实会话中提炼。",
    "",
    "## 附录：来源摘要",
    "",
    `- 来源长期记忆：${itemCount} 条`,
    `- 来源采集记录：${captureCount} 条`,
    `- 来源工作区：${workspaceCount} 个`,
    "- 生成方式：fallback",
  ].join("\n");
}

export function MemoryScreen({
  actions,
  candidates,
  dashboard,
  newProjectGuide,
  exported,
  items,
  search,
  selfCheck,
  migrateDataDir,
  selectDataDir,
  settings,
  status,
}: {
  actions: AppActions;
  candidates: MemoryCandidatesResult | null;
  dashboard: MemoryOutcomeDashboardResult | null;
  newProjectGuide: MemoryNewProjectGuideResult | null;
  exported: MemoryExportResult | null;
  items: MemoryItemsResult | null;
  search: MemoryQueryResult | null;
  selfCheck: MemorySelfCheckResult | null;
  migrateDataDir: (targetDir: string) => Promise<MemoryAssistMigrationResult | null>;
  selectDataDir: () => Promise<string | null>;
  settings: SettingsResult | null;
  status: MemoryStatusResult | null;
}) {
  const [manualEditing, setManualEditing] = useState(false);
  const [manualDraft, setManualDraft] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [showArchived, setShowArchived] = useState(false);
  const [showSources, setShowSources] = useState(false);
  const [editingMemoryId, setEditingMemoryId] = useState("");
  const [editingText, setEditingText] = useState("");
  const [editingCategory, setEditingCategory] = useState("");
  const [importText, setImportText] = useState("");
  const [replaceExisting, setReplaceExisting] = useState(false);
  const [selectedDataDir, setSelectedDataDir] = useState("");
  const [storageMigrationBusy, setStorageMigrationBusy] = useState(false);

  const allItems = items?.items ?? [];
  const activeItems = allItems.filter((item) => item.tier !== "archived");
  const archivedItems = allItems.filter((item) => item.tier === "archived");
  const exemptItems = allItems.filter((item) => item.exempt);
  const sourceItems = showArchived ? allItems : activeItems;
  const manualItem =
    activeItems.find((item) => item.category === "lesson-manual") ??
    allItems.find((item) => item.category === "lesson-manual");
  const manualText = manualItem?.text?.trim() || fallbackMemoryManual(status);
  const headings = memoryManualHeadings(manualEditing ? manualDraft : manualText);
  const avgStrength = allItems.length ? allItems.reduce((total, item) => total + (item.strength ?? 0), 0) / allItems.length : 0;
  const avgRetention = allItems.length ? allItems.reduce((total, item) => total + (item.retention ?? 0), 0) / allItems.length : 0;
  const captureProgress = status?.memory.captureProgress;
  const latestWorkspaceCapture = Math.max(0, ...(status?.memory.workspaces ?? []).map((workspace) => workspace.latestCaptureAt || 0));
  const latestScanAt = captureProgress?.lastScanAt || latestWorkspaceCapture;
  const firstBaselineAt = captureProgress?.firstBaselineAt || 0;
  const workspaceSummary = status?.memory.codexWorkspace || status?.memory.workspaces?.[0]?.workspace || "global";
  const outcome = dashboard?.dashboard;
  const pendingCandidates = candidates?.candidates ?? [];
  const handoffItems = outcome?.handoffItems ?? [];
  const continueItems = handoffItems.slice(0, 3);
  const guide = newProjectGuide?.guide;
  const guidePitfalls = guide?.pitfalls ?? [];
  const guideApproaches = guide?.bestPractices ?? [];
  const guidePrompt = guide?.prompt ?? "";
  const guideSelectedCount = guidePitfalls.length + guideApproaches.length;
  const projectWorkspace = outcome?.workspace || workspaceSummary;
  const latestHandoffUpdatedAt = handoffItems.reduce((latest, item) => Math.max(latest, item.updatedAt || 0), 0);
  const projectItems = sourceItems.filter((item) => item.workspace === projectWorkspace || item.workspace === "global");
  const trend = outcome?.trend ?? [];
  const trendMax = Math.max(1, ...trend.map((point) => point.captures + point.learned + point.recalls));
  const mcpEnabled = Boolean(settings?.settings.memoryAssistMcpEnabled);
  const memoryEnabled = status?.memory.enabled ?? Boolean(settings?.settings.memoryAssistEnabled);
  const exportJson = exported ? JSON.stringify(exported.data, null, 2) : "";
  const matches = search?.memory.results ?? [];
  const selfCheckSummary = selfCheck
    ? selfCheck.report.checks.map((check) => `${check.name}:${check.status}`).join(" / ")
    : "等待自检";
  const currentDbPath = status?.memory.dbPath?.trim() || "";
  const currentDataDir = (() => {
    const separator = Math.max(currentDbPath.lastIndexOf("/"), currentDbPath.lastIndexOf("\\"));
    if (separator < 0) return currentDbPath || "等待状态加载";
    if (separator === 0) return currentDbPath.slice(0, 1);
    if (separator === 2 && /^[A-Za-z]:/.test(currentDbPath)) return currentDbPath.slice(0, 3);
    return currentDbPath.slice(0, separator);
  })();
  const normalizedCurrentDataDir = currentDataDir.replace(/[\\/]+$/, "").toLocaleLowerCase();
  const normalizedSelectedDataDir = selectedDataDir.trim().replace(/[\\/]+$/, "").toLocaleLowerCase();

  const chooseDataDir = async () => {
    const selected = await selectDataDir();
    if (selected) setSelectedDataDir(selected);
  };
  const migrateData = async () => {
    const targetDir = selectedDataDir.trim();
    if (!targetDir || storageMigrationBusy) return;
    setStorageMigrationBusy(true);
    try {
      const result = await migrateDataDir(targetDir);
      if (result) setSelectedDataDir("");
    } finally {
      setStorageMigrationBusy(false);
    }
  };

  const beginManualEdit = () => {
    setManualDraft(manualText);
    setManualEditing(true);
  };
  const saveManual = async () => {
    const text = manualDraft.trim();
    if (!text) return;
    if (manualItem) {
      const saved = await actions.updateMemoryAssistItem(manualItem.id, {
        text,
        workspace: manualItem.workspace,
        category: manualItem.category,
        tags: manualItem.tags,
        source: manualItem.source || "manager",
        sourceSessionId: manualItem.sourceSessionId,
      });
      if (saved) setManualEditing(false);
      return;
    }
    const saved = await actions.learnMemoryAssistItem(text, "lesson-manual");
    if (saved) setManualEditing(false);
  };
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
  const copyManual = async () => {
    await navigator.clipboard?.writeText(manualEditing ? manualDraft : manualText);
    actions.showNotice({ title: "复制注入手册", message: "已复制会话经验教训注入手册全文。", status: "ok" });
  };
  const copyHandoff = async () => {
    const text = [
      `# 项目接续：${projectWorkspace}`,
      "",
      ...continueItems.map((item) => `- [${item.category}] ${item.text}`),
    ].join("\n");
    await navigator.clipboard?.writeText(text);
    actions.showNotice({ title: "复制项目接续", message: "已复制当前看板中的项目接续摘要。", status: "ok" });
  };
  const recallMethodLabel = (eventType: string) => {
    if (eventType === "inject") return "会话注入";
    if (eventType === "search") return "搜索命中";
    return eventType || "召回";
  };
  const guideText = (item: MemoryNewProjectExperience) => `${item.text}（${item.category} · ${item.sourceCount} 条来源）`;
  const copyNewProjectPrompt = async () => {
    if (!guidePrompt) return;
    await navigator.clipboard?.writeText(guidePrompt);
    actions.showNotice({ title: "复制新项目提示词", message: "已复制完整的新项目启动提示词。", status: "ok" });
  };

  return (
    <div className="stack memory-page">
      <Panel title="开始工作" detail="选择继续当前项目，或按需生成一份基于既有记忆的新项目启动指南。">
        <div className="memory-start-grid">
          <section className="memory-start-card">
            <div><strong>继续当前项目</strong><span>{projectWorkspace}</span></div>
            <span className="memory-start-updated">最近更新：{latestHandoffUpdatedAt ? new Date(latestHandoffUpdatedAt * 1000).toLocaleString() : "暂无接续更新时间"}</span>
            <div className="memory-handoff-list">
              {continueItems.length ? continueItems.map((item) => (
                <article className="memory-outcome-item compact" key={`handoff-${item.id}`}>
                  <span>{item.category}</span>
                  <p>{item.text}</p>
                </article>
              )) : <Empty text="当前项目尚无可用于接续的关键记录。" />}
            </div>
            <Button disabled={!continueItems.length} onClick={() => void copyHandoff()} size="sm" variant="outline"><Copy className="h-4 w-4" />复制项目接续</Button>
          </section>
          <section className="memory-start-card">
            <div><strong>开启新项目</strong><span>跨项目经验指南</span></div>
            <p>需要时再从已有记忆中整理避坑、优秀方式与完整提示词，不会在进入页面时自动生成。</p>
            <Button onClick={() => void actions.loadMemoryNewProjectGuide()} size="sm">{newProjectGuide ? "重新生成预览" : "生成启动指南"}</Button>
          </section>
        </div>
        {newProjectGuide ? (
          <div className="memory-new-project-preview">
            <div className="memory-guide-stats">
              <InfoRow label="来源范围" value={`${guide?.sourceWorkspaceCount ?? 0} 个`} />
              <InfoRow label="源记忆" value={`${guide?.sourceItemCount ?? 0} 条`} />
              <InfoRow label="精选经验" value={`${guideSelectedCount} 条`} />
              <InfoRow label="来源更新截至" value={guide?.generatedAt ? new Date(guide.generatedAt * 1000).toLocaleString() : "未记录"} />
            </div>
            <section><strong>避坑</strong>{guidePitfalls.length ? <ul>{guidePitfalls.map((item, index) => <li key={`pitfall-${index}`}>{guideText(item)}</li>)}</ul> : <Empty text="暂无可提炼的避坑记录。" />}</section>
            <section><strong>优秀方式</strong>{guideApproaches.length ? <ul>{guideApproaches.map((item, index) => <li key={`approach-${index}`}>{guideText(item)}</li>)}</ul> : <Empty text="暂无可提炼的优秀方式。" />}</section>
            <section className="memory-new-project-prompt"><div><strong>完整提示词</strong><Button disabled={!guidePrompt} onClick={() => void copyNewProjectPrompt()} size="sm" variant="outline"><Copy className="h-4 w-4" />复制提示词</Button></div>{guidePrompt ? <pre>{guidePrompt}</pre> : <Empty text="当前记忆不足，尚未生成完整提示词。" />}</section>
          </div>
        ) : null}
      </Panel>

      <Panel title="记忆成果" detail="今日结果、7/30 天趋势与真实召回证据均来自本地记忆库记录。">
        <div className="memory-outcome-stats">
          <InfoRow label="今日采集" value={`${outcome?.todayCaptures ?? 0} 条`} />
          <InfoRow label="新增长期记忆" value={`${outcome?.todayLearned ?? 0} 条`} />
          <InfoRow label="待确认" value={`${outcome?.pendingCandidates ?? pendingCandidates.length} 条`} />
          <InfoRow label="真实命中" value={`${outcome?.todayRecalls ?? 0} 条`} />
        </div>
        <div className="memory-outcome-toolbar">
          <span>最近 {outcome?.rangeDays ?? 30} 天</span>
          <div className="action-row">
            <Button onClick={() => void actions.refreshMemoryOutcomeDashboard(7)} size="sm" variant={(outcome?.rangeDays ?? 30) === 7 ? "default" : "outline"}>7 天</Button>
            <Button onClick={() => void actions.refreshMemoryOutcomeDashboard(30)} size="sm" variant={(outcome?.rangeDays ?? 30) === 30 ? "default" : "outline"}>30 天</Button>
          </div>
        </div>
        {trend.length ? (
          <div className="memory-trend-chart" aria-label={`${outcome?.rangeDays ?? 30} 天记忆趋势`}>
            {trend.map((point) => (
              <div className="memory-trend-column" key={point.date} title={`${point.date}：采集 ${point.captures}，新增 ${point.learned}，召回 ${point.recalls}`}>
                <div className="memory-trend-bars">
                  <i className="capture" style={{ height: point.captures ? `${Math.max(3, (point.captures / trendMax) * 100)}%` : 0 }} />
                  <i className="learned" style={{ height: point.learned ? `${Math.max(3, (point.learned / trendMax) * 100)}%` : 0 }} />
                  <i className="recall" style={{ height: point.recalls ? `${Math.max(3, (point.recalls / trendMax) * 100)}%` : 0 }} />
                </div>
                <span>{point.date.slice(5)}</span>
              </div>
            ))}
          </div>
        ) : <Empty text="所选时间范围内暂无趋势记录。" />}
        <div className="memory-trend-legend"><span>采集</span><span>新增</span><span>命中条目</span></div>
        <div className="memory-breakdown-grid">
          <div className="memory-breakdown-list">
            <strong>项目分布</strong>
            {(outcome?.workspaceBreakdown ?? []).map((item) => <span key={`workspace-${item.key}`}>{item.key}<b>{item.count}</b></span>)}
            {outcome?.workspaceBreakdown.length ? null : <em>暂无项目分布</em>}
          </div>
          <div className="memory-breakdown-list">
            <strong>类别分布</strong>
            {(outcome?.categoryBreakdown ?? []).map((item) => <span key={`category-${item.key}`}>{item.key}<b>{item.count}</b></span>)}
            {outcome?.categoryBreakdown.length ? null : <em>暂无类别分布</em>}
          </div>
        </div>
        <h3 className="memory-outcome-subtitle">最近真实召回</h3>
        <div className="memory-recall-list">
          {outcome?.recentRecalls.length ? outcome.recentRecalls.map((event) => (
            <article className="memory-recall-card" key={event.id}>
              <div><strong>{event.agent || "unknown"}</strong><span>{recallMethodLabel(event.eventType)} · {new Date(event.createdAt * 1000).toLocaleString()}</span></div>
              <p>查询：{event.querySummary || "未记录查询摘要"}</p>
              <blockquote>{event.memory?.text || "命中记忆当前不可用"}</blockquote>
            </article>
          )) : <Empty text="尚无可验证召回记录" />}
        </div>
      </Panel>

      <Panel title="待确认" detail="候选记忆在确认后才会进入长期记忆；忽略不会删除其他记忆。">
        <div className="memory-outcome-list">
          {pendingCandidates.length ? pendingCandidates.map((candidate) => (
            <article className="memory-outcome-item" key={candidate.id}>
              <span>{candidate.category} · {candidate.workspace} · {candidate.source}</span>
              <p>{candidate.text}</p>
              {candidate.reason ? <em>{candidate.reason}</em> : null}
              <div className="action-row">
                <Button onClick={() => void actions.approveMemoryAssistCandidate(candidate.id)} size="sm">确认记忆</Button>
                <Button onClick={() => void actions.rejectMemoryAssistCandidate(candidate.id)} size="sm" variant="outline">忽略</Button>
              </div>
            </article>
          )) : <Empty text="当前没有待确认记忆。" />}
        </div>
      </Panel>

      <Panel title="项目记忆" detail="搜索、编辑以及归档或恢复当前项目与 global 记忆。">
        <div className="memory-source-toolbar">
          <label className="memory-archive-toggle">
            <input checked={showArchived} onChange={(event) => { const next = event.currentTarget.checked; setShowArchived(next); void actions.refreshMemoryAssist(false, next); }} type="checkbox" />
            <span>显示归档</span>
          </label>
          <Button onClick={() => void actions.refreshMemoryAssist()} size="sm" variant="outline"><RefreshCw className="h-4 w-4" />刷新</Button>
        </div>
        <div className="memory-assist-search">
          <label className="ops-form-field">
            <span>搜索项目记忆</span>
            <input onChange={(event) => setSearchQuery(event.currentTarget.value)} onKeyDown={(event) => { if (event.key === "Enter" && searchQuery.trim()) void actions.searchMemoryAssist(searchQuery, showArchived); }} placeholder="搜索项目约定、历史修复或进度" value={searchQuery} />
          </label>
          <Button disabled={!searchQuery.trim()} onClick={() => void actions.searchMemoryAssist(searchQuery, showArchived)} size="sm" variant="outline">搜索</Button>
        </div>
        {matches.length ? (
          <div className="memory-assist-list">
            <strong>搜索结果：{search?.memory.query}</strong>
            {matches.slice(0, 8).map((match) => <div className="memory-assist-row" key={`outcome-match-${match.item.id}`}><span>{match.item.category} · {match.item.workspace}</span><p>{match.item.text}</p></div>)}
          </div>
        ) : search ? <Empty text="没有匹配到项目记忆。" /> : null}
        <details className="memory-project-list">
          <summary>查看项目记忆（{projectItems.length}）</summary>
          <div className="memory-assist-list">
          {projectItems.length ? projectItems.map((item) => {
            const editing = editingMemoryId === item.id;
            return (
              <article className={`memory-assist-row memory-lesson-card${item.tier === "archived" ? " memory-archived" : ""}`} key={`project-${item.id}`}>
                <span>{item.category} · {item.workspace}</span>
                {editing ? <><label className="ops-form-field"><span>分类</span><input onChange={(event) => setEditingCategory(event.currentTarget.value)} value={editingCategory} /></label><label className="ops-form-field"><span>记忆内容</span><textarea className="ops-textarea compact" onChange={(event) => setEditingText(event.currentTarget.value)} value={editingText} /></label><div className="action-row"><Button disabled={!editingText.trim()} onClick={() => void saveEditedMemory(item)} size="sm"><Save className="h-4 w-4" />保存</Button><Button onClick={cancelEditMemory} size="sm" variant="outline">取消</Button></div></> : <><p>{item.text}</p><MemoryTierControls actions={actions} item={item} /><Button onClick={() => beginEditMemory(item)} size="sm" variant="outline"><Pencil className="h-4 w-4" />编辑</Button></>}
              </article>
            );
          }) : <Empty text="当前项目暂无记忆。" />}
          </div>
        </details>
      </Panel>

      <details className="memory-diagnostics">
        <summary>高级诊断</summary>
        <div className="memory-diagnostics-body">
      <Panel title="记忆存储" detail="迁移会保留原数据；完成后需要重启正在运行的 Codex、Launcher 和 MCP。">
        <div className="ops-status-list">
          <StatusRow label="当前目录" status={currentDbPath ? "ok" : "not_checked"} value={currentDataDir} />
          <StatusRow label="目标目录" status={selectedDataDir ? "running" : "not_checked"} value={selectedDataDir || "尚未选择"} />
        </div>
        <div className="action-row">
          <Button disabled={storageMigrationBusy} onClick={() => void chooseDataDir()} size="sm" variant="outline">
            <FolderOpen className="h-4 w-4" />
            选择目录
          </Button>
          <Button
            disabled={!normalizedSelectedDataDir || normalizedSelectedDataDir === normalizedCurrentDataDir || storageMigrationBusy}
            onClick={() => void migrateData()}
            size="sm"
          >
            <FileDown className="h-4 w-4" />
            {storageMigrationBusy ? "迁移中" : "迁移数据"}
          </Button>
        </div>
      </Panel>
      <section className="memory-layer-grid" aria-label="盘古记忆三层链路状态">
        <Panel title="历史会话采集层" detail="主路径：从 Codex / Claude 本地可解析会话源建立采集证据。">
          <div className="ops-status-list">
            <StatusRow label="Codex 会话源" status="running" value="session DB / rollout 文件" />
            <StatusRow label="Claude 会话源" status="running" value="claude-code / local-agent / audit / .claude" />
            <StatusRow label="采集记录" status={(status?.memory.totalCaptures ?? 0) > 0 ? "ok" : "not_checked"} value={`${status?.memory.totalCaptures ?? 0} 条`} />
            <StatusRow label="扫描模式" status="running" value={(status?.memory.totalCaptures ?? 0) > 0 ? "增量采集中" : "首次全量待建立"} />
            <StatusRow label="最近扫描" status={latestWorkspaceCapture > 0 ? "ok" : "not_checked"} value={latestWorkspaceCapture ? new Date(latestWorkspaceCapture * 1000).toLocaleString() : "等待扫描"} />
          </div>
        </Panel>
        <Panel title="注入实时监听层" detail="辅助路径：只负责页面实时状态、workspace/thread 与最近输入。">
          <div className="ops-status-list">
            <StatusRow label="Codex 注入" status={status?.memory.codexInjected ? "ok" : "not_checked"} value={status?.memory.codexInjected ? "已注入" : "等待 Codex 注入"} />
            <StatusRow label="Codex 对话监控" status={status?.memory.active ? "running" : "not_checked"} value={status?.memory.active ? "监控中" : "等待会话变化"} />
            <StatusRow label="当前 workspace/thread" status={workspaceSummary ? "ok" : "not_checked"} value={workspaceSummary} />
            <StatusRow label="Claude 使用方式" status="ok" value="MCP 共享，不走前端注入" />
          </div>
        </Panel>
        <Panel title="核心算法裁判层" detail="裁判路径：去重、分类、留存、归档、手册生成与自检。">
          <div className="ops-status-list">
            <StatusRow label="待处理候选" status={(status?.memory.pendingCandidates ?? 0) > 0 ? "running" : "ok"} value={`${status?.memory.pendingCandidates ?? 0} 条`} />
            <StatusRow label="长期记忆" status={(status?.memory.totalItems ?? 0) > 0 ? "ok" : "not_checked"} value={`${status?.memory.totalItems ?? 0} 条`} />
            <StatusRow label="已归档" status={archivedItems.length ? "ok" : "not_checked"} value={`${archivedItems.length} 条`} />
            <StatusRow label="注入手册" status={manualItem ? "ok" : "not_checked"} value={manualItem ? "已进入注入缓存" : "等待提炼"} />
            <StatusRow label="自检摘要" status={selfCheck?.report.status ?? "not_checked"} value={selfCheckSummary} />
          </div>
        </Panel>
      </section>

      <Panel title="增量采集状态" detail="首次建立采集进度；后续只扫描新增会话或已有会话新增上下文。">
        <div className="memory-stat-grid">
          <InfoRow label="首次基线" value={firstBaselineAt > 0 ? `已建立 · ${new Date(firstBaselineAt * 1000).toLocaleString()}` : "等待首次全量建立"} />
          <InfoRow label="采集进度" value={`${captureProgress?.totalSources ?? 0} 个源已建立采集进度`} />
          <InfoRow label="Codex 源" value={`${captureProgress?.codexSources ?? 0} 个 · session DB / rollout / 实时注入`} />
          <InfoRow label="Claude 源" value={`${captureProgress?.claudeSources ?? 0} 个 · audit.jsonl / local_*.json / .claude/sessions`} />
          <InfoRow label="新增上下文" value={`${captureProgress?.newContextCount ?? 0} 条 · 最近一次扫描`} />
          <InfoRow label="跳过未变化" value={`${captureProgress?.skippedUnchangedSessions ?? 0} 个会话源 · 最近一次扫描`} />
        </div>
        <div className="ops-note">
          <ShieldCheck className="h-4 w-4" />
          <span>采集层只拿全可解析对话上下文；留存、去重、分类、归档、手册生成统一交给盘古核心算法。</span>
        </div>
      </Panel>

      <Panel title="会话经验教训注入手册" detail="单个 Markdown 注入文档；前端只展示核心算法生成内容与目录，不写死业务章节。">
        <div className="memory-manual-toolbar">
          <Button onClick={() => void actions.refineLongTermMemory()} size="sm">
            <PencilRuler className="h-4 w-4" />
            重新提炼会话经验
          </Button>
          {manualEditing ? (
            <>
              <Button disabled={!manualDraft.trim()} onClick={() => void saveManual()} size="sm" variant="outline">
                <Save className="h-4 w-4" />
                保存
              </Button>
              <Button onClick={() => setManualEditing(false)} size="sm" variant="outline">取消</Button>
            </>
          ) : (
            <Button onClick={beginManualEdit} size="sm" variant="outline">
              <Pencil className="h-4 w-4" />
              编辑注入手册
            </Button>
          )}
          <Button onClick={() => void copyManual()} size="sm" variant="outline">
            <Copy className="h-4 w-4" />
            复制全文
          </Button>
          <Button onClick={() => setShowSources(true)} size="sm" variant="outline">查看来源条目</Button>
        </div>
        <div className="memory-manual-meta">
          <InfoRow label="更新时间" value={manualItem ? new Date(manualItem.updatedAt * 1000).toLocaleString() : "fallback 预览"} />
          <InfoRow label="来源统计" value={`长期记忆 ${status?.memory.totalItems ?? 0} 条 / 采集记录 ${status?.memory.totalCaptures ?? 0} 条 / 工作区 ${status?.memory.workspaces?.length ?? 0} 个`} />
          <InfoRow label="同步注入缓存" value={status?.memory.injectSummaryCachePath ? compactPath(status.memory.injectSummaryCachePath) : "等待生成"} />
          <InfoRow label="生成方式" value={manualItem?.source || "fallback"} />
        </div>
        <div className="memory-manual-layout">
          <aside className="memory-manual-toc">
            <strong>目录</strong>
            {headings.length ? headings.map((heading) => (
              <span className={heading.depth === 3 ? "child" : ""} key={`${heading.depth}:${heading.title}`}>{heading.title}</span>
            )) : <em>暂无标题</em>}
          </aside>
          {manualEditing ? (
            <textarea className="ops-textarea mono memory-manual-editor" onChange={(event) => setManualDraft(event.currentTarget.value)} value={manualDraft} />
          ) : (
            <pre className="memory-manual-document">{manualText}</pre>
          )}
        </div>
        <div className="ops-note">
          <Archive className="h-4 w-4" />
          <span>压缩整合会将源条目软归档而非物理删除；手册使用稳定 ID 增量更新。</span>
        </div>
      </Panel>

      <Panel title="遗忘曲线与记忆分层" detail="Ebbinghaus 衰减 + active/archive 两层；注入只看 active 层。">
        <div className="memory-stat-grid">
          <InfoRow label="active 记忆" value={`${activeItems.length} 条`} />
          <InfoRow label="archived 记忆" value={`${archivedItems.length} 条`} />
          <InfoRow label="常驻豁免" value={`${exemptItems.length} 条`} />
          <InfoRow label="平均 strength" value={`${Math.round(avgStrength * 100)}%`} />
          <InfoRow label="平均 retention" value={`${Math.round(avgRetention * 100)}%`} />
          <InfoRow label="注入层" value="只使用 active 层" />
        </div>
        <div className="ops-note">
          <Pin className="h-4 w-4" />
          <span>manual / safety-rule / project-rule 等常驻记忆不受自动归档影响。归档是软标记，可恢复，不是删除。</span>
        </div>
        <div className="memory-tier-preview">
          {sourceItems.slice(0, 6).map((item) => (
            <div className={`memory-assist-row${item.tier === "archived" ? " memory-archived" : ""}`} key={`tier-${item.id}`}>
              <span>{item.category} · {item.workspace}</span>
              <p>{item.text}</p>
              <MemoryTierControls actions={actions} item={item} />
            </div>
          ))}
          {sourceItems.length ? null : <Empty text="暂无可展示的记忆分层条目。" />}
        </div>
      </Panel>

      <Panel title="MCP 跨 Agent 共享" detail="Claude/Codex 通过同一份 sqlite 与 MCP 工具共享盘古记忆。">
        <div className="memory-stat-grid">
          <InfoRow label="MCP 开关" value={mcpEnabled ? "已开启" : "未开启"} />
          <InfoRow label="总开关" value={memoryEnabled ? "盘古记忆已启用" : "盘古记忆未启用"} />
          <InfoRow label="共享数据库" value={compactPath(status?.memory.dbPath)} />
          <InfoRow label="MCP 注册状态" value={mcpEnabled ? "可注册/刷新" : "等待开启后注册"} />
          <InfoRow label="Claude 使用方式" value="通过 MCP 共享同一份盘古记忆" />
          <InfoRow label="Codex 使用方式" value="前端注入 + 本地会话采集 + MCP 可选" />
        </div>
        <div className="memory-tool-grid">
          {[
            ["memory_search", "检索记忆"],
            ["memory_list", "列出记忆"],
            ["memory_recent", "最近记忆"],
            ["memory_learn", "写入记忆"],
          ].map(([name, detail]) => (
            <div className="memory-tool-card" key={name}>
              <strong>{name}</strong>
              <span>{detail}</span>
            </div>
          ))}
        </div>
        <div className="action-row">
          <Button onClick={() => void actions.registerMemoryMcpServer()} size="sm">
            <Network className="h-4 w-4" />
            注册 MCP 到 Claude/Codex
          </Button>
          <Button onClick={() => void actions.refreshMemoryAssist()} size="sm" variant="outline">
            <RefreshCw className="h-4 w-4" />
            刷新 MCP 状态
          </Button>
        </div>
      </Panel>

      <Panel title="来源条目审查" detail="默认折叠；支持搜索、显示归档、编辑、删除、归档/恢复、导入导出。">
        <div className="memory-source-toolbar">
          <Button onClick={() => setShowSources((value) => !value)} size="sm" variant="outline">
            {showSources ? "折叠来源条目" : "展开来源条目"}
          </Button>
          <label className="memory-archive-toggle">
            <input
              checked={showArchived}
              onChange={(event) => {
                const next = event.currentTarget.checked;
                setShowArchived(next);
                void actions.refreshMemoryAssist(false, next);
              }}
              type="checkbox"
            />
            <span>显示归档</span>
          </label>
          <Button onClick={() => void actions.exportMemoryAssist()} size="sm" variant="outline">
            <FileDown className="h-4 w-4" />
            导出
          </Button>
        </div>
        {showSources ? (
          <>
            <div className="memory-assist-search">
              <label className="ops-form-field">
                <span>搜索来源条目</span>
                <input
                  onChange={(event) => setSearchQuery(event.currentTarget.value)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" && searchQuery.trim()) void actions.searchMemoryAssist(searchQuery, showArchived);
                  }}
                  placeholder="搜索项目约定、经验教训、历史修复..."
                  value={searchQuery}
                />
              </label>
              <Button disabled={!searchQuery.trim()} onClick={() => void actions.searchMemoryAssist(searchQuery, showArchived)} size="sm" variant="outline">
                <RefreshCw className="h-4 w-4" />
                搜索
              </Button>
            </div>
            {matches.length ? (
              <div className="memory-assist-list">
                <strong>搜索结果：{search?.memory.query}</strong>
                {matches.slice(0, 8).map((match) => (
                  <div className="memory-assist-row" key={`match-${match.item.id}`}>
                    <span>{match.item.category} · {match.item.workspace} · score {match.score.toFixed(2)}</span>
                    <p>{match.item.text}</p>
                    {match.matchedKeywords.length ? <em>命中：{match.matchedKeywords.slice(0, 8).join(" / ")}</em> : null}
                  </div>
                ))}
              </div>
            ) : search ? <Empty text="没有匹配到来源条目。" /> : null}
            <div className="memory-assist-list">
              {sourceItems.length ? sourceItems.map((item) => {
                const editing = editingMemoryId === item.id;
                const archived = item.tier === "archived";
                return (
                  <div className={`memory-assist-row memory-lesson-card${archived ? " memory-archived" : ""}`} key={item.id}>
                    <span>{item.category} · {item.workspace}</span>
                    {editing ? (
                      <>
                        <label className="ops-form-field">
                          <span>分类</span>
                          <input onChange={(event) => setEditingCategory(event.currentTarget.value)} value={editingCategory} />
                        </label>
                        <label className="ops-form-field">
                          <span>来源内容</span>
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
                        <MemoryTierControls actions={actions} item={item} />
                        <div className="action-row">
                          <Button onClick={() => beginEditMemory(item)} size="sm" variant="outline">
                            <Pencil className="h-4 w-4" />
                            编辑
                          </Button>
                          <Button onClick={() => void actions.deleteMemoryAssistItem(item.id)} size="sm" variant="outline">
                            <Trash2 className="h-4 w-4" />
                            删除
                          </Button>
                        </div>
                      </>
                    )}
                  </div>
                );
              }) : <Empty text="暂无来源条目。" />}
            </div>
            <div className="memory-assist-transfer">
              <div className="memory-assist-list">
                <strong>导出 JSON</strong>
                <textarea className="ops-textarea compact mono" readOnly value={exportJson} />
              </div>
              <div className="memory-assist-list">
                <strong>导入 JSON</strong>
                <textarea className="ops-textarea compact mono" onChange={(event) => setImportText(event.currentTarget.value)} placeholder="粘贴 memory-assist/v1 导出 JSON。" value={importText} />
                <div className="ops-toggle-line">
                  <span>替换已有记忆</span>
                  <ToggleSwitch checked={replaceExisting} onChange={setReplaceExisting} />
                </div>
                <Button disabled={!importText.trim()} onClick={() => void actions.importMemoryAssist(importText, replaceExisting)} size="sm">
                  <FileUp className="h-4 w-4" />
                  导入
                </Button>
              </div>
            </div>
          </>
        ) : <Empty text="来源条目已折叠。点击展开后可审查、搜索、编辑和归档恢复。" />}
      </Panel>
        </div>
      </details>
    </div>
  );
}

export const SessionManagementScreen = memo(function SessionManagementScreen({
  actions,
  codexSessionContext,
  codexSessionContextError,
  codexSessionContextLoading,
  codexSessionContextTarget,
  claudeSessionContext,
  claudeSessionContextError,
  claudeSessionContextLoading,
  claudeSessionContextTarget,
  claudeSessions,
  localSessions,
  providerSync,
  settings,
}: {
  actions: AppActions;
  codexSessionContext: CodexSessionContextPage | null;
  codexSessionContextError: string;
  codexSessionContextLoading: boolean;
  codexSessionContextTarget: LocalSession | null;
  claudeSessionContext: ClaudeSessionContextPage | null;
  claudeSessionContextError: string;
  claudeSessionContextLoading: boolean;
  claudeSessionContextTarget: ClaudeSession | null;
  claudeSessions: ClaudeSessionsResult | null;
  localSessions: LocalSessionsResult | null;
  providerSync: ProviderSyncResult | null;
  settings: SettingsResult | null;
}) {
  const codexSessions = useMemo(() => localSessions?.sessions ?? [], [localSessions]);
  const codexSessionProjectGroups = useMemo(() => groupLocalSessionsByProject(codexSessions), [codexSessions]);
  const claudeSessionsList = useMemo(() => claudeSessions?.sessions ?? [], [claudeSessions]);
  const claudeSessionProjectGroups = useMemo(() => groupClaudeSessionsByProject(claudeSessionsList), [claudeSessionsList]);
  const claudeContextPayload = claudeSessionContext;
  const showingCodexContext = Boolean(codexSessionContextTarget);
  const sessionContextTarget = codexSessionContextTarget ?? claudeSessionContextTarget;
  const sessionContextPayload = codexSessionContext ?? claudeSessionContext;
  const sessionContextLoading = showingCodexContext ? codexSessionContextLoading : claudeSessionContextLoading;
  const sessionContextError = showingCodexContext ? codexSessionContextError : claudeSessionContextError;
  const closeSessionContext = showingCodexContext ? actions.closeCodexSessionContext : actions.closeClaudeSessionContext;
  const loadEarlierSessionContext = showingCodexContext ? actions.loadEarlierCodexSessionContext : actions.loadEarlierClaudeSessionContext;
  const claudeContextDialogRef = useRef<HTMLElement | null>(null);
  useEffect(() => {
    if (!sessionContextTarget) return;
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    const focusFrame = window.requestAnimationFrame(() => claudeContextDialogRef.current?.focus());
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") closeSessionContext();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.cancelAnimationFrame(focusFrame);
      window.removeEventListener("keydown", handleKeyDown);
      document.body.style.overflow = previousOverflow;
    };
  }, [closeSessionContext, sessionContextTarget]);
  const syncSummary = providerSync
    ? `${providerSync.changedSessionFiles ?? 0} 个会话文件，${providerSync.sqliteRowsUpdated ?? 0} 行索引`
    : "尚未执行";
  const renderSessionBrowserPanel = <T extends LocalSession | ClaudeSession,>({
    title,
    ariaLabel,
    emptyText,
    data,
    groups,
    onRefresh,
    onOpen,
    onDelete,
    sourceLabel = "数据库",
    sourceCountLabel = "候选库",
    statusLabel,
  }: {
    title: string;
    ariaLabel: string;
    emptyText: string;
    data: LocalSessionsResult | ClaudeSessionsResult | null;
    groups: Array<{ key: string; label: string; subtitle: string; sessions: T[] }>;
    onRefresh?: () => void;
    onOpen?: (session: T) => void;
    onDelete?: (session: T) => void;
    sourceLabel?: string;
    sourceCountLabel?: string;
    statusLabel?: string;
  }) => {
    const sessionCount = data?.sessions.length ?? 0;
    const sourceRoot = data ? ("sourceRoot" in data ? data.sourceRoot : data.dbPath) : "";
    const sourcePaths = data ? ("sourcePaths" in data ? data.sourcePaths : data.dbPaths) : [];
    const warningCount = data && "warnings" in data ? data.warnings.length : 0;
    const loadFailed = Boolean(data && statusFailed(data.status));
    return (
      <Panel title={title} detail={`${sessionCount} 个本地会话；删除会先写备份。${warningCount ? ` ${warningCount} 个来源需要检查。` : ""}`}>
        <div className="codex-session-toolbar">
          <div>
            <span>{sourceLabel}</span>
            <strong>{sourceRoot ? compactPath(sourceRoot) : statusLabel || "尚未读取"}</strong>
          </div>
          <div>
            <span>{sourceCountLabel}</span>
            <strong>{sourcePaths.length} 个</strong>
          </div>
          <div>
            <span>会话数</span>
            <strong>{sessionCount} 个</strong>
          </div>
          {onRefresh ? (
            <Button onClick={onRefresh} size="sm" variant="outline">
              <RefreshCw className="h-4 w-4" />
              刷新
            </Button>
          ) : null}
        </div>
        {loadFailed ? (
          <div className="ops-danger-zone" role="alert">
            <AlertTriangle className="h-4 w-4" />
            <span>{data?.message || "会话加载失败，请刷新后重试。"}</span>
          </div>
        ) : (
          <div className="codex-session-browser" aria-label={ariaLabel}>
            <div className="codex-session-browser-title">项目</div>
            {groups.length ? groups.map((group) => (
              <section className="codex-session-project" key={`${title}:${group.key}`}>
                <div className="codex-session-project-header" title={group.subtitle || group.label}>
                  <FileCode2 className="h-4 w-4" />
                  <strong>{group.label}</strong>
                </div>
                <div className="codex-session-project-list">
                  {group.sessions.map((session) => (
                    <div className="codex-session-row" key={`${title}:${"sourcePath" in session ? session.sourcePath : session.dbPath}:${session.id}`}>
                      <button
                        className="codex-session-main"
                        onClick={() => onOpen?.(session)}
                        title={session.title || session.id}
                        type="button"
                      >
                        <span>{session.title || "未命名会话"}</span>
                        <time>{formatSessionRelativeTime(session.updatedAtMs)}</time>
                      </button>
                      {onDelete ? (
                        <button
                          className="codex-session-delete"
                          onClick={(event) => {
                            event.stopPropagation();
                            onDelete(session);
                          }}
                          title="删除会话"
                          type="button"
                        >
                          <Trash2 className="h-4 w-4" />
                        </button>
                      ) : null}
                    </div>
                  ))}
                </div>
              </section>
            )) : <Empty text={emptyText} />}
          </div>
        )}
      </Panel>
    );
  };
  return (
    <>
      <div className="stack">
        <Panel title="会话管理" detail="历史会话修复、Codex 会话管理和 Claude 会话管理集中在这里。">
          <div className="ops-note">
            <ShieldCheck className="h-4 w-4" />
            <span>会话相关动作会优先在这里刷新和核对，避免在工具页和会话页之间来回跳。</span>
          </div>
        </Panel>
        <div className="session-management-wide-grid">
          <div className="session-history-card">
            <Panel title="历史会话修复" detail="用于修复切换供应商后 Codex 历史会话不可见或元数据不一致的问题。">
              <div className="ops-status-list">
                <StatusRow label="供应商同步" status={settings?.settings.providerSyncEnabled ? "running" : "disabled"} value={settings?.settings.providerSyncEnabled ? "已开启" : "未开启"} />
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
          </div>
          <div className="session-codex-card">
            {renderSessionBrowserPanel({
              title: "Codex 会话管理",
              ariaLabel: "Codex 本地会话项目列表",
              emptyText: "暂未读取到 Codex 本地会话。",
              data: localSessions,
              groups: codexSessionProjectGroups,
              onRefresh: () => void actions.refreshLocalSessions(),
              onOpen: (session) => void actions.loadCodexSessionContext(session),
              onDelete: (session) => void actions.deleteLocalSession(session),
            })}
          </div>
          <div className="session-claude-card">
            {renderSessionBrowserPanel({
              title: "Claude 会话管理",
              ariaLabel: "Claude 本地会话项目列表",
              emptyText: "暂未读取到 Claude 本地会话。",
              data: claudeSessions,
              groups: claudeSessionProjectGroups,
              onRefresh: () => void actions.refreshClaudeSessions(),
              onOpen: (session) => void actions.loadClaudeSessionContext(session),
              onDelete: (session) => void actions.deleteClaudeSession(session),
              sourceLabel: "Claude 会话源",
              sourceCountLabel: "候选源",
            })}
          </div>
        </div>
      </div>
      {sessionContextTarget ? createPortal(
        <div
          className="claude-session-context-overlay"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) closeSessionContext();
          }}
          role="presentation"
        >
          <section
            aria-labelledby="claude-session-context-title"
            aria-modal="true"
            className="claude-session-context-dialog"
            onKeyDown={(event) => {
              if (event.key === "Escape") closeSessionContext();
            }}
            ref={claudeContextDialogRef}
            role="dialog"
            tabIndex={-1}
          >
            <header className="claude-session-context-header">
              <div>
                <span>{showingCodexContext ? "Codex" : "Claude"} 会话上下文</span>
                <h2 id="claude-session-context-title">
                  {sessionContextPayload?.title || sessionContextTarget.title || "未命名会话"}
                </h2>
                <p>
                  {compactPath(sessionContextPayload?.cwd || sessionContextTarget.cwd || "未知项目")}
                  {sessionContextPayload ? ` · ${sessionContextPayload.totalMessages} 条可读消息` : ""}
                </p>
              </div>
              <button
                aria-label="关闭会话上下文"
                className="claude-session-context-close"
                onClick={closeSessionContext}
                title="关闭会话上下文"
                type="button"
              >
                <X className="h-5 w-5" />
              </button>
            </header>

            <div className="claude-session-context-meta">
              <span>{showingCodexContext ? "Codex rollout" : (claudeSessionContext?.sourceKind || claudeSessionContextTarget?.sourceKind)}</span>
              <strong>{compactPath(showingCodexContext ? (codexSessionContext?.rolloutPath || codexSessionContextTarget?.rolloutPath || "") : (claudeSessionContext?.sourcePath || claudeSessionContextTarget?.sourcePath || ""))}</strong>
            </div>

            <div className="claude-session-context-body">
              {sessionContextPayload?.hasMoreBefore ? (
                <Button
                  disabled={sessionContextLoading}
                  onClick={() => void loadEarlierSessionContext()}
                  size="sm"
                  variant="outline"
                >
                  <RefreshCw className={sessionContextLoading ? "h-4 w-4 animate-spin" : "h-4 w-4"} />
                  加载更早内容
                </Button>
              ) : null}

              {sessionContextError ? (
                <div className="claude-session-context-error">
                  <AlertTriangle className="h-4 w-4" />
                  <span>{sessionContextError}</span>
                  {!sessionContextPayload ? (
                    <Button onClick={() => void (showingCodexContext ? actions.loadCodexSessionContext(codexSessionContextTarget!) : actions.loadClaudeSessionContext(claudeSessionContextTarget!))} size="sm" variant="outline">
                      重试
                    </Button>
                  ) : null}
                </div>
              ) : null}

              {sessionContextLoading && !sessionContextPayload ? (
                <div className="claude-session-context-loading">
                  <RefreshCw className="h-5 w-5 animate-spin" />
                  <span>正在读取真实会话上下文...</span>
                </div>
              ) : null}

              {sessionContextPayload && !sessionContextPayload.messages.length && !sessionContextLoading ? (
                <Empty text="该会话没有可展示的文本上下文。" />
              ) : null}

              {sessionContextPayload?.messages.map((message) => (
                <article
                  className={`claude-session-context-message role-${message.role}`}
                  key={`${sessionContextPayload.sessionId}:${message.sequence}`}
                >
                  <header>
                    <strong>{({ user: "用户", assistant: showingCodexContext ? "Codex" : "Claude", tool: "工具", system: "系统", developer: "开发者" } as Record<string, string>)[message.role] || message.role}</strong>
                    <span>#{message.sequence}</span>
                    {("timestampMs" in message && message.timestampMs) ? <time>{new Date(message.timestampMs).toLocaleString("zh-CN", { hour12: false })}</time> : ("timestamp" in message && message.timestamp ? <time>{message.timestamp}</time> : null)}
                  </header>
                  <pre>{message.text}</pre>
                </article>
              ))}
            </div>
          </section>
        </div>,
        document.body,
      ) : null}
    </>
  );
});

export const PluginListItem = memo(function PluginListItem({
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

export function PluginHubScreen({
  actions,
  devMode,
  hub,
  preview,
  orgPlugin,
  marketplace,
}: {
  actions: AppActions;
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
  const installButtonLabel = selected ? pluginInstallButtonLabel(selected.installKind) : "安装";
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

export function PromptOptimizerCard({ actions }: { actions: AppActions }) {
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

export function MaintenanceToolsPanel({
  actions,
  claudeDesktop,
  overview,
  settings,
  watcher,
}: {
  actions: AppActions;
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

export function LogsScreen({ actions, logs }: { actions: AppActions; logs: LogsResult | null }) {
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

export const MaintenanceScreen = memo(function MaintenanceScreen({
  actions,
  claudeDesktop,
  overview,
  settings,
  watcher,
}: {
  actions: AppActions;
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
});

export const SettingsScreen = memo(function SettingsScreen({
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
  actions: AppActions;
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
    ["电脑操作守护", "computerUseGuardEnabled"],
    ["插件入口解锁", "codexAppPluginEntryUnlock"],
    ["插件市场解锁", "codexAppPluginMarketplaceUnlock"],
    ["特殊插件强制安装", "codexAppForcePluginInstall"],
    ["模型白名单解锁", "codexAppModelWhitelistUnlock"],
    ["会话删除", "codexAppSessionDelete"],
    ["Markdown 导出", "codexAppMarkdownExport"],
    ["会话项目移动", "codexAppProjectMove"],
    ["对话时间线", "codexAppConversationTimeline"],
    ["对话阅读视图", "codexAppConversationView"],
    ["切换对话保留位置", "codexAppThreadScrollRestore"],
    ["Zed 远程打开", "codexAppZedRemoteOpen"],
    ["Zed 项目记录", "zedRemoteProjectRegistryEnabled"],
    ["同步 Zed 设置", "zedRemoteSyncToZedSettings"],
    ["上游工作树创建", "codexAppUpstreamWorktreeCreate"],
    ["原生菜单栏位置", "codexAppNativeMenuPlacement"],
    ["Claude 中文覆盖", "claudeAppChineseOverlayEnabled"],
    ["Fast 按钮", "codexAppServiceTierControls"],
    ["图片覆盖", "codexAppImageOverlayEnabled"],
    ["Codex 目标", "codexGoalsEnabled"],
    ["盘古记忆", "memoryAssistEnabled"],
    ["盘古记忆 DOM 标识", "memoryAssistInjectEnabled"],
    ["自动学习", "memoryAssistAutoSuggestEnabled"],
    ["记忆 LLM 摘要", "memoryAssistLlmSummaryEnabled"],
    ["记忆 MCP 共享", "memoryAssistMcpEnabled"],
    ["CLI 包装器", "cliWrapperEnabled"],
  ] as const;
  return (
    <div className="stack">
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
        <Panel title="CLI 命令包装器" detail="命令行包装器用于把本地 Codex CLI 请求接入当前配置。">
          <div className="ops-toggle-line">
            <span>启用 CLI 命令包装器</span>
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
            <InfoRow label="生效方式" value="保存后重建 Codex CLI 命令包装器" />
            <InfoRow label="依赖" value="需要本机可执行 Codex CLI" />
          </div>
          <Button disabled={!s} onClick={() => void saveDraft()} variant="outline">保存 CLI 命令包装器</Button>
        </Panel>
      </div>
      </div>
      <LogsScreen actions={actions} logs={logs} />
    </div>
  );
});

export const AboutScreen = memo(function AboutScreen({
  actions,
  claudeDesktop,
  overview,
  updateInfo,
}: {
  actions: AppActions;
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
        <Panel title="联系我" detail="官方 QQ 群一键添加与合作代理微信。">
          <div className="contact-card">
            <div className="contact-line">
              <span className="contact-label">官方QQ群：</span>
              <span className="contact-group-number">10061615</span>
              <button className="contact-link" type="button" onClick={() => void actions.openExternalUrl(CONTACT_QQ_GROUP_PRIMARY_URL)}>一键添加</button>
              <span className="contact-group-number">1076215359</span>
              <button className="contact-link" type="button" onClick={() => void actions.openExternalUrl(CONTACT_QQ_GROUP_SECONDARY_URL)}>一键添加</button>
            </div>
            <div className="contact-wechat">
              <div>
                <strong>合作代理请联系微信</strong>
                <p>扫码添加微信，备注合作代理。</p>
              </div>
              <img className="contact-qr" src={contactWechatQr} alt="合作代理微信二维码" />
            </div>
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
            <Button disabled={!release || !updateInfo?.assetUrl} onClick={() => void actions.performUpdate(release)} variant="outline">
              <Download className="h-4 w-4" />
              下载并运行安装包
            </Button>
          </div>
        </Panel>
      </div>
    </div>
  );
});
