import { AGGREGATE_STRATEGIES } from "@/constants";
import type { BackendSettings, RelayProfile, SupplierPreset } from "@/types";

export const DEFAULT_SUPPLIER_CONTEXT_WINDOW = "1000000";

function normalizedContextWindow(value: unknown) {
  const digits = String(value ?? "").replace(/[^\d]/g, "");
  return digits && Number.parseInt(digits, 10) > 0 ? digits : DEFAULT_SUPPLIER_CONTEXT_WINDOW;
}

export function supplierIdFromName(value: string) {
  const id = value.trim().toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "");
  return id || "provider";
}

export function uniqueSupplierProfileId(profiles: RelayProfile[], base: string) {
  const root = supplierIdFromName(base);
  const existing = new Set(profiles.map((profile) => profile.id));
  if (!existing.has(root)) return root;
  for (let index = 2; index < 999; index += 1) {
    const candidate = `${root}-${index}`;
    if (!existing.has(candidate)) return candidate;
  }
  return `${root}-${Date.now().toString(36)}`;
}

/**
 * Provider id 是会话里记录的持久标识，与核心侧 `provider_id_for_base_url` 同一条规则：
 * 只取中转地址的 host[:port]。同一家中转换 key、或重建一条指向它的供应商条目，id 都不变，
 * 旧会话才不会指向一个已被删掉的 provider 表。
 */
export function supplierIdForBaseUrl(baseUrl: string) {
  const trimmed = String(baseUrl || "").trim();
  if (!trimmed) return "";
  const authority = trimmed
    .replace(/^[a-z][a-z0-9+.-]*:\/\//i, "")
    .split(/[/?#]/)[0]
    .split("@")
    .pop() || "";
  const slug = authority.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "");
  return slug ? `relay-${slug}` : "";
}

export function createSupplierProfile(settings: BackendSettings): RelayProfile {
  return normalizeSupplierProfile(withSupplierGeneratedFiles({
    id: uniqueSupplierProfileId(settings.relayProfiles, "provider"),
    name: "",
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

export function createAggregateSupplierProfile(settings: BackendSettings): RelayProfile {
  return normalizeSupplierProfile(withSupplierGeneratedFiles({
    ...createSupplierProfile(settings),
    id: uniqueSupplierProfileId(settings.relayProfiles, "aggregate"),
    name: `聚合供应商 ${settings.relayProfiles.filter((profile) => profile.aggregateEnabled).length + 1}`,
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

export function normalizeSupplierProfile(profile: RelayProfile): RelayProfile {
  const hasExplicitModelList = typeof profile.modelList === "string";
  const modelList = hasExplicitModelList ? profile.modelList : "";
  const apiKey = supplierProfileResolvedApiKey(profile);
  const storedBaseUrl = profile.baseUrl || "";
  const upstreamBaseUrl = profile.upstreamBaseUrl || "";
  // The local protocol proxy belongs in generated runtime config only. Keep
  // the provider record pointed at its real upstream endpoint when an older
  // route-enabled save left the proxy URL in baseUrl.
  const baseUrl = isLocalSupplierProxyUrl(storedBaseUrl)
    ? upstreamBaseUrl
    : storedBaseUrl || upstreamBaseUrl;
  const model = profile.model || profile.testModel || firstSupplierModel(modelList) || "gpt-5.5";
  const targetApp = profile.targetApp || "codex";
  const modelMappingEnabled = typeof profile.modelMappingEnabled === "boolean"
    ? profile.modelMappingEnabled
    : !!profile.modelMapping || !!profile.modelMappingJson;
  const routeEnabled = typeof profile.routeEnabled === "boolean"
    ? profile.routeEnabled
    : profile.claudeDesktopMode === "proxy" || /\bproxy\b/i.test(profile.routeMode || "");
  // 如果 modelMappingEnabled 显式为 false，清空映射字段，防止下次反向推断
  const normalizedModelMapping = modelMappingEnabled
    ? normalizeSupplierModelMappingFields(profile)
    : { modelMapping: "", modelMappingJson: "" };
  return {
    ...profile,
    id: supplierIdFromName(profile.id || profile.name),
    // Empty is intentional for a new profile; never replace it with the id.
    name: profile.name ?? "",
    model,
    testModel: profile.testModel || model,
    baseUrl,
    upstreamBaseUrl: upstreamBaseUrl || baseUrl,
    apiKey,
    protocol: profile.protocol || "responses",
    relayMode: profile.relayMode === "official" ? "official" : "pureApi",
    officialMixApiKey: false,
    configContents: profile.configContents ?? "",
    authContents: profile.authContents ?? "",
    modelList: hasExplicitModelList ? modelList : model,
    codexCatalogJson: profile.codexCatalogJson ?? "",
    contextWindow: targetApp === "codex"
      ? normalizedContextWindow(profile.contextWindow)
      : profile.contextWindow ?? "",
    autoCompactLimit: profile.autoCompactLimit ?? "",
    userAgent: profile.userAgent ?? "",
    notes: profile.notes ?? "",
    websiteUrl: profile.websiteUrl ?? "",
    authField: profile.authField ?? "",
    headerOverride: profile.headerOverride ?? "",
    bodyOverride: profile.bodyOverride ?? "",
    hideAiSignature: !!profile.hideAiSignature,
    teammatesMode: !!profile.teammatesMode,
    toolSearchEnabled: !!profile.toolSearchEnabled,
    maxThinkingEnabled: !!profile.maxThinkingEnabled,
    disableAutoUpdate: !!profile.disableAutoUpdate,
    importSource: profile.importSource ?? "",
    targetApp,
    apiFormat: profile.apiFormat ?? "",
    claudeDesktopMode: targetApp === "codex" ? "" : routeEnabled ? "proxy" : "direct",
    routeEnabled,
    routeMode: targetApp === "codex"
      ? (routeEnabled ? "Codex Proxy" : "Codex Direct")
      : (routeEnabled ? "Claude Desktop Proxy" : "Claude Desktop Direct"),
    modelMapping: normalizedModelMapping.modelMapping,
    modelMappingEnabled,
    modelMappingJson: normalizedModelMapping.modelMappingJson,
    aggregateEnabled: !!profile.aggregateEnabled,
    aggregateStrategy: profile.aggregateStrategy || (profile.aggregateEnabled ? "failover" : ""),
    aggregateMembers: Array.isArray(profile.aggregateMembers) ? profile.aggregateMembers : [],
  };
}



export type SupplierModelMappingRow = {
  role: "sonnet" | "opus" | "fable" | "haiku" | "subagent";
  label: string;
  routeId: string;
  displayName: string;
  requestModel: string;
  supports1m: boolean;
};

export type SupplierDirectModelRow = {
  model: string;
  supports1m: boolean;
};

export type SupplierCodexCatalogRow = {
  displayName: string;
  model: string;
  supports1m: boolean;
  maxContextWindow: string;
};

/** 「最大上下文」输入框的默认 K 值：未勾 1M 且留空时按 256K 落盘。 */
const CODEX_CATALOG_DEFAULT_LIMIT_K = 256;

/** 把「最大上下文」输入框的 K 值换算成落盘的字节数。 */
export function supplierCodexCatalogLimitTokens(maxContextWindow: string) {
  const kilo = Number.parseInt(String(maxContextWindow ?? "").replace(/[^\d]/g, ""), 10);
  return String((kilo > 0 ? kilo : CODEX_CATALOG_DEFAULT_LIMIT_K) * 1000);
}

function codexCatalogLimitK(value: unknown) {
  const tokens = Number.parseInt(String(value ?? "").replace(/[^\d]/g, ""), 10);
  return tokens > 0 ? String(Math.round(tokens / 1000)) : String(CODEX_CATALOG_DEFAULT_LIMIT_K);
}

export function supplierCodexCatalogRows(profile: RelayProfile): SupplierCodexCatalogRow[] {
  const isCodex = (profile.targetApp || "codex") === "codex";
  const raw = String(profile.codexCatalogJson ?? "").trim();
  if (raw) {
    try {
      const parsed = JSON.parse(raw) as unknown;
      if (Array.isArray(parsed)) {
        const seen = new Set<string>();
        return parsed.flatMap((item) => {
          if (!item || typeof item !== "object") return [];
          const value = item as Record<string, unknown>;
          const model = typeof value.model === "string" ? value.model.trim() : "";
          if (!model || seen.has(model)) return [];
          seen.add(model);
          const displayName = typeof value.displayName === "string"
            ? value.displayName.trim()
            : typeof value.display_name === "string"
              ? value.display_name.trim()
              : "";
          const legacyContextWindow = isCodex
            ? normalizedContextWindow(value.contextWindow ?? value.context_window)
            : String(value.contextWindow ?? value.context_window ?? "").replace(/[^\d]/g, "");
          const supports1m = isCodex
            ? value.supports1m !== false && value.supports_1m !== false && legacyContextWindow !== ""
            : false;
          const maxContextWindow = isCodex
            ? codexCatalogLimitK(value.maxContextWindow ?? value.max_context_window)
            : String(value.maxContextWindow ?? value.max_context_window ?? "").replace(/[^\d]/g, "");
          return [{ displayName, model, supports1m, maxContextWindow }];
        });
      }
    } catch {
      // Invalid legacy metadata falls back to the existing model list below.
    }
  }

  const fallbackModels = supplierDirectModelRows(profile.modelList).map((row) => row.model);
  const model = profile.model?.trim() || profile.testModel?.trim() || "";
  const models = fallbackModels.length ? fallbackModels : model ? [model] : [];
  const seen = new Set<string>();
  return models.flatMap((item) => {
    const normalized = item.trim();
    if (!normalized || seen.has(normalized)) return [];
    seen.add(normalized);
    return [{
      displayName: normalized,
      model: normalized,
      supports1m: isCodex,
      maxContextWindow: "",
    }];
  });
}

export function supplierCodexCatalogJson(rows: SupplierCodexCatalogRow[]) {
  const seen = new Set<string>();
  const normalized = rows.flatMap((row) => {
    const model = row.model.trim();
    if (!model || seen.has(model)) return [];
    seen.add(model);
    const displayName = row.displayName.trim();
    return [{
      model,
      ...(displayName ? { displayName } : {}),
      ...(row.supports1m
        ? { contextWindow: 1_000_000, maxContextWindow: 1_000_000 }
        : { supports1m: false, maxContextWindow: Number.parseInt(supplierCodexCatalogLimitTokens(row.maxContextWindow), 10) }),
    }];
  });
  return JSON.stringify(normalized, null, 2);
}

export function supplierCodexCatalogModelList(rows: SupplierCodexCatalogRow[]) {
  const serialized = supplierCodexCatalogJson(rows);
  try {
    const parsed = JSON.parse(serialized) as Array<{ model?: unknown }>;
    return parsed
      .map((row) => typeof row.model === "string" ? row.model : "")
      .filter(Boolean)
      .join("\n");
  } catch {
    return "";
  }
}

/**
 * Codex 目标实际生效的 `model_context_window`：勾了 1M 落 1000000，
 * 不勾按首行「最大上下文」的 K 值（留空 256K）；没有映射行时返回空串。
 */
export function supplierCodexEffectiveContextWindow(profile: RelayProfile) {
  if ((profile.targetApp || "codex") !== "codex") return "";
  const firstRow = supplierCodexCatalogRows(profile).find((row) => row.model.trim());
  if (!firstRow) return "";
  return firstRow.supports1m ? "1000000" : supplierCodexCatalogLimitTokens(firstRow.maxContextWindow);
}

export function supplierDirectModelRows(modelList: string): SupplierDirectModelRow[] {
  return String(modelList || "")
    .split(/\r?\n/)
    .map((line) => {
      const supports1m = /\s*\[1m\]\s*$/i.test(line);
      return {
        model: line.replace(/\s*\[1m\]\s*$/i, "").trim(),
        supports1m,
      };
    })
    .filter((row) => row.model);
}

export function supplierDirectModelList(rows: SupplierDirectModelRow[]) {
  return rows
    .map((row) => ({ model: row.model.trim(), supports1m: row.supports1m }))
    .filter((row) => row.model)
    .map((row) => `${row.model}${row.supports1m ? " [1M]" : ""}`)
    .join("\n");
}

/**
 * Claude Desktop rejects a whole gateway catalogue when one direct route is
 * malformed.  Keep this in sync with cc-switch's route whitelist.
 */
export function supplierDirectModelIsClaudeDesktopSafe(model: string) {
  const normalized = model.trim().toLowerCase();
  if (!normalized || normalized.includes("[1m]")) return false;
  const routeTail = normalized.startsWith("anthropic/claude-")
    ? normalized.slice("anthropic/claude-".length)
    : normalized.startsWith("claude-")
      ? normalized.slice("claude-".length)
      : "";
  return ["sonnet-", "opus-", "haiku-", "fable-"]
    .some((prefix) => routeTail.startsWith(prefix) && routeTail.length > prefix.length);
}

export const SUPPLIER_MODEL_MAPPING_DEFAULTS: SupplierModelMappingRow[] = [
  { role: "sonnet", label: "Sonnet", routeId: "claude-sonnet-4-6", displayName: "claude-opus-4-6", requestModel: "claude-opus-4-6", supports1m: true },
  { role: "opus", label: "Opus", routeId: "claude-opus-4-8", displayName: "claude-opus-4-8", requestModel: "claude-opus-4-8", supports1m: true },
  { role: "fable", label: "Fable", routeId: "claude-fable-5", displayName: "claude-Fable-5", requestModel: "claude-Fable-5", supports1m: true },
  { role: "haiku", label: "Haiku", routeId: "claude-haiku-4-5", displayName: "claude-haiku-4-5", requestModel: "claude-haiku-4-5", supports1m: true },
  { role: "subagent", label: "Subagent", routeId: "claude-subagent", displayName: "", requestModel: "", supports1m: true },
];

type SupplierModelMappingJsonEntry = Record<string, unknown>;

function supplierModelMappingDefaultForEntry(entry: SupplierModelMappingJsonEntry) {
  const role = typeof entry.role === "string" ? entry.role.trim().toLowerCase() : "";
  const label = typeof entry.label === "string" ? entry.label.trim().toLowerCase() : "";
  const routeId = typeof entry.routeId === "string" ? entry.routeId.trim() : "";
  return SUPPLIER_MODEL_MAPPING_DEFAULTS.find((row) =>
    row.role === role || row.label.toLowerCase() === label || row.routeId === routeId);
}

function isLocalSupplierProxyUrl(value: string) {
  return /^https?:\/\/(?:127\.0\.0\.1|localhost):\d+\/v1\/?$/i.test(value.trim());
}

function supplierModelMappingRowFromEntry(entry: SupplierModelMappingJsonEntry): SupplierModelMappingRow | null {
  const fallback = supplierModelMappingDefaultForEntry(entry);
  if (!fallback) return null;
  const requestModel = typeof entry.requestModel === "string"
    ? entry.requestModel.trim()
    : typeof entry.model === "string"
      ? entry.model.trim()
      : "";
  const displayName = typeof entry.displayName === "string"
    ? entry.displayName.trim()
    : typeof entry.labelOverride === "string"
      ? entry.labelOverride.trim()
      : "";
  return {
    ...fallback,
    label: typeof entry.label === "string" && entry.label.trim() ? entry.label.trim() : fallback.label,
    routeId: typeof entry.routeId === "string" && entry.routeId.trim() ? entry.routeId.trim() : fallback.routeId,
    displayName,
    requestModel: requestModel || displayName,
    supports1m: typeof entry.supports1m === "boolean"
      ? entry.supports1m
      : typeof entry.supports_1m === "boolean"
        ? entry.supports_1m
        : false,
  };
}

function supplierModelMappingRowsFromText(text: string): SupplierModelMappingRow[] {
  const rows: SupplierModelMappingRow[] = [];
  for (const rawLine of String(text || "").split(/\r?\n/)) {
    const supports1m = /\s*\[1m\]\s*$/i.test(rawLine);
    const line = rawLine.replace(/\s*\[1m\]\s*$/i, "").trim();
    const separator = line.indexOf(":");
    if (separator <= 0) continue;

    const left = line.slice(0, separator).trim();
    const right = line.slice(separator + 1).trim();
    const routeMatch = left.match(/\(([^)]+)\)/);
    const routeId = routeMatch?.[1]?.trim() || "";
    const label = left.replace(/\s*\([^)]+\)\s*$/, "").trim();
    const fallback = SUPPLIER_MODEL_MAPPING_DEFAULTS.find((row) =>
      row.role === label.toLowerCase() || row.label.toLowerCase() === label.toLowerCase() || row.routeId === routeId);
    if (!fallback) continue;

    const arrow = right.indexOf("->");
    const displayName = (arrow >= 0 ? right.slice(0, arrow) : right).trim();
    const requestModel = (arrow >= 0 ? right.slice(arrow + 2) : right).trim();
    if (!displayName && !requestModel) continue;
    rows.push({
      ...fallback,
      label: label || fallback.label,
      routeId: routeId || fallback.routeId,
      displayName: displayName || requestModel,
      requestModel: requestModel || displayName,
      supports1m,
    });
  }
  return rows;
}

function supplierModelMappingEntriesFromJson(raw: string): SupplierModelMappingJsonEntry[] | null {
  try {
    const parsed: unknown = JSON.parse(raw);
    return Array.isArray(parsed) && parsed.every((entry) => entry && typeof entry === "object" && !Array.isArray(entry))
      ? parsed as SupplierModelMappingJsonEntry[]
      : null;
  } catch {
    return null;
  }
}

function repairKnownClaudeHaikuMappingSplit(
  entries: SupplierModelMappingJsonEntry[],
  textRows: SupplierModelMappingRow[],
) {
  const explicitHaiku = textRows.find((row) =>
    row.role === "haiku"
    && row.routeId === "claude-haiku-4-5"
    && row.requestModel === "claude-opus-4-7");
  if (!explicitHaiku) return entries;

  const index = entries.findIndex((entry) => {
    const row = supplierModelMappingRowFromEntry(entry);
    return row?.role === "haiku"
      && row.routeId === "claude-haiku-4-5"
      && row.requestModel === "claude-haiku-4-5"
      && (!row.displayName || row.displayName === "claude-haiku-4-5");
  });
  if (index < 0) return entries;

  const repaired = entries.map((entry) => ({ ...entry }));
  repaired[index] = {
    ...repaired[index],
    role: "haiku",
    label: explicitHaiku.label,
    routeId: explicitHaiku.routeId,
    displayName: explicitHaiku.displayName,
    requestModel: explicitHaiku.requestModel,
    supports1m: explicitHaiku.supports1m,
  };
  return repaired;
}

function normalizeSupplierModelMappingFields(profile: RelayProfile) {
  const originalText = profile.modelMapping ?? "";
  const textRows = supplierModelMappingRowsFromText(originalText);
  const rawJson = (profile.modelMappingJson ?? "").trim();
  let entries = rawJson ? supplierModelMappingEntriesFromJson(rawJson) : null;

  if ((!rawJson || (entries && entries.length === 0)) && textRows.length > 0) {
    entries = textRows.map((row) => ({ ...row }));
  } else if (entries) {
    entries = repairKnownClaudeHaikuMappingSplit(entries, textRows);
  }

  if (!entries || entries.length === 0) {
    return { modelMapping: originalText, modelMappingJson: rawJson };
  }
  const rows = entries
    .map(supplierModelMappingRowFromEntry)
    .filter((row): row is SupplierModelMappingRow => row !== null);
  if (!rows.length) {
    return { modelMapping: originalText, modelMappingJson: rawJson };
  }
  return {
    modelMapping: supplierModelMappingText(rows),
    modelMappingJson: JSON.stringify(entries, null, 2),
  };
}

export const SUPPLIER_API_FORMAT_OPTIONS = [
  {
    value: "Anthropic Messages",
    label: "Anthropic Messages（原生）",
    detail: "Claude / Claude Desktop 原生 Anthropic Messages 协议，不需要路由。",
    routeRequired: false,
  },
  {
    value: "OpenAI Chat Completions",
    label: "OpenAI Chat Completions（需要路由）",
    detail: "通过本地路由把 Anthropic 请求转换为 OpenAI Chat Completions。",
    routeRequired: true,
  },
  {
    value: "OpenAI Responses API",
    label: "OpenAI Responses API（需要路由）",
    detail: "通过本地路由把 Anthropic 请求转换为 OpenAI Responses API。",
    routeRequired: true,
  },
  {
    value: "Gemini Native generateContent",
    label: "Gemini Native generateContent（需要路由）",
    detail: "通过本地路由把 Anthropic 请求转换为 Gemini 原生 generateContent。",
    routeRequired: true,
  },
] as const;

export function normalizedSupplierApiFormat(value?: string) {
  const raw = (value || "").trim();
  if (!raw) return "";
  const key = raw.toLowerCase().replace(/[\s_-]+/g, "");
  if (key === "anthropic" || key === "anthropicmessages") return "Anthropic Messages";
  if (key === "openaichat" || key === "openaichatcompletions" || raw === "OpenAI Chat Completions") return "OpenAI Chat Completions";
  if (key === "openairesponses" || key === "openairesponsesapi" || raw === "OpenAI Responses") return "OpenAI Responses API";
  if (key === "gemininative" || key === "gemininativegeneratecontent") return "Gemini Native generateContent";
  return raw;
}

export function supplierApiFormatOption(value?: string) {
  const normalized = normalizedSupplierApiFormat(value);
  return SUPPLIER_API_FORMAT_OPTIONS.find((option) => option.value === normalized);
}

export function supplierApiFormatRequiresRoute(value?: string) {
  return supplierApiFormatOption(value)?.routeRequired ?? false;
}

export function supplierRouteEnabled(profile: RelayProfile) {
  return !!profile.routeEnabled;
}

export function supplierModelMappingRows(profile: RelayProfile): SupplierModelMappingRow[] {
  const defaults = SUPPLIER_MODEL_MAPPING_DEFAULTS.map((row) => ({ ...row }));
  const normalized = normalizeSupplierModelMappingFields(profile);
  const raw = normalized.modelMappingJson.trim() || supplierModelMappingJsonFromText(normalized.modelMapping);
  if (!raw.trim()) return defaults;
  try {
    const parsed = JSON.parse(raw) as Partial<SupplierModelMappingRow>[] | Record<string, {
      model?: unknown;
      requestModel?: unknown;
      displayName?: unknown;
      labelOverride?: unknown;
      supports1m?: unknown;
      supports_1m?: unknown;
    }>;
    if (!Array.isArray(parsed) && parsed && typeof parsed === "object") {
      return defaults.map((row) => {
        const found = parsed[row.routeId] ?? Object.entries(parsed).find(([, value]) =>
          typeof value?.model === "string" && value.model === row.requestModel)?.[1];
        return found ? {
          ...row,
          displayName: typeof found.labelOverride === "string"
            ? found.labelOverride
            : typeof found.displayName === "string"
              ? found.displayName
              : row.displayName,
          requestModel: typeof found.model === "string"
            ? found.model
            : typeof found.requestModel === "string"
              ? found.requestModel
              : row.requestModel,
          supports1m: typeof found.supports1m === "boolean"
            ? found.supports1m
            : typeof found.supports_1m === "boolean"
              ? found.supports_1m
              : row.supports1m,
        } : row;
      });
    }
    if (!Array.isArray(parsed)) return defaults;
    return defaults.map((row) => {
      const found = parsed.find((item) => item.role === row.role || item.label === row.label || item.routeId === row.routeId);
      return found ? {
        ...row,
        routeId: typeof found.routeId === "string" ? found.routeId : row.routeId,
        displayName: typeof found.displayName === "string" ? found.displayName : row.displayName,
        requestModel: typeof found.requestModel === "string" ? found.requestModel : row.requestModel,
        supports1m: typeof found.supports1m === "boolean" ? found.supports1m : row.supports1m,
      } : row;
    });
  } catch {
    return defaults;
  }
}

export function supplierModelMappingJson(rows: SupplierModelMappingRow[]) {
  return JSON.stringify(rows, null, 2);
}

export function supplierModelMappingText(rows: SupplierModelMappingRow[]) {
  return rows
    .map((row) => `${row.label} (${row.routeId}): ${row.displayName || row.requestModel || ""} -> ${row.requestModel || row.displayName || ""}${row.supports1m ? " [1M]" : ""}`)
    .join("\n");
}

export function supplierModelMappingJsonFromText(text: string) {
  const rows = supplierModelMappingRowsFromText(text);
  return rows.length ? supplierModelMappingJson(rows) : "";
}

export function withSupplierPreservedImportedFiles(profile: RelayProfile): RelayProfile {
  const codexProfileHasEditedEndpoint = (profile.targetApp || "codex") === "codex"
    && !!(profile.baseUrl || profile.upstreamBaseUrl || "").trim();
  const submittedProfile = codexProfileHasEditedEndpoint
    ? { ...profile, relayMode: "pureApi" as const, officialMixApiKey: false }
    : profile;
  const currentApiKey = (profile.apiKey || "").trim();
  const apiKeyExplicit = profile.apiKeyExplicit === true;
  const normalized = normalizeSupplierProfile({
    ...submittedProfile,
    apiKey: currentApiKey,
    apiKeyExplicit,
  });
  const apiKey = apiKeyExplicit ? currentApiKey : supplierProfileResolvedApiKey(normalized);
  const current = { ...normalized, apiKey, apiKeyExplicit };
  const isClaudeTarget = normalized.targetApp === "claude" || normalized.targetApp === "claude-desktop";
  if (isClaudeTarget) {
    // 凭据之外的托管键（Base URL、模型与角色映射）也要按表单重写：
    // 只换凭据会让「改了模型配置、保存后下面的 JSON 还是第一次的样子」，
    // 而真正写进 agent 的 env 也一直停在导入时的旧值。
    const authKey = preferredClaudeCredentialField(current);
    return {
      ...current,
      configContents: supplierEffectiveConfigContents({ ...current, apiKey, authField: authKey }),
      authContents: synchronizeClaudeAuthCredential(current.authContents, apiKey, authKey),
    };
  }
  // Codex 供应商同一条规矩：托管键（模型、上下文窗口、provider 表里的地址与协议）
  // 按表单重写，用户自加的行保留 —— 官方登录档案不动，它的配置不归这张表单管。
  return {
    ...current,
    configContents: supplierProfileIsCodexOfficialLogin(current)
      ? current.configContents ?? ""
      : supplierEffectiveConfigContents(current),
    authContents: synchronizeCodexAuthCredential(current.authContents, apiKey),
  };
}

const CLAUDE_CREDENTIAL_FIELDS = [
  "OPENAI_API_KEY",
  "ANTHROPIC_AUTH_TOKEN",
  "ANTHROPIC_API_KEY",
  "api_key",
  "apiKey",
] as const;

type ClaudeCredentialField = "ANTHROPIC_AUTH_TOKEN" | "ANTHROPIC_API_KEY";

function jsonObject(contents: string): Record<string, unknown> | null {
  try {
    const parsed = JSON.parse(String(contents || "")) as unknown;
    return parsed && typeof parsed === "object" && !Array.isArray(parsed)
      ? parsed as Record<string, unknown>
      : null;
  } catch {
    return null;
  }
}

function objectHasCredentialField(object: Record<string, unknown> | null, field: ClaudeCredentialField) {
  if (!object) return false;
  if (typeof object[field] === "string") return true;
  const env = object.env;
  return !!env && typeof env === "object" && !Array.isArray(env)
    && typeof (env as Record<string, unknown>)[field] === "string";
}

function preferredClaudeCredentialField(profile: RelayProfile): ClaudeCredentialField {
  if (profile.authField === "ANTHROPIC_API_KEY") return "ANTHROPIC_API_KEY";
  if (profile.authField === "ANTHROPIC_AUTH_TOKEN") return "ANTHROPIC_AUTH_TOKEN";

  const containers = [jsonObject(profile.configContents), jsonObject(profile.authContents)];
  const hasApiKey = containers.some((container) => objectHasCredentialField(container, "ANTHROPIC_API_KEY"));
  const hasAuthToken = containers.some((container) => objectHasCredentialField(container, "ANTHROPIC_AUTH_TOKEN"));
  return hasApiKey && !hasAuthToken ? "ANTHROPIC_API_KEY" : "ANTHROPIC_AUTH_TOKEN";
}

function removeCredentialFields(object: Record<string, unknown>) {
  for (const field of CLAUDE_CREDENTIAL_FIELDS) delete object[field];
  for (const value of Object.values(object)) removeCredentialFieldsFromValue(value);
}

function removeCredentialFieldsFromValue(value: unknown) {
  if (Array.isArray(value)) {
    for (const item of value) removeCredentialFieldsFromValue(item);
    return;
  }
  if (value && typeof value === "object") {
    removeCredentialFields(value as Record<string, unknown>);
  }
}

function synchronizeClaudeConfigCredential(
  profile: RelayProfile,
  apiKey: string,
  authKey: ClaudeCredentialField,
) {
  const parsed = jsonObject(profile.configContents);
  if (!parsed) {
    return withSupplierGeneratedFiles({ ...profile, apiKey, authField: authKey }).configContents;
  }

  const config = { ...parsed };
  const currentEnv = config.env;
  const env = currentEnv && typeof currentEnv === "object" && !Array.isArray(currentEnv)
    ? { ...currentEnv as Record<string, unknown> }
    : {};
  removeCredentialFields(config);
  removeCredentialFields(env);
  if (apiKey) env[authKey] = apiKey;
  config.env = env;
  return `${JSON.stringify(config, null, 2)}\n`;
}

function synchronizeClaudeAuthCredential(
  contents: string,
  apiKey: string,
  authKey: ClaudeCredentialField,
) {
  const auth = { ...(jsonObject(contents) ?? {}) };
  removeCredentialFields(auth);
  if (apiKey) auth[authKey] = apiKey;
  return `${JSON.stringify(auth, null, 2)}\n`;
}

function synchronizeCodexAuthCredential(contents: string, apiKey: string) {
  const auth = { ...(jsonObject(contents) ?? {}) };
  removeCredentialFields(auth);
  if (apiKey) auth.OPENAI_API_KEY = apiKey;
  return Object.keys(auth).length ? `${JSON.stringify(auth, null, 2)}\n` : "";
}

export function withSupplierGeneratedFiles(profile: RelayProfile): RelayProfile {
  const normalized = normalizeSupplierProfile(profile);
  const apiKey = supplierProfileResolvedApiKey(normalized);
  const generated = { ...normalized, apiKey };
  if (generated.targetApp === "claude" || generated.targetApp === "claude-desktop") {
    const authKey = preferredClaudeCredentialField(generated);
    const routeRows = generated.modelMappingEnabled
      ? supplierModelMappingRows(generated).filter((row) => row.routeId.trim() && row.requestModel.trim())
      : [];
    const claudeDesktopModelRoutes = Object.fromEntries(routeRows.map((row) => [row.routeId.trim(), {
      model: row.requestModel.trim(),
      labelOverride: row.displayName.trim() || undefined,
      supports1m: row.supports1m,
    }]));
    const isDirectMode = !generated.modelMappingEnabled && !(generated.modelList || "").trim();
    return {
      ...generated,
      configContents: `${JSON.stringify({
        app_type: generated.targetApp,
        env: {
          ANTHROPIC_BASE_URL: generated.baseUrl,
          ...(apiKey ? { [authKey]: apiKey } : {}),
          ANTHROPIC_MODEL: generated.model,
          ...Object.fromEntries(routeRows.map((row) => {
            const key = row.role === "sonnet"
              ? "ANTHROPIC_DEFAULT_SONNET_MODEL"
              : row.role === "opus"
                ? "ANTHROPIC_DEFAULT_OPUS_MODEL"
                : row.role === "fable"
                  ? "ANTHROPIC_DEFAULT_FABLE_MODEL"
                  : row.role === "haiku"
                    ? "ANTHROPIC_DEFAULT_HAIKU_MODEL"
                    : "CLAUDE_CODE_SUBAGENT_MODEL";
            return [key, row.requestModel.trim()];
          })),
        },
        meta: {
          apiFormat: normalizedSupplierApiFormat(generated.apiFormat || "Anthropic Messages"),
          claudeDesktopMode: isDirectMode ? "direct" : "proxy",
          claudeDesktopModelRoutes,
        },
      }, null, 2)}\n`,
      authContents: apiKey ? `${JSON.stringify({ [authKey]: apiKey }, null, 2)}\n` : "",
    };
  }
  return {
    ...generated,
    configContents: buildSupplierConfigToml(generated),
    authContents: apiKey ? `${JSON.stringify({ OPENAI_API_KEY: apiKey }, null, 2)}\n` : "",
  };
}

/** 「配置 JSON」里由管理器托管的 env 键：保存时整体按表单重写，其余键原样保留。 */
const CLAUDE_MANAGED_ENV_KEYS = [
  "ANTHROPIC_BASE_URL",
  "CLAUDE_BASE_URL",
  "ANTHROPIC_MODEL",
  "ANTHROPIC_DEFAULT_HAIKU_MODEL",
  "ANTHROPIC_DEFAULT_OPUS_MODEL",
  "ANTHROPIC_DEFAULT_FABLE_MODEL",
  "ANTHROPIC_DEFAULT_SONNET_MODEL",
  "CLAUDE_CODE_SUBAGENT_MODEL",
  "OPENAI_API_KEY",
  "ANTHROPIC_AUTH_TOKEN",
  "ANTHROPIC_API_KEY",
  "api_key",
  "apiKey",
] as const;

function asJsonObject(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null;
}

/**
 * 把表单能改的托管键按新值写进存量 JSON，导入供应商自带的其它键（自定义 env、
 * meta 里的额外字段、localProxyOverrides 等）保留 —— 存量键不再能盖住表单。
 */
function mergeClaudeConfigOntoStored(stored: string, generated: string) {
  const previous = jsonObject(stored);
  const next = jsonObject(generated);
  if (!previous || !next) return generated || stored;
  const previousEnv = asJsonObject(previous.env) ?? {};
  const env: Record<string, unknown> = { ...previousEnv };
  for (const key of CLAUDE_MANAGED_ENV_KEYS) delete env[key];
  const meta = { ...(asJsonObject(previous.meta) ?? {}), ...(asJsonObject(next.meta) ?? {}) };
  return `${JSON.stringify({ ...previous, ...next, env: { ...env, ...(asJsonObject(next.env) ?? {}) }, meta }, null, 2)}\n`;
}

/** Codex 的 config.toml 里由管理器托管的键：保存时整体按表单重写，其余行原样保留。 */
const CODEX_MANAGED_TOP_KEYS = ["model", "model_provider", "model_context_window"];
const CODEX_MANAGED_PROVIDER_KEYS = [
  "name",
  "wire_api",
  "requires_openai_auth",
  "experimental_bearer_token",
  "env_key",
  "base_url",
];

function tomlSectionHeader(line: string) {
  const match = /^\s*\[([^\]]+)\]\s*$/.exec(line);
  return match ? match[1] : "";
}

function tomlAssignedKey(line: string) {
  const match = /^\s*([A-Za-z0-9_.\-]+)\s*=/.exec(line);
  return match ? match[1] : "";
}

function tomlQuotedValue(line: string) {
  const match = /^\s*[A-Za-z0-9_.\-]+\s*=\s*"([^"]*)"/.exec(line);
  return match ? match[1] : "";
}

/**
 * 把表单能改的托管键按新值写进存量的 config.toml：模型、上下文窗口、以及该 provider
 * 表里的名字、协议、鉴权字段与地址。其余行与别的表保留 —— 存量键不再能盖住表单，
 * 所以「上面改了模型，下面 config 不动」不会再出现。
 */
function mergeCodexConfigOntoStored(stored: string, generated: string) {
  const top = new Map<string, string>();
  const provider = new Map<string, string>();
  let section = "";
  for (const line of generated.split("\n")) {
    const header = tomlSectionHeader(line);
    if (header) {
      section = header;
      continue;
    }
    const key = tomlAssignedKey(line);
    if (!key) continue;
    if (!section) top.set(key, line.trimEnd());
    else if (section.startsWith("model_providers.")) provider.set(key, line.trimEnd());
  }
  const providerId = tomlQuotedValue(top.get("model_provider") ?? "");
  const providerSection = `model_providers.${providerId}`;

  const keptTop: string[] = [];
  const sections: { name: string; lines: string[] }[] = [];
  let storedProviderId = "";
  let current: { name: string; lines: string[] } | null = null;
  for (const line of stored.split(/\r?\n/)) {
    const header = tomlSectionHeader(line);
    if (header) {
      current = { name: header, lines: [] };
      sections.push(current);
      continue;
    }
    const key = tomlAssignedKey(line);
    if (!current) {
      if (key === "model_provider") storedProviderId = tomlQuotedValue(line);
      if (key && CODEX_MANAGED_TOP_KEYS.includes(key)) continue;
      keptTop.push(line);
      continue;
    }
    if (current.name === providerSection && key && CODEX_MANAGED_PROVIDER_KEYS.includes(key)) continue;
    current.lines.push(line);
  }

  // 存量配置指向另一个 provider id（旧 id 是按 UI 编号取的）：把旧表里不属于托管键的
  // 行（http_headers、自定义字段、上游私有参数）搬到新表，别在换 id 时悄悄丢掉。
  let carriedProviderLines: string[] = [];
  const out: string[] = [];
  for (const key of CODEX_MANAGED_TOP_KEYS) {
    const line = top.get(key);
    if (line) out.push(line);
  }
  out.push(...keptTop);
  let wroteProvider = false;
  for (const item of sections) {
    if (item.name.startsWith("model_providers.") && item.name !== providerSection) {
      if (
        storedProviderId
        && storedProviderId !== providerId
        && item.name === `model_providers.${storedProviderId}`
      ) {
        carriedProviderLines = item.lines.filter(
          (line) => line.trim() && !CODEX_MANAGED_PROVIDER_KEYS.includes(tomlAssignedKey(line)),
        );
      }
      continue;
    }
    if (item.name === providerSection) {
      out.push(`[${providerSection}]`);
      for (const key of CODEX_MANAGED_PROVIDER_KEYS) {
        const line = provider.get(key);
        if (line) out.push(line);
      }
      out.push(...item.lines);
      wroteProvider = true;
      continue;
    }
    out.push(`[${item.name}]`);
    out.push(...item.lines);
  }
  if (!wroteProvider && providerId) {
    out.push(`[${providerSection}]`);
    for (const key of CODEX_MANAGED_PROVIDER_KEYS) {
      const line = provider.get(key);
      if (line) out.push(line);
    }
    out.push(...carriedProviderLines);
  }
  return `${out.join("\n").replace(/\n{3,}/g, "\n\n").replace(/^\n+/, "").replace(/\s+$/, "")}\n`;
}

/** 保存与显示共用同一份规则：这家供应商写进 agent 的那份文件会是什么样。 */
export function supplierEffectiveConfigContents(profile: RelayProfile): string {
  const normalized = normalizeSupplierProfile(profile);
  const isClaudeTarget = normalized.targetApp === "claude" || normalized.targetApp === "claude-desktop";
  const authKey = isClaudeTarget ? preferredClaudeCredentialField(normalized) : "";
  const generated = withSupplierGeneratedFiles({
    ...normalized,
    apiKey: supplierProfileResolvedApiKey(normalized),
    ...(authKey ? { authField: authKey } : {}),
  });
  const stored = (normalized.configContents ?? "").trim();
  if (isClaudeTarget) {
    return stored ? mergeClaudeConfigOntoStored(stored, generated.configContents) : generated.configContents;
  }
  return stored ? mergeCodexConfigOntoStored(stored, generated.configContents) : generated.configContents;
}


export function supplierProfileHasApiKey(profile: RelayProfile) {
  return !!supplierProfileResolvedApiKey(profile);
}

export function supplierProfileIsCodexOfficialLogin(profile: RelayProfile) {
  const targetApp = String(profile.targetApp || "codex").trim().toLowerCase();
  return targetApp === "codex"
    && profile.relayMode === "official"
    && !profile.officialMixApiKey
    && !supplierProfileIsCcswitch(profile);
}

export function supplierProfileCanActivate(profile: RelayProfile) {
  return supplierProfileIsCodexOfficialLogin(profile) || supplierProfileHasApiKey(profile);
}

export function supplierProfileIsCcswitch(profile: RelayProfile) {
  const name = (profile.name || "").toLowerCase();
  const userAgent = profile.userAgent || "";
  return (profile.importSource || "") === "cc-switch" || userAgent === "ccswitch" || userAgent.startsWith("ccswitch:") || name.includes("ccswitch") || name.includes("cc-switch");
}

export function supplierTargetAppLabel(targetApp?: string) {
  if (targetApp === "claude") return "Claude";
  if (targetApp === "claude-desktop") return "Claude Desktop";
  if (targetApp === "codex") return "Codex";
  return targetApp || "Codex";
}

export function supplierApiFormatLabel(profile: RelayProfile) {
  if (profile.apiFormat) return normalizedSupplierApiFormat(profile.apiFormat);
  return supplierProtocolLabel(profile.protocol);
}

export function supplierProfileResolvedApiKey(profile: RelayProfile) {
  const explicitKey = (profile.apiKey || "").trim();
  if (profile.apiKeyExplicit) return explicitKey;
  if (explicitKey) return explicitKey;

  const authKey = supplierApiKeyFromAuthContents(profile.authContents);
  const configKey = supplierApiKeyFromConfigContents(profile.configContents);
  return supplierProfilePrefersConfigApiKey(profile)
    ? configKey || authKey
    : authKey || configKey;
}

function supplierProfilePrefersConfigApiKey(profile: RelayProfile) {
  const targetApp = String(profile.targetApp || "codex").trim().toLowerCase();
  return targetApp === "claude" || targetApp === "claude-desktop";
}

export function supplierApiKeyFromAuthContents(contents: string) {
  const text = String(contents || "").trim();
  if (!text) return "";
  try {
    const parsed = JSON.parse(text) as Record<string, unknown>;
    for (const key of ["OPENAI_API_KEY", "ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_API_KEY", "api_key", "apiKey"]) {
      const value = parsed[key];
      if (typeof value === "string" && value.trim()) return value.trim();
    }
  } catch {
    const match = text.match(/"(?:OPENAI_API_KEY|ANTHROPIC_AUTH_TOKEN|ANTHROPIC_API_KEY|api_key|apiKey)"\s*:\s*"([^"]+)"/);
    if (match?.[1]?.trim()) return match[1].trim();
  }
  return "";
}

export function supplierApiKeyFromConfigContents(contents: string) {
  const text = String(contents || "");
  try {
    const parsed = JSON.parse(text) as Record<string, unknown> & { env?: Record<string, unknown> };
    for (const source of [parsed, parsed.env]) {
      if (!source) continue;
      for (const key of ["OPENAI_API_KEY", "ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_API_KEY", "api_key", "apiKey"]) {
        const value = source[key];
        if (typeof value === "string" && value.trim()) return value.trim();
      }
    }
  } catch {
    // Codex profiles use TOML; fall through to the existing TOML-compatible extraction.
  }
  const bearer = text.match(/experimental_bearer_token\s*=\s*["']([^"']+)["']/);
  if (bearer?.[1]?.trim()) return bearer[1].trim();
  const authorization = text.match(/Authorization\s*=\s*["']Bearer\s+([^"']+)["']/i);
  return authorization?.[1]?.trim() || "";
}

/** 核心侧 `local_responses_proxy_base_url(57321)`：Codex 走本机代理时 config.toml 里的地址。 */
const CODEX_LOCAL_PROXY_BASE_URL = "http://127.0.0.1:57321/v1";

/**
 * Codex 什么时候要经本机代理：开了路由，或者用户为这家供应商声明了 Codex 说不出的
 * 上游格式（Chat Completions、Anthropic Messages）—— 与核心侧 `codex_base_url_for_profile`
 * 同一条规则。
 */
function codexRouteGoesThroughLocalProxy(profile: RelayProfile) {
  if ((profile.targetApp || "codex") !== "codex") return false;
  const format = String(profile.apiFormat || "").trim().toLowerCase().replace(/[\s_-]+/g, "");
  if (format === "anthropic" || format === "anthropicmessages") return true;
  // 原生 Responses 的上游直连：开不开路由都不经过代理 —— 代理只为「格式不一样」而存在。
  if (format === "openairesponses") return false;
  return String(profile.protocol || "").trim().toLowerCase() === "chatcompletions";
}

export function buildSupplierConfigToml(profile: RelayProfile) {
  const model = profile.model.trim();
  const baseUrl = codexRouteGoesThroughLocalProxy(profile)
    ? CODEX_LOCAL_PROXY_BASE_URL
    : profile.baseUrl.trim();
  const providerSource = profile.protocol === "chatCompletions" && profile.upstreamBaseUrl
    ? profile.upstreamBaseUrl
    : profile.baseUrl;
  // 只由真实上游地址派生：走本机代理时 config 里写的是 127.0.0.1，那不是中转身份。
  const providerId =
    supplierIdForBaseUrl(providerSource) ||
    supplierIdFromName(profile.id || profile.name);
  const apiKey = supplierProfileResolvedApiKey(profile);
  const contextWindow = supplierCodexEffectiveContextWindow(profile) || normalizedContextWindow(profile.contextWindow);
  return [
    model ? `model = ${tomlString(model)}` : null,
    `model_provider = ${tomlString(providerId)}`,
    'model_reasoning_effort = "high"',
    contextWindow ? `model_context_window = ${contextWindow}` : null,
    "",
    `[model_providers.${providerId}]`,
    `name = ${tomlString(profile.name.trim() || providerId)}`,
    'wire_api = "responses"',
    "requires_openai_auth = true",
    apiKey ? `experimental_bearer_token = ${tomlString(apiKey)}` : 'env_key = "OPENAI_API_KEY"',
    baseUrl ? `base_url = ${tomlString(baseUrl)}` : null,
    "",
  ].filter((line): line is string => line !== null).join("\n");
}

export function tomlString(value: string) {
  return JSON.stringify(value);
}

export function firstSupplierModel(modelList: string) {
  return supplierDirectModelRows(modelList)[0]?.model || "";
}

export function redactSupplierAuth(contents: string) {
  try {
    const parsed = JSON.parse(contents || "{}") as Record<string, unknown>;
    for (const key of ["OPENAI_API_KEY", "ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_API_KEY", "api_key", "apiKey"]) {
      const value = parsed[key];
      if (typeof value === "string" && value) {
        parsed[key] = `${value.slice(0, 6)}...${value.slice(-4)}`;
      }
    }
    return `${JSON.stringify(parsed, null, 2)}\n`;
  } catch {
    return "{\n  \"OPENAI_API_KEY\": \"***\"\n}\n";
  }
}

const SUPPLIER_SECRET_CONFIG_KEYS = new Set([
  "apikey",
  "openaiapikey",
  "anthropicapikey",
  "anthropicauthtoken",
  "authorization",
  "bearertoken",
  "accesstoken",
  "token",
  "cookie",
  "clientsecret",
  "password",
  "privatekey",
  "secret",
]);

function supplierConfigKeyIsSecret(key: string) {
  const normalized = key.replace(/[^a-z]/gi, "").toLowerCase();
  return SUPPLIER_SECRET_CONFIG_KEYS.has(normalized)
    || normalized.endsWith("apikey")
    || normalized.endsWith("authtoken")
    || normalized.endsWith("bearertoken")
    || normalized.endsWith("accesstoken")
    || normalized.endsWith("secret")
    || normalized.endsWith("privatekey");
}

function redactSupplierConfigValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(redactSupplierConfigValue);
  if (!value || typeof value !== "object") return value;
  return Object.fromEntries(Object.entries(value as Record<string, unknown>).map(([key, child]) => [
    key,
    supplierConfigKeyIsSecret(key) && typeof child === "string"
      ? "***redacted***"
      : redactSupplierConfigValue(child),
  ]));
}

/** Redacts credentials before a hidden API-key configuration preview is shown. */
export function redactSupplierConfig(contents: string) {
  const text = String(contents || "");
  try {
    const value = JSON.parse(text);
    const suffix = text.endsWith("\n") ? "\n" : "";
    return `${JSON.stringify(redactSupplierConfigValue(value), null, 2)}${suffix}`;
  } catch {
    return text
      .replace(
        /((?:OPENAI_API_KEY|ANTHROPIC_AUTH_TOKEN|ANTHROPIC_API_KEY|experimental_bearer_token|bearer_token|access_token|api_key|apiKey)\s*[=:]\s*["']?)([^"'\s,}\]]+)/gi,
        "$1***redacted***",
      )
      .replace(
        /(Authorization\s*[=:]\s*["']?(?:Bearer\s+)?)([^"'\s,}\]]+)/gi,
        "$1***redacted***",
      );
  }
}

export function supplierCategoryLabel(category: SupplierPreset["category"]) {
  const labels: Record<SupplierPreset["category"], string> = {
    official: "官方",
    cn_official: "国内官方",
    aggregator: "聚合/中转",
    third_party: "第三方",
  };
  return labels[category];
}

export function aggregateStrategyLabel(strategy?: string) {
  return AGGREGATE_STRATEGIES.find((item) => item.id === strategy)?.label ?? "失败切换";
}

export function supplierProtocolLabel(protocol?: string) {
  return protocol === "chatCompletions" ? "Chat Completions" : "Responses";
}

export function supplierRelayModeLabel(mode?: string) {
  if (mode === "official") return "官方登录";
  if (mode === "mixedApi") return "官方混入 API Key";
  return "纯 API";
}
