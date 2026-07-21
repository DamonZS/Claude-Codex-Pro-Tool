import { AGGREGATE_STRATEGIES } from "@/constants";
import type { BackendSettings, RelayProfile, SupplierPreset } from "@/types";

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

export function createSupplierProfile(settings: BackendSettings): RelayProfile {
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
  const baseUrl = profile.baseUrl || profile.upstreamBaseUrl || "";
  const model = profile.model || profile.testModel || firstSupplierModel(modelList) || "gpt-5.5";
  const targetApp = profile.targetApp || "codex";
  const modelMappingEnabled = typeof profile.modelMappingEnabled === "boolean"
    ? profile.modelMappingEnabled
    : !!profile.modelMapping || !!profile.modelMappingJson;
  const routeEnabled = typeof profile.routeEnabled === "boolean"
    ? profile.routeEnabled
    : profile.claudeDesktopMode === "proxy" || /\bproxy\b/i.test(profile.routeMode || "");
  const normalizedModelMapping = normalizeSupplierModelMappingFields(profile);
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
    modelList: hasExplicitModelList ? modelList : model,
    codexCatalogJson: profile.codexCatalogJson ?? "",
    contextWindow: profile.contextWindow ?? "",
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
  contextWindow: string;
};

export function supplierCodexCatalogRows(profile: RelayProfile): SupplierCodexCatalogRow[] {
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
          const contextWindow = String(value.contextWindow ?? value.context_window ?? "")
            .replace(/[^\d]/g, "");
          return [{ displayName, model, contextWindow }];
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
      contextWindow: String(profile.contextWindow || "").replace(/[^\d]/g, ""),
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
    const contextWindow = String(row.contextWindow || "").replace(/[^\d]/g, "");
    return [{
      model,
      ...(displayName ? { displayName } : {}),
      ...(contextWindow && Number.parseInt(contextWindow, 10) > 0
        ? { contextWindow: Number.parseInt(contextWindow, 10) }
        : {}),
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
  const currentApiKey = (profile.apiKey || "").trim();
  const apiKeyExplicit = profile.apiKeyExplicit === true;
  const normalized = normalizeSupplierProfile({
    ...profile,
    apiKey: currentApiKey,
    apiKeyExplicit,
  });
  const apiKey = apiKeyExplicit ? currentApiKey : supplierProfileResolvedApiKey(normalized);
  const current = { ...normalized, apiKey, apiKeyExplicit };
  const isClaudeTarget = normalized.targetApp === "claude" || normalized.targetApp === "claude-desktop";
  if (isClaudeTarget) {
    const authKey = preferredClaudeCredentialField(current);
    return {
      ...current,
      configContents: synchronizeClaudeConfigCredential(current, apiKey, authKey),
      authContents: synchronizeClaudeAuthCredential(current.authContents, apiKey, authKey),
    };
  }
  return {
    ...current,
    configContents: current.configContents ?? "",
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
          claudeDesktopMode: supplierRouteEnabled(generated) ? "proxy" : "direct",
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

export function supplierProfileHasApiKey(profile: RelayProfile) {
  return !!supplierProfileResolvedApiKey(profile);
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

export function buildSupplierConfigToml(profile: RelayProfile) {
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
    official: "\u5b98\u65b9",
    cn_official: "\u56fd\u5185\u5b98\u65b9",
    aggregator: "\u805a\u5408/\u4e2d\u8f6c",
    third_party: "\u7b2c\u4e09\u65b9",
  };
  return labels[category];
}

export function aggregateStrategyLabel(strategy?: string) {
  return AGGREGATE_STRATEGIES.find((item) => item.id === strategy)?.label ?? "\u5931\u8d25\u5207\u6362";
}

export function supplierProtocolLabel(protocol?: string) {
  return protocol === "chatCompletions" ? "Chat Completions" : "Responses";
}

export function supplierRelayModeLabel(mode?: string) {
  if (mode === "official") return "\u5b98\u65b9\u767b\u5f55";
  if (mode === "mixedApi") return "\u5b98\u65b9\u6df7\u5165 API Key";
  return "\u7eaf API";
}
