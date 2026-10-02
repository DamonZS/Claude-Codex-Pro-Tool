import {
  type CSSProperties,
  type Dispatch,
  type KeyboardEvent as ReactKeyboardEvent,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
  type SetStateAction,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import {
  Activity,
  ArrowLeft,
  BarChart3,
  Copy,
  Download,
  Edit,
  Eye,
  EyeOff,
  GripVertical,
  KeyRound,
  Network,
  Play,
  Plus,
  RefreshCw,
  Save,
  ShieldCheck,
  Trash2,
  Wrench,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { AGGREGATE_STRATEGIES, SUPPLIER_PRESETS } from "@/constants";
import type { AppActions } from "@/lib/actions";
import { Empty, InfoRow, Panel, ToggleSwitch } from "@/components/ui/ops";
import { statusFailed, statusOk } from "@/lib/helpers";
import {
  aggregateStrategyLabel,
  createAggregateSupplierProfile,
  createSupplierProfile,
  normalizeSupplierProfile,
  redactSupplierConfig,
  supplierIdFromName,
  supplierProfileCanActivate,
  supplierProfileIsCodexOfficialLogin,
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
import type {
  BackendSettings,
  ClaudeDesktopDevModeStatusResult,
  ClaudeDesktopProviderApplyResult,
  ClaudeDesktopProviderPreviewResult,
  CredentialEnvironmentResult,
  RelayProfile,
  RelayProfileModelsResult,
  SettingsResult,
  Status,
  SupplierPreset,
  SupplierSaveResult,
  SupplierTargetApp,
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
  const [activeIndex, setActiveIndex] = useState(-1);
  const [position, setPosition] = useState<CSSProperties>({});
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const menuRef = useRef<HTMLDivElement | null>(null);
  const menuId = useId();
  const valueAvailable = !showAvailabilityWarning || !value || options.includes(value);

  const openMenu = (direction: 1 | -1 = 1) => {
    const selectedIndex = options.indexOf(value);
    setActiveIndex(selectedIndex >= 0 ? selectedIndex : direction > 0 ? (options.length ? 0 : -1) : options.length - 1);
    setOpen(true);
  };

  const closeMenu = (restoreFocus = false) => {
    setOpen(false);
    if (restoreFocus) requestAnimationFrame(() => triggerRef.current?.focus());
  };

  const selectOption = (option: string) => {
    onChange(option);
    closeMenu(true);
  };

  const handleTriggerKeyDown = (event: ReactKeyboardEvent<HTMLButtonElement>) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const direction = event.key === "ArrowDown" ? 1 : -1;
      if (!open) {
        openMenu(direction);
        return;
      }
      setActiveIndex((index) => {
        if (!options.length) return -1;
        const start = index < 0 ? (direction > 0 ? -1 : 0) : index;
        return (start + direction + options.length) % options.length;
      });
      return;
    }
    if (event.key === "Home" && open) {
      event.preventDefault();
      setActiveIndex(options.length ? 0 : -1);
      return;
    }
    if (event.key === "End" && open) {
      event.preventDefault();
      setActiveIndex(options.length - 1);
      return;
    }
    if ((event.key === "Enter" || event.key === " ") && open) {
      event.preventDefault();
      const option = options[activeIndex];
      if (option) selectOption(option);
      return;
    }
    if (event.key === "Escape" && open) {
      event.preventDefault();
      closeMenu(true);
    }
  };

  useEffect(() => {
    if (!open) return;
    const updatePosition = () => {
      const trigger = triggerRef.current;
      if (!trigger) return;
      const anchor = iconOnly ? trigger.closest<HTMLElement>(".supplier-model-input-dropdown") ?? trigger : trigger;
      const rect = anchor.getBoundingClientRect();
      const viewportWidth = window.innerWidth;
      const viewportHeight = window.innerHeight;
      const gap = 8;
      const spaceBelow = Math.max(0, viewportHeight - rect.bottom - gap);
      const spaceAbove = Math.max(0, rect.top - gap);
      const opensUp = spaceBelow < 180 && spaceAbove > spaceBelow;
      const availableSpace = opensUp ? spaceAbove : spaceBelow;
      const width = Math.min(Math.max(rect.width, 280), Math.max(160, viewportWidth - 16));
      const left = Math.min(Math.max(8, rect.left), Math.max(8, viewportWidth - width - 8));
      setPosition({
        left,
        width,
        maxHeight: Math.max(44, Math.min(320, availableSpace)),
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
      if (event.key === "Escape" && !event.defaultPrevented) {
        event.preventDefault();
        closeMenu(true);
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
  }, [iconOnly, open]);

  useEffect(() => {
    if (!open || activeIndex < 0) return;
    menuRef.current
      ?.querySelector<HTMLElement>(`[data-option-index="${activeIndex}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [activeIndex, open]);

  const menu = open ? createPortal(
    <div aria-label={triggerLabel || placeholder} className="supplier-model-dropdown-menu" id={menuId} ref={menuRef} style={{ ...position, position: "fixed" }} role="listbox">
      {!valueAvailable && value ? <div className="supplier-model-dropdown-warning">当前配置不可用：{value}</div> : null}
      {options.length ? options.map((option, index) => (
        <button
          aria-selected={option === value}
          className={`${option === value ? "selected" : ""}${index === activeIndex ? " active" : ""}`.trim()}
          data-option-index={index}
          id={`${menuId}-option-${index}`}
          key={option}
          onClick={() => selectOption(option)}
          onMouseDown={(event) => event.preventDefault()}
          role="option"
          tabIndex={-1}
          title={option}
          type="button"
        >{option}</button>
      )) : <div className="supplier-model-dropdown-empty">暂无可用模型</div>}
    </div>,
    document.body,
  ) : null;

  return (
    <div className={`supplier-model-dropdown ${compact ? "compact" : ""} ${iconOnly ? "icon-only" : ""}`}>
      <button
        aria-activedescendant={open && activeIndex >= 0 ? `${menuId}-option-${activeIndex}` : undefined}
        aria-controls={menuId}
        aria-label={triggerLabel || placeholder}
        aria-expanded={open}
        aria-haspopup="listbox"
        className={`supplier-model-dropdown-trigger ${!valueAvailable ? "unavailable" : ""}`}
        onClick={() => open ? closeMenu() : openMenu()}
        onKeyDown={handleTriggerKeyDown}
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

function credentialEnvironmentScopeLabel(scope: string): string {
  switch (scope) {
    case "windows-user-environment":
      return "当前 CCP 进程和 Windows 当前用户环境";
    case "macos-launchd-user-session":
      return "当前 CCP 进程和 macOS launchd 用户会话";
    case "linux-systemd-user-manager":
      return "当前 CCP 进程和 Linux systemd 用户会话";
    default:
      return "当前 CCP 进程和可管理的用户会话环境";
  }
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
  focusProfileId,
  onClaudeDesktopProviderDraftChange,
}: {
  actions: AppActions;
  settings: SettingsResult | null;
  claudeDesktopDevMode: ClaudeDesktopDevModeStatusResult | null;
  claudeDesktopProviderPreview: ClaudeDesktopProviderPreviewResult | null;
  claudeDesktopProviderApply: ClaudeDesktopProviderApplyResult | null;
  credentialEnvironment: CredentialEnvironmentResult | null;
  focusProfileId?: string | null;
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
  const [supplierRouteToggleBusy, setSupplierRouteToggleBusy] = useState(false);
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
  const lastFocusedProfileIdRef = useRef<string | null>(null);
  const supplierRouteToggleInFlightRef = useRef(false);
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
    return sourceProfiles.some((profile) => supplierTargetForProfile(profile) === targetApp && !!profile.routeEnabled);
  };
  const withSupplierRoutingState = (profile: RelayProfile, targetApp: SupplierTargetApp, enabled: boolean) => {
    const claudeDesktopMode = targetApp === "codex" ? "" : enabled ? "proxy" : "direct";
    let configContents = profile.configContents ?? "";
    if (targetApp !== "codex") {
      let config: Record<string, unknown> = {};
      try {
        const parsed: unknown = JSON.parse(configContents);
        if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
          config = { ...parsed as Record<string, unknown> };
        }
      } catch {
        // Invalid imported JSON is replaced with the current form state below.
      }
      const existingMeta = config.meta;
      const meta = existingMeta && typeof existingMeta === "object" && !Array.isArray(existingMeta)
        ? { ...existingMeta as Record<string, unknown> }
        : {};
      delete meta.claude_desktop_mode;
      meta.claudeDesktopMode = claudeDesktopMode;
      config.meta = meta;
      configContents = `${JSON.stringify(config, null, 2)}\n`;
    }
    return normalizeSupplierProfile({
      ...profile,
      targetApp,
      configContents,
      routeEnabled: enabled,
      claudeDesktopMode,
      routeMode: targetApp === "codex"
        ? (enabled ? "Codex Proxy" : "Codex Direct")
        : (enabled ? "Claude Desktop Proxy" : "Claude Desktop Direct"),
    });
  };
  const withActiveSupplierId = (current: BackendSettings, targetApp: SupplierTargetApp, profileId: string): BackendSettings => {
    if (targetApp === "claude") return { ...current, activeClaudeRelayId: profileId };
    if (targetApp === "claude-desktop") return { ...current, activeClaudeDesktopRelayId: profileId };
    return { ...current, activeRelayId: profileId };
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
    if (!statusOk(result.status)) {
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
    setDraft(withSupplierRoutingState(profile, targetApp, !!profile.routeEnabled));
  };
  useEffect(() => {
    if (!focusProfileId || lastFocusedProfileIdRef.current === focusProfileId) return;
    const profile = profiles.find((item) => item.id === focusProfileId);
    if (!profile) return;
    lastFocusedProfileIdRef.current = focusProfileId;
    setSupplierTargetFilter(supplierTargetForProfile(profile));
    openProfileEditor(profile);
  }, [focusProfileId, profileIdsKey]);
  const createProfile = () => {
    if (!appSettings) return;
    supplierModelFetchRequestRef.current += 1;
    setModelFetch(null);
    setShowSupplierApiKey(false);
    setEditingId(null);
    const targetApp = supplierTargetFilter;
    const profile = withSupplierRoutingState({
      ...createSupplierProfile(appSettings),
      id: uniqueSupplierProfileId(appSettings.relayProfiles, "provider"),
      name: "供应商",
      targetApp,
    }, targetApp, supplierRoutingEnabledForTarget(targetApp));
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
      ...withSupplierRoutingState(profile, targetApp, !!profile.routeEnabled),
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
  // Keep the name field editable without regenerating the profile on every
  // keystroke. Generated config normalization can otherwise race the input
  // value and make imported supplier names appear immutable.
  const updateSupplierName = (name: string) => {
    setDraft((current) => current ? { ...current, name } : current);
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
    const scopeLabel = credentialEnvironmentScopeLabel(credentialEnvironment.userScope);
    if (!window.confirm(`确认从${scopeLabel}删除环境变量「${variableName}」？不会删除 auth.json 凭据、修改 CODEX_HOME 或系统级环境变量。`)) return;
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
      contextWindow: "1000000",
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
    const saveName = draft.name.trim() || normalizedId;
    const routedDraft = withSupplierRoutingState({ ...draft, id: normalizedId, name: saveName }, targetApp, !!draft.routeEnabled);
    const normalized = supplierProfileIsCcswitch(routedDraft)
      ? withSupplierPreservedImportedFiles(routedDraft)
      : normalizeSupplierProfile(withSupplierGeneratedFiles(routedDraft));
    const isCodexOfficialLogin = supplierProfileIsCodexOfficialLogin(normalized);
    if (!normalized.name.trim() || (!aggregateDraft && !isCodexOfficialLogin && !normalized.baseUrl.trim())) {
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
    if (shouldApplySupplier && !supplierProfileCanActivate(normalized)) {
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
          setSupplierTargetFilter(supplierTargetForProfile(savedProfile));
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
    await saveDraft({ applySupplier: true });
  };
  const removeProfile = async (profile: RelayProfile) => {
    if (!appSettings) {
      window.alert("设置尚未加载，无法删除供应商。");
      return;
    }
    if (!window.confirm(`确认删除供应商「${profile.name || profile.id}」？`)) return;
    const deletingActiveCodex = supplierTargetForProfile(profile) === "codex"
      && appSettings.activeRelayId === profile.id;
    if (deletingActiveCodex) {
      // 删除活动 Codex profile 前先清理 live config/auth，避免列表已删但
      // 独立启动 Codex 仍继续使用已删除的中转供应商。
      await actions.clearRelayMode();
    }
    const nextProfiles = profiles
      .filter((item) => item.id !== profile.id)
      .map((item) => item.aggregateEnabled ? { ...item, aggregateMembers: (item.aggregateMembers ?? []).filter((id) => id !== profile.id) } : item);
    const nextForTarget = (targetApp: SupplierTargetApp, currentId: string) => {
      if (currentId !== profile.id) return currentId;
      return nextProfiles.find((item) => supplierTargetForProfile(item) === targetApp)?.id ?? "";
    };
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
  const toggleVisibleSupplierRouting = async (enabled: boolean) => {
    if (!appSettings || !routableSupplierProfiles.length || supplierRouteToggleInFlightRef.current) return;
    supplierRouteToggleInFlightRef.current = true;
    setSupplierRouteToggleBusy(true);
    try {
      const visibleIds = new Set(routableSupplierProfiles.map((profile) => profile.id));
      const nextProfiles = appSettings.relayProfiles.map((profile) => {
        if (!visibleIds.has(profile.id)) return profile;
        return withSupplierRoutingState(profile, supplierRouteGroup, enabled);
      });
      const nextSettings = { ...appSettings, relayProfiles: nextProfiles };
      actions.showNotice({ title: "供应商路由", message: enabled ? `正在开启 ${supplierRouteGroupLabel} 供应商路由...` : `正在关闭 ${supplierRouteGroupLabel} 供应商路由...`, status: "running" });
      const activeProfileId = activeSupplierIdForTarget(supplierRouteGroup);
      const isDisablingActiveRoute = !enabled && visibleIds.has(activeProfileId);
      if (isDisablingActiveRoute && supplierRouteGroup === "codex") {
        const switched = await actions.switchSupplierProfile("codex", activeProfileId, nextSettings);
        if (!switched || !statusOk(switched.status)) return;
        actions.showNotice({ title: "供应商路由", message: "已关闭 Codex 供应商路由，运行中的代理配置已撤销。", status: "ok" });
        return;
      }
      if (isDisablingActiveRoute && supplierRouteGroup === "claude-desktop") {
        const restored = await (actions.restoreClaudeDesktopProviderOfficial as unknown as (skipConfirm?: boolean) => Promise<{ status?: Status } | null>)(true);
        if (!restored || !statusOk(restored.status)) return;
        const saved = await saveSupplierSettings({ ...appSettings, relayProfiles: nextProfiles });
        if (!saved) return;
        actions.showNotice({ title: "供应商路由", message: "已关闭 Claude Desktop 供应商路由，运行中的代理配置已撤销。", status: "ok" });
        return;
      }
      const saved = await saveSupplierSettings(nextSettings);
      if (saved) {
        actions.showNotice({ title: "供应商路由", message: enabled ? `已开启 ${supplierRouteGroupLabel} 供应商路由。` : `已关闭 ${supplierRouteGroupLabel} 供应商路由。`, status: "ok" });
      }
    } finally {
      supplierRouteToggleInFlightRef.current = false;
      setSupplierRouteToggleBusy(false);
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
  const supplierRouteGroup = supplierTargetFilter;
  const supplierRouteGroupLabel = supplierRouteGroup === "claude-desktop"
    ? "Claude Desktop"
    : supplierRouteGroup === "claude"
      ? "Claude"
      : "Codex";
  const routableSupplierProfiles = useMemo(
    () => profiles.filter((profile) => supplierTargetForProfile(profile) === supplierRouteGroup),
    [profiles, supplierRouteGroup],
  );
  const supplierRouteSwitchEnabled = routableSupplierProfiles.some((profile) => !!profile.routeEnabled);
  const supplierRouteSwitchDisabled = supplierRouteToggleBusy || !appSettings || !routableSupplierProfiles.length;
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
  const importFromCcswitch = async () => {
    if (!appSettings) return;
    setImportOpen(false);
    const result = await actions.importCcswitchCodexProviders();
    if (!result || !statusOk(result.status)) return;
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
    const saved = await saveSupplierSettings({ ...appSettings, relayProfiles: nextProfiles });
    if (!saved) return;
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
      ? `${aggregateStrategyLabel(profile.aggregateStrategy)} / ${profile.aggregateMembers?.length ?? 0} 个成员`
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
        <button aria-label={"拖拽排序"} className="supplier-drag-handle" disabled={options.overlay} onPointerDown={options.overlay ? undefined : (event) => beginSupplierPointerDrag(event, profile.id)} title={"按住拖拽排序"} type="button">
          <GripVertical className="h-4 w-4" focusable="false" />
        </button>
        <div className="supplier-avatar">{aggregate ? "聚" : (profile.name || profile.id || "P").slice(0, 1).toUpperCase()}</div>
        <div className="supplier-card-main">
          <div className="supplier-title-line">
            <strong>{profile.name || profile.id}</strong>
            {aggregate ? <span className="supplier-badge">聚合</span> : null}
            {imported ? <span className="supplier-badge">cc-switch</span> : null}
          </div>
          {aggregate ? <span className="supplier-card-subtitle">{summary}</span> : null}
          <button className="supplier-url-link" disabled type="button">{displayUrl}</button>
        </div>
        <div className="supplier-card-actions">
          <button className={`supplier-card-action-button supplier-card-use-button ${selected ? "current" : ""}`} disabled={selected || aggregate || appSettings?.relayProfilesEnabled === false || options.overlay} onClick={() => void actions.switchSupplierProfile(targetApp, profile.id)} type="button">
            <Play className="h-4 w-4" />
            {selected ? "使用中" : "使用"}
          </button>
          <button className="supplier-card-action-button" disabled={options.overlay} onClick={() => openProfileEditor(profile)} title="编辑" type="button"><Edit className="h-4 w-4" /></button>
          <button className="supplier-card-action-button" disabled={options.overlay} onClick={() => duplicateProfile(profile)} title="复制" type="button"><Copy className="h-4 w-4" /></button>
          <button className="supplier-card-action-button" disabled title="检测连通" type="button"><Activity className="h-4 w-4" /></button>
          <button className="supplier-card-action-button" disabled title="用量配置" type="button"><BarChart3 className="h-4 w-4" /></button>
          <button className="supplier-card-action-button" disabled={options.overlay} onClick={() => void removeProfile(profile)} title="删除供应商" type="button"><Trash2 className="h-4 w-4" /></button>
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
              <label className="ops-form-field"><span>名称</span><input onChange={(event) => updateSupplierName(event.currentTarget.value)} value={generated.name} /></label>
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
    const canSwitch = !generated.aggregateEnabled;
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
          <label className="ops-form-field"><span>供应商名称</span><input onBlur={(event) => updateNewDraftIdFromName(event.currentTarget.value)} onChange={(event) => updateSupplierName(event.currentTarget.value)} value={draft.name} /></label>
          <label className="ops-form-field"><span>备注</span><input onChange={(event) => updateDraft({ notes: event.currentTarget.value })} placeholder="例如：公司专用账号" value={generated.notes || ""} /></label>
          <label className="ops-form-field"><span>官网链接</span><input onChange={(event) => updateDraft({ websiteUrl: event.currentTarget.value })} placeholder="https://example.com" value={generated.websiteUrl || ""} /></label>
          <label className="ops-form-field"><span>API Key</span><div className="supplier-secret-input"><input onChange={(event) => updateDraft({ apiKey: event.currentTarget.value, apiKeyExplicit: true })} type={showSupplierApiKey ? "text" : "password"} value={generated.apiKey} /><button aria-label={showSupplierApiKey ? "隐藏密钥" : "显示密钥"} onClick={() => setShowSupplierApiKey((value) => !value)} title={showSupplierApiKey ? "隐藏密钥" : "显示密钥"} type="button">{showSupplierApiKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}</button></div></label>
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
                      <div className="supplier-model-input-dropdown">
                        <input
                          aria-label={`${row.label} 实际请求模型`}
                          onChange={(event) => updateSupplierModelMapping(row.role, "requestModel", event.currentTarget.value)}
                          placeholder="例如: claude-sonnet-4-6"
                          value={row.requestModel || ""}
                        />
                        <SupplierModelDropdown
                          compact
                          iconOnly
                          onChange={(value) => updateSupplierModelMapping(row.role, "requestModel", value)}
                          options={supplierModelOptions}
                          placeholder="选择已获取模型"
                          showAvailabilityWarning={false}
                          triggerLabel="选择已获取模型"
                          value={row.requestModel || ""}
                        />
                      </div>
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

  const credentialEnvironmentExternalSource = Boolean(
    credentialEnvironment?.externalSourceLikely,
  );
  const credentialEnvironmentScopeUnavailable = Boolean(
    credentialEnvironment?.present && !credentialEnvironment.userScopeAvailable,
  );


  return (
    <div className="supplier-list-shell">
      {credentialEnvironment && (credentialEnvironment.present || credentialEnvironment.restartRequired) ? <div className="supplier-env-card"><ShieldCheck className="h-5 w-5" /><div><strong>{credentialEnvironment.conflict ? "检测到凭据环境变量冲突" : "检测到凭据环境变量"}</strong><p>{credentialEnvironment.conflict ? `${credentialEnvironment.variableName} 与当前 Codex 供应商凭据不一致，可能覆盖 config.toml / auth.json 并导致 401；不会清理 CODEX_HOME。` : credentialEnvironment.present ? `${credentialEnvironment.variableName} 已存在，当前未发现与活动供应商的值冲突。` : `${credentialEnvironment.variableName} 已从 CCP 可管理的作用域清理。`}{credentialEnvironmentScopeUnavailable ? " 用户会话环境暂不可访问，CCP 未执行扩大范围的清理。" : ""}{credentialEnvironmentExternalSource ? " 该值来自 CCP 外部启动环境，需在原设置来源中清理。" : ""}{credentialEnvironment.restartRequired ? " 请完全退出并重新启动 Codex。" : ""}</p><span className="supplier-env-chip">{credentialEnvironment.variableName} {credentialEnvironment.userPresent ? "用户会话" : credentialEnvironment.systemPresent ? "系统环境" : credentialEnvironment.processPresent ? "当前进程" : "已清理"}</span></div><div className="supplier-env-actions"><Button disabled={!credentialEnvironment.canClearUser || credentialEnvironmentBusy} onClick={() => void clearCredentialEnvironment()} size="sm" variant="outline"><Trash2 className="h-4 w-4" />删除</Button><Button disabled={credentialEnvironmentBusy} onClick={() => void refreshCredentialEnvironment()} size="sm" variant="outline"><RefreshCw className={`h-4 w-4 ${credentialEnvironmentBusy ? "spin" : ""}`} />{credentialEnvironmentBusy ? "检测中" : "检测"}</Button></div></div> : null}
      <div className="supplier-control-row"><div className="supplier-route-master-toggle"><Network className="h-4 w-4" /><span>开启路由</span><ToggleSwitch checked={supplierRouteSwitchEnabled} disabled={supplierRouteSwitchDisabled} onChange={(value) => void toggleVisibleSupplierRouting(value)} /></div><div className="supplier-toolbar right"><div className="supplier-target-filter" aria-label="供应商目标应用过滤"><button className={supplierTargetFilter === "codex" ? "active" : ""} onClick={() => setSupplierTargetFilter("codex")} type="button">Codex</button><button className={supplierTargetFilter === "claude" ? "active" : ""} onClick={() => setSupplierTargetFilter("claude")} type="button">Claude</button><button className={supplierTargetFilter === "claude-desktop" ? "active" : ""} onClick={() => setSupplierTargetFilter("claude-desktop")} type="button">Claude Desktop</button></div><Button disabled={!appSettings} onClick={createProfile}><Plus className="h-4 w-4" />添加供应商</Button><Button disabled={!appSettings} onClick={createAggregateProfile} variant="outline"><Plus className="h-4 w-4" />添加聚合供应商</Button><div className="supplier-import-wrap"><Button onClick={() => setImportOpen((value) => !value)} variant="outline"><Download className="h-4 w-4" />从第三方导入</Button>{importOpen ? <div className="supplier-drop-popover"><button onClick={() => void importFromCcswitch()} type="button"><strong>ccswitch</strong><span>发现并导入 Codex / Claude / Claude Desktop 配置</span></button><button className={`supplier-menu-action ${supplierRefreshBusy ? "busy" : ""}`} disabled={supplierRefreshBusy} onClick={() => void refreshSupplierList()} type="button"><RefreshCw className={`h-4 w-4 ${supplierRefreshBusy ? "spin" : ""}`} />{supplierRefreshBusy ? "刷新中..." : "刷新列表"}</button></div> : null}</div></div></div>
      <div className="supplier-card-list">
        {filteredOrderedProfiles.length ? filteredOrderedProfiles.map((profile) => renderSupplierCard(profile)) : <Empty text="暂无供应商配置，点击“添加供应商”创建一个真实可切换的 Codex API 配置。" />}
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
