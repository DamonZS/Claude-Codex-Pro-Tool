import { type Dispatch, type DragEvent, type SetStateAction, memo, useEffect, useMemo, useState } from "react";
import {
  Activity,
  AlertTriangle,
  Archive,
  ArchiveRestore,
  CheckCircle2,
  Copy,
  Download,
  ExternalLink,
  FileCode2,
  FileDown,
  FileUp,
  GripVertical,
  KeyRound,
  Languages,
  MessageCircle,
  Network,
  Pencil,
  PencilRuler,
  Pin,
  Plus,
  Power,
  RefreshCw,
  Save,
  ShieldCheck,
  Trash2,
  Wrench,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { MemoryActivityWave } from "@/components/MemoryActivityWave";
import {
  AGGREGATE_STRATEGIES,
  CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_LOCAL_SOURCE,
  CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_NAME,
  CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_SOURCE,
  CODEX_THIRD_PARTY_PLUGIN_MARKETPLACE_NAME,
  CODEX_THIRD_PARTY_PLUGIN_REPOSITORY_URL,
  SUPPLIER_DRAG_MIME_TYPE,
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
  redactSupplierAuth,
  supplierIdFromName,
  supplierProfileHasApiKey,
  supplierProfileIsCcswitch,
  supplierProtocolLabel,
  supplierRelayModeLabel,
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
import type {
  BackendSettings,
  ClaudeChineseWindowResult,
  ClaudeContextEntriesResult,
  ClaudeDesktopDevModeStatusResult,
  ClaudeDesktopMarketplaceStatusResult,
  ClaudeDesktopOrgPluginStatusResult,
  ClaudeDesktopProviderApplyResult,
  ClaudeDesktopProviderPreviewResult,
  ClaudeDesktopResult,
  ClaudeZhPatchResult,
  CodexPluginMarketplaceStatusResult,
  ContextEntries,
  ContextEntriesResult,
  ContextEntry,
  ContextKind,
  LiveContextEntriesResult,
  LocalSessionsResult,
  LogsResult,
  MemoryExportResult,
  MemoryItem,
  MemoryItemsResult,
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
  UpdateResult,
  WatcherResult,
} from "@/types";

export function OverviewScreen({
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
  actions: AppActions;
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
            <StatusRow label="运行状态" status={memoryRuntimeStatus} value={memoryRuntimeValue} />
            <StatusRow label="Codex 注入" status={memoryInjectStatus} value={memoryInjectValue} />
            <StatusRow label="对话监控" status={memoryMonitorStatus} value={memoryMonitorValue} />
          </div>
          <div className="ops-note">
            <Activity className="h-4 w-4" />
            <span>对话监控</span>
            <MemoryActivityWave active={memoryMonitorActive} />
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

// Phase 2 tiering UI: a decay strength bar (exempt items read "常驻" and show
// full), plus an archive/restore action. Shared by the overview detail list and
// the memory management screen so both surfaces stay consistent.
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

export function SupplierScreen({
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
  actions: AppActions;
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
});

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
export const ContextManagerPanel = memo(function ContextManagerPanel({
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
  actions: AppActions;
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
  const sourceEntries = useMemo(
    () => (isCodex ? mergeContextEntries(entries, liveEntries) : entries),
    [isCodex, entries, liveEntries],
  );
  const currentEntries = useMemo(() => contextEntriesByKind(sourceEntries, tab), [sourceEntries, tab]);
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
});

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

export const SessionManagementScreen = memo(function SessionManagementScreen({
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
  actions: AppActions;
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
        <LogsScreen actions={actions} logs={logs} />
      </div>
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
});
