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
    aggregateEnabled: !!profile.aggregateEnabled,
    aggregateStrategy: profile.aggregateStrategy || (profile.aggregateEnabled ? "failover" : ""),
    aggregateMembers: Array.isArray(profile.aggregateMembers) ? profile.aggregateMembers : [],
  };
}

export function withSupplierGeneratedFiles(profile: RelayProfile): RelayProfile {
  const normalized = normalizeSupplierProfile(profile);
  const apiKey = supplierProfileResolvedApiKey(normalized);
  const generated = { ...normalized, apiKey };
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
  const name = profile.name.toLowerCase();
  return profile.userAgent === "ccswitch" || name.includes("ccswitch") || name.includes("cc-switch");
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

export function supplierApiKeyFromConfigContents(contents: string) {
  const match = String(contents || "").match(/experimental_bearer_token\s*=\s*["']([^"']+)["']/);
  return match?.[1]?.trim() || "";
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
    if (typeof parsed.OPENAI_API_KEY === "string" && parsed.OPENAI_API_KEY) {
      parsed.OPENAI_API_KEY = `${parsed.OPENAI_API_KEY.slice(0, 6)}...${parsed.OPENAI_API_KEY.slice(-4)}`;
    }
    return `${JSON.stringify(parsed, null, 2)}\n`;
  } catch {
    return "{\n  \"OPENAI_API_KEY\": \"***\"\n}\n";
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
