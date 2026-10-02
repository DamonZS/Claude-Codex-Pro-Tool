import type { AgentAppId } from "@/components/settings/contract";

/**
 * Suppliers for agents that are not served by the Codex / Claude / Claude
 * Desktop relay editor. Each agent keeps its own native config file; CCP only
 * patches the provider entry it owns (CC Switch style).
 */
export type AgentProviderAppId = Exclude<AgentAppId, "codex" | "claude" | "claude-desktop">;

/** How CCP applies a supplier to the agent. */
export type AgentApplyMode =
  /** Only one supplier is active; switching rewrites the key fields. */
  | "switch"
  /** Many suppliers live side by side in the agent's own file. */
  | "additive"
  /** Settings live in encrypted app storage; CCP shows copy-ready values. */
  | "manual";

export type AgentApiFormat = "openai-chat" | "openai-responses" | "anthropic";

export type AgentProviderApp = {
  id: AgentProviderAppId;
  label: string;
  mode: AgentApplyMode;
  formats: AgentApiFormat[];
  /** Shown under the list so users know which file CCP touches. */
  configHint: string;
};

export const AGENT_PROVIDER_APPS: ReadonlyArray<AgentProviderApp> = [
  { id: "gemini", label: "Gemini", mode: "switch", formats: ["openai-chat"], configHint: "~/.gemini/.env · settings.json" },
  { id: "grok", label: "Grok Build", mode: "switch", formats: ["openai-chat", "openai-responses"], configHint: "~/.grok/config.toml" },
  { id: "opencode", label: "OpenCode", mode: "additive", formats: ["openai-chat", "anthropic"], configHint: "~/.config/opencode/opencode.json" },
  { id: "openclaw", label: "OpenClaw", mode: "additive", formats: ["openai-chat", "openai-responses", "anthropic"], configHint: "~/.openclaw/openclaw.json" },
  { id: "hermes", label: "Hermes", mode: "additive", formats: ["openai-chat"], configHint: "~/.hermes/config.yaml" },
  { id: "pi", label: "Pi", mode: "additive", formats: ["openai-chat", "openai-responses", "anthropic"], configHint: "~/.pi/agent/models.json" },
  { id: "mcode", label: "MiniMax Code", mode: "additive", formats: ["anthropic", "openai-chat", "openai-responses"], configHint: "~/.minimax/config.yaml" },
  { id: "workbuddy", label: "WorkBuddy", mode: "additive", formats: ["openai-chat"], configHint: "~/.workbuddy/models.json" },
  { id: "cursor", label: "Cursor", mode: "manual", formats: ["openai-chat"], configHint: "Cursor Settings → Models" },
];

export function agentProviderApp(id: AgentProviderAppId) {
  return AGENT_PROVIDER_APPS.find((app) => app.id === id) ?? AGENT_PROVIDER_APPS[0];
}

export type AgentProvider = {
  id: string;
  appId: AgentProviderAppId;
  name: string;
  baseUrl: string;
  /** Write-only: never returned by the backend, only sent when changed. */
  apiKey?: string;
  /** True when a key is stored for this supplier. */
  hasApiKey: boolean;
  apiFormat: AgentApiFormat;
  models: string[];
  /** Model the agent should default to (switch-mode apps). */
  defaultModel: string;
  notes: string;
  sortIndex: number;
};

export type AgentProviderState = {
  /** Switch-mode: id of the supplier written to the live file. */
  activeId: string | null;
  /** Additive-mode: ids currently present in the agent's file. */
  appliedIds: string[];
  /** Resolved config path, or empty when the agent is not installed. */
  configPath: string;
  installed: boolean;
};

/**
 * `apply_agent_provider` result. For manual-mode apps (Cursor) the backend
 * returns the values once so the user can paste them into the app's settings.
 */
export type AgentApplyResult = AgentProvidersResult & {
  reveal?: { baseUrl: string; apiKey: string; model: string } | null;
};

export type AgentProvidersResult = {
  status: string;
  message: string;
  providers: AgentProvider[];
  states: Partial<Record<AgentProviderAppId, AgentProviderState>>;
};

export const AGENT_PROVIDER_COMMANDS = {
  list: "list_agent_providers",
  save: "save_agent_provider",
  delete: "delete_agent_provider",
  apply: "apply_agent_provider",
  unapply: "unapply_agent_provider",
  reorder: "reorder_agent_providers",
} as const;

export function emptyAgentProvider(appId: AgentProviderAppId): AgentProvider {
  const app = agentProviderApp(appId);
  return {
    id: "",
    appId,
    name: "",
    baseUrl: "",
    apiKey: "",
    hasApiKey: false,
    apiFormat: app.formats[0],
    models: [],
    defaultModel: "",
    notes: "",
    sortIndex: 0,
  };
}
