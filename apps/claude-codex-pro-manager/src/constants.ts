// 模块级常量。从 App.tsx 抽出（任务#3 组件拆分）。
import type { AggregateStrategy, SupplierPreset } from "@/types";

export const PONYTAIL_REPOSITORY_URL = "https://github.com/DietrichGebert/ponytail";
export const CODEX_THIRD_PARTY_PLUGIN_MARKETPLACE_NAME = "awesome-codex-plugins";
export const CODEX_THIRD_PARTY_PLUGIN_REPOSITORY_URL =
  "https://github.com/hashgraph-online/awesome-codex-plugins.git";
export const CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_NAME = "codex-skills-alternative";
export const CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_SOURCE =
  "https://github.com/DKeken/codex-skills-alternative";
export const CODEX_PRODUCT_DESIGN_SKILL_MARKETPLACE_LOCAL_SOURCE =
  "~\\.codex\\plugins\\cache\\codex-skills-alternative-marketplace";
export const PLUGIN_REPOSITORY_REPAIR_PROMPT_KEY_PREFIX = "tools-plugin-repository-repair";
export const SUPPLIER_DRAG_MIME_TYPE = "application/x-claude-codex-pro-supplier-id";

export const MEMORY_ALL_WORKSPACES = "__all__";
export const MEMORY_GLOBAL_WORKSPACE = "global";

export const SUPPLIER_PRESETS: SupplierPreset[] = [
  {
    id: "openai",
    name: "OpenAI Official",
    category: "official",
    baseUrl: "https://api.openai.com/v1",
    protocol: "responses",
    model: "gpt-5.5",
    websiteUrl: "https://chatgpt.com/codex",
  },
  {
    id: "anthropic",
    name: "Anthropic / Claude",
    category: "official",
    baseUrl: "https://api.anthropic.com",
    protocol: "chatCompletions",
    model: "claude-sonnet-4-6",
    modelList: ["claude-sonnet-4-6", "claude-opus-4-8", "claude-haiku-4-5"],
    websiteUrl: "https://claude.ai",
    apiKeyUrl: "https://console.anthropic.com/settings/keys",
    targetApp: "claude-desktop",
    apiFormat: "Anthropic Messages",
    claudeDesktopMode: "direct",
    routeEnabled: false,
    routeMode: "Claude Desktop Direct",
    modelMappingEnabled: false,
  },
  {
    id: "deepseek",
    name: "DeepSeek",
    category: "cn_official",
    baseUrl: "https://api.deepseek.com",
    protocol: "chatCompletions",
    model: "deepseek-v4-flash",
    modelList: ["deepseek-v4-flash", "deepseek-v4-pro"],
    apiKeyUrl: "https://platform.deepseek.com/api_keys",
  },
  {
    id: "kimi",
    name: "Kimi",
    category: "cn_official",
    baseUrl: "https://api.moonshot.cn/v1",
    protocol: "chatCompletions",
    model: "kimi-k2.6",
    modelList: ["kimi-k2.6"],
  },
  {
    id: "qwen",
    name: "Qwen / Bailian",
    category: "cn_official",
    baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1",
    protocol: "chatCompletions",
    model: "qwen3-coder-plus",
    modelList: ["qwen3-coder-plus", "qwen3-max"],
  },
  {
    id: "siliconflow",
    name: "SiliconFlow",
    category: "aggregator",
    baseUrl: "https://api.siliconflow.cn/v1",
    protocol: "chatCompletions",
    model: "Pro/MiniMaxAI/MiniMax-M2.7",
    modelList: ["Pro/MiniMaxAI/MiniMax-M2.7"],
  },
  {
    id: "openrouter",
    name: "OpenRouter",
    category: "aggregator",
    baseUrl: "https://openrouter.ai/api/v1",
    protocol: "chatCompletions",
    model: "openai/gpt-5.5",
  },
];

export const AGGREGATE_STRATEGIES: AggregateStrategy[] = [
  { id: "failover", label: "失败切换", detail: "请求失败后按成员顺序切换到下一个供应商。" },
  { id: "conversationRoundRobin", label: "按对话轮转", detail: "同一对话固定成员，新对话轮换成员。" },
  { id: "requestRoundRobin", label: "按请求轮转", detail: "每次请求按列表顺序轮换成员。" },
  { id: "weightedRoundRobin", label: "权重轮转", detail: "按成员权重分配请求，权重相同则平均。" },
];
