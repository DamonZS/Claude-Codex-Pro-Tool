import { describe, expect, it } from "vitest";

import { supplierEffectiveConfigContents, withSupplierGeneratedFiles } from "@/lib/supplier";
import type { RelayProfile } from "@/types";

function profile(overrides: Partial<RelayProfile> = {}): RelayProfile {
  return {
    id: "myclaude-1783774200007-ccswitch",
    name: "小模型 (ccswitch)",
    model: "claude-fable-5",
    baseUrl: "https://api.toporeduce.cn",
    upstreamBaseUrl: "https://api.toporeduce.cn",
    apiKey: "sk-the-new-key",
    protocol: "anthropic",
    relayMode: "pureApi",
    officialMixApiKey: false,
    testModel: "",
    configContents: "",
    authContents: "",
    useCommonConfig: false,
    contextSelection: { mcpServers: [], skills: [], plugins: [] },
    contextSelectionInitialized: true,
    contextWindow: "",
    autoCompactLimit: "",
    modelList: "",
    userAgent: "ccswitch:claude",
    targetApp: "claude",
    importSource: "cc-switch",
    ...overrides,
  };
}

/** 导入时留下的那份 JSON：托管键还停在导入那天的样子，另有一个不属于管理器的键。 */
const IMPORTED_STALE_JSON = `${JSON.stringify({
  app_type: "claude",
  env: {
    ANTHROPIC_BASE_URL: "https://api.toporeduce.cn",
    ANTHROPIC_AUTH_TOKEN: "sk-the-old-key",
    ANTHROPIC_MODEL: "claude-fable-5",
    ANTHROPIC_DEFAULT_OPUS_MODEL: "claude-opus-4-8",
    ANTHROPIC_DEFAULT_SONNET_MODEL: "claude-opus-4-6",
    ANTHROPIC_DEFAULT_HAIKU_MODEL: "claude-opus-4-7",
    XR_SOMETHING_MINE: "keep-me",
  },
  localProxyOverrides: { headers: '{"X-Provider": "your-gateway"}', body: "{}" },
  meta: { claudeDesktopMode: "direct", somethingMine: true },
}, null, 2)}\n`;

const MAPPING_ROWS = [
  { role: "sonnet", label: "Sonnet", routeId: "sonnet", displayName: "GLM 5.2", requestModel: "glm-5.2", supports1m: false },
  { role: "opus", label: "Opus", routeId: "opus", displayName: "GLM 5.3", requestModel: "glm-5.3", supports1m: false },
  { role: "haiku", label: "Haiku", routeId: "haiku", displayName: "DeepSeek Flash", requestModel: "deepseek-v4.1-flash-1011", supports1m: false },
  { role: "subagent", label: "Subagent", routeId: "subagent", displayName: "DeepSeek Pro", requestModel: "deepseek-v4-pro-0813", supports1m: false },
] as const;

describe("supplierEffectiveConfigContents", () => {
  it("rewrites the managed keys of an imported profile and keeps everything else", () => {
    const effective = supplierEffectiveConfigContents(profile({
      configContents: IMPORTED_STALE_JSON,
      modelMappingEnabled: true,
      modelMappingJson: JSON.stringify(MAPPING_ROWS),
    }));
    const parsed = JSON.parse(effective) as {
      env: Record<string, string>;
      localProxyOverrides: { headers: string };
      meta: Record<string, unknown>;
    };

    // 表单上的值到了该在的地方……
    expect(parsed.env.ANTHROPIC_BASE_URL).toBe("https://api.toporeduce.cn");
    expect(parsed.env.ANTHROPIC_AUTH_TOKEN).toBe("sk-the-new-key");
    expect(parsed.env.ANTHROPIC_MODEL).toBe("claude-fable-5");
    expect(parsed.env.ANTHROPIC_DEFAULT_SONNET_MODEL).toBe("glm-5.2");
    expect(parsed.env.ANTHROPIC_DEFAULT_OPUS_MODEL).toBe("glm-5.3");
    expect(parsed.env.ANTHROPIC_DEFAULT_HAIKU_MODEL).toBe("deepseek-v4.1-flash-1011");
    expect(parsed.env.CLAUDE_CODE_SUBAGENT_MODEL).toBe("deepseek-v4-pro-0813");
    // ……而不再留下上一个角色的旧值，也不留下旧凭据。
    expect(JSON.stringify(parsed.env)).not.toContain("claude-opus-4-8");
    expect(JSON.stringify(parsed.env)).not.toContain("sk-the-old-key");
    // 不属于管理器的键照旧保留。
    expect(parsed.env.XR_SOMETHING_MINE).toBe("keep-me");
    expect(parsed.localProxyOverrides.headers).toContain("your-gateway");
    expect(parsed.meta.somethingMine).toBe(true);
    // 有映射就是在走本地代理。
    expect(parsed.meta.claudeDesktopMode).toBe("proxy");
  });

  it("builds the same JSON the save path would write when there is nothing stored yet", () => {
    const current = profile({ modelMappingEnabled: false });
    expect(supplierEffectiveConfigContents(current))
      .toBe(withSupplierGeneratedFiles(current).configContents);
  });

  it("rewrites the managed keys of a Codex profile and keeps the rows the form does not own", () => {
    const stored = [
      'model = "gpt-6.1-sol"',
      'model_provider = "gpt"',
      'model_reasoning_summary = "auto"',
      "model_context_window = 400000",
      "",
      "[model_providers.gpt]",
      'name = "GPT"',
      'base_url = "https://api.old.example/v1"',
      'wire_api = "responses"',
      'requires_openai_auth = true',
      'env_key = "OPENAI_API_KEY"',
      'http_headers = { "X-Provider" = "your-gateway" }',
      "",
      "[mcp_servers.mine]",
      'command = "mine"',
      "",
    ].join("\n");

    const effective = supplierEffectiveConfigContents(profile({
      targetApp: "codex",
      importSource: "",
      userAgent: "",
      protocol: "responses",
      apiFormat: "openai_responses",
      id: "gpt",
      name: "GPT",
      model: "gpt-6-astra",
      baseUrl: "https://api.new.example/v1",
      contextWindow: "1000000",
      configContents: stored,
    }));

    // 表单说的算：模型、上下文窗口、地址都换成新的。
    expect(effective).toContain('model = "gpt-6-astra"');
    expect(effective).toContain('model_provider = "gpt"');
    expect(effective).toContain("model_context_window = 1000000");
    expect(effective).toContain('base_url = "https://api.new.example/v1"');
    expect(effective).not.toContain("gpt-6.1-sol");
    expect(effective).not.toContain("api.old.example");
    // 不属于表单的行照旧保留（用户自己加的键与表）。
    expect(effective).toContain('model_reasoning_summary = "auto"');
    expect(effective).toContain('http_headers = { "X-Provider" = "your-gateway" }');
    expect(effective).toContain("[mcp_servers.mine]");
    expect(effective).toContain('command = "mine"');
  });

  it("writes the local proxy address once the route takes over", () => {
    const routed = profile({
      targetApp: "codex",
      importSource: "",
      userAgent: "",
      protocol: "chatCompletions",
      apiFormat: "openai_chat",
      id: "gpt",
      name: "GPT",
      model: "gpt-6-astra",
      baseUrl: "https://api.new.example/v1",
      routeEnabled: true,
      configContents: "",
    });
    expect(supplierEffectiveConfigContents(routed)).toContain('base_url = "http://127.0.0.1:57321/v1"');
    // 原生 Responses 的上游：开了路由也直连它，代理只为「格式不一样」而存在。
    for (const routeEnabled of [false, true]) {
      expect(supplierEffectiveConfigContents(profile({
        targetApp: "codex",
        importSource: "",
        userAgent: "",
        protocol: "responses",
        apiFormat: "openai_responses",
        baseUrl: "https://api.new.example/v1",
        routeEnabled,
        configContents: "",
      }))).toContain('base_url = "https://api.new.example/v1"');
    }
  });

  it("builds the same TOML the save path would write when there is nothing stored yet", () => {
    const current = profile({
      targetApp: "codex",
      importSource: "",
      userAgent: "",
      protocol: "responses",
      apiFormat: "openai_responses",
    });
    expect(supplierEffectiveConfigContents(current))
      .toBe(withSupplierGeneratedFiles(current).configContents);
  });
});