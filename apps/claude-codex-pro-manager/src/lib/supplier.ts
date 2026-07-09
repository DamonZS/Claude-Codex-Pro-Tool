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
    targetApp: profile.targetApp ?? "",
    apiFormat: profile.apiFormat ?? "",
    claudeDesktopMode: profile.claudeDesktopMode ?? "",
    routeEnabled: typeof profile.routeEnabled === "boolean"
      ? profile.routeEnabled
      : supplierApiFormatRequiresRoute(profile.apiFormat) || !!profile.modelMappingEnabled,
    routeMode: profile.routeMode ?? "",
    modelMapping: profile.modelMapping ?? "",
    modelMappingEnabled: !!profile.modelMappingEnabled || !!profile.modelMapping || !!profile.modelMappingJson,
    modelMappingJson: profile.modelMappingJson ?? supplierModelMappingJsonFromText(profile.modelMapping ?? ""),
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

export const SUPPLIER_MODEL_MAPPING_DEFAULTS: SupplierModelMappingRow[] = [
  { role: "sonnet", label: "Sonnet", routeId: "claude-sonnet-4-6", displayName: "", requestModel: "", supports1m: true },
  { role: "opus", label: "Opus", routeId: "claude-opus-4-8", displayName: "", requestModel: "", supports1m: true },
  { role: "fable", label: "Fable", routeId: "claude-fable-5", displayName: "", requestModel: "", supports1m: true },
  { role: "haiku", label: "Haiku", routeId: "claude-haiku-4-5", displayName: "", requestModel: "", supports1m: false },
  { role: "subagent", label: "Subagent", routeId: "claude-subagent", displayName: "", requestModel: "", supports1m: true },
];

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
  return !!profile.routeEnabled || supplierApiFormatRequiresRoute(profile.apiFormat);
}

export function supplierModelMappingRows(profile: RelayProfile): SupplierModelMappingRow[] {
  const defaults = SUPPLIER_MODEL_MAPPING_DEFAULTS.map((row) => ({ ...row }));
  const raw = profile.modelMappingJson || supplierModelMappingJsonFromText(profile.modelMapping || "");
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
  const rows = SUPPLIER_MODEL_MAPPING_DEFAULTS.map((row) => ({ ...row }));
  const lines = String(text || "").split(/\r?\n/);
  for (const line of lines) {
    const [left, rightRaw] = line.split(":");
    if (!left || !rightRaw) continue;
    const role = left.trim().toLowerCase();
    const row = rows.find((item) => item.role === role || item.label.toLowerCase() === role);
    if (!row) continue;
    const cleaned = rightRaw.replace(/\[1m\]/ig, "").trim();
    const routeMatch = left.match(/\(([^)]+)\)/);
    if (routeMatch?.[1]?.trim()) row.routeId = routeMatch[1].trim();
    row.displayName = cleaned;
    row.requestModel = cleaned;
    row.supports1m = /\[1m\]/i.test(rightRaw) || row.supports1m;
  }
  return rows.some((row) => row.displayName || row.requestModel) ? supplierModelMappingJson(rows) : "";
}

export function withSupplierPreservedImportedFiles(profile: RelayProfile): RelayProfile {
  const normalized = normalizeSupplierProfile(profile);
  const apiKey = supplierProfileResolvedApiKey(normalized);
  const authKey = normalized.targetApp === "claude" || normalized.targetApp === "claude-desktop"
    ? "ANTHROPIC_AUTH_TOKEN"
    : "OPENAI_API_KEY";
  return {
    ...normalized,
    apiKey,
    configContents: normalized.configContents ?? "",
    authContents: `${JSON.stringify({ [authKey]: apiKey }, null, 2)}
`,
  };
}

export function withSupplierGeneratedFiles(profile: RelayProfile): RelayProfile {
  const normalized = normalizeSupplierProfile(profile);
  const apiKey = supplierProfileResolvedApiKey(normalized);
  const generated = { ...normalized, apiKey };
  if (generated.targetApp === "claude" || generated.targetApp === "claude-desktop") {
    const routeRows = supplierModelMappingRows(generated).filter((row) => row.routeId.trim() && row.requestModel.trim());
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
          ANTHROPIC_AUTH_TOKEN: apiKey,
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
          claudeDesktopMode: generated.claudeDesktopMode || (supplierRouteEnabled(generated) ? "proxy" : "direct"),
          claudeDesktopModelRoutes,
        },
      }, null, 2)}\n`,
      authContents: `${JSON.stringify({ ANTHROPIC_AUTH_TOKEN: apiKey }, null, 2)}\n`,
    };
  }
  return {
    ...generated,
    configContents: buildSupplierConfigToml(generated),
    authContents: `${JSON.stringify({ OPENAI_API_KEY: apiKey }, null, 2)}\n`,
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
  return (profile.apiKey || "").trim()
    || supplierApiKeyFromAuthContents(profile.authContents)
    || supplierApiKeyFromConfigContents(profile.configContents);
}

export function supplierApiKeyFromAuthContents(contents: string) {
  const text = String(contents || "").trim();
  if (!text) return "";
  try {
    const parsed = JSON.parse(text) as Record<string, unknown>;
    for (const key of ["OPENAI_API_KEY", "ANTHROPIC_AUTH_TOKEN", "api_key", "apiKey"]) {
      const value = parsed[key];
      if (typeof value === "string" && value.trim()) return value.trim();
    }
  } catch {
    const match = text.match(/"(?:OPENAI_API_KEY|api_key|apiKey)"\s*:\s*"([^"]+)"/);
    if (match?.[1]?.trim()) return match[1].trim();
  }
  return "";
}

export function supplierApiKeyFromConfigContents(contents: string) {
  const text = String(contents || "");
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
  return modelList.split(/\r?\n/).map((item) => item.trim()).find(Boolean) || "";
}

export function redactSupplierAuth(contents: string) {
  try {
    const parsed = JSON.parse(contents || "{}") as Record<string, unknown>;
    for (const key of ["OPENAI_API_KEY", "ANTHROPIC_AUTH_TOKEN", "api_key", "apiKey"]) {
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
