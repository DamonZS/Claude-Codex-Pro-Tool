import { invoke as tauriInvoke } from "@tauri-apps/api/core";

import announcementConfig from "../../../assets/config/announcement.json";

type Status = "ok" | "failed" | "not_checked" | string;

type CommandResult<T extends Record<string, unknown>> = T & {
  status: Status;
  message: string;
};

type PreviewPluginItem = {
  id: string;
  name: string;
  description: string;
  sourceId: string;
  sourceLabel: string;
  sourceUrl: string;
  category: string;
  author: string;
  homepage: string;
  license: string;
  tags: string[];
  installKind: string;
  installStatus: string;
  installCommand: string[];
  configPreview: string;
  risk: string;
  requirements: string[];
};

type PreviewMemoryItem = {
  id: string;
  text: string;
  workspace: string;
  category: string;
  tags: string[];
  source: string;
  sourceSessionId: string;
  createdAt: number;
  updatedAt: number;
  lastAccessedAt: number;
  accessCount: number;
  tier: string;
  strength: number;
  archivedAt: number;
  retention: number;
  exempt: boolean;
};

type PreviewMemoryCandidate = {
  id: string;
  text: string;
  workspace: string;
  category: string;
  tags: string[];
  source: string;
  reason: string;
  sourceSessionId: string;
  status: string;
  createdAt: number;
  updatedAt: number;
};

const now = () => Date.now();

function previewSettings() {
  return {
    codexAppPath: "D:\\Project\\Claude-Codex-Pro-Tool\\target\\debug\\claude-codex-pro-manager.exe",
    codexExtraArgs: [],
    providerSyncEnabled: true,
    providerSyncSavedProviders: [],
    providerSyncManualProviders: [],
    providerSyncLastSelectedProvider: "preview",
    relayProfilesEnabled: true,
    enhancementsEnabled: true,
    computerUseGuardEnabled: true,
    codexAppPluginEntryUnlock: true,
    codexAppPluginMarketplaceUnlock: true,
    codexAppForcePluginInstall: false,
    codexAppModelWhitelistUnlock: true,
    codexAppSessionDelete: true,
    codexAppMarkdownExport: true,
    codexAppProjectMove: true,
    codexAppConversationTimeline: true,
    codexAppConversationView: true,
    codexAppThreadScrollRestore: true,
    codexAppZedRemoteOpen: false,
    zedRemoteOpenStrategy: "native",
    zedRemoteProjectRegistryEnabled: false,
    zedRemoteSyncToZedSettings: false,
    codexAppUpstreamWorktreeCreate: false,
    codexAppNativeMenuPlacement: true,
    claudeAppChineseOverlayEnabled: true,
    codexAppServiceTierControls: true,
    codexAppImageOverlayEnabled: false,
    codexAppImageOverlayPath: "",
    codexAppImageOverlayOpacity: 70,
    codexGoalsEnabled: true,
    memoryAssistEnabled: true,
    memoryAssistInjectEnabled: true,
    memoryAssistAutoSuggestEnabled: true,
    memoryAssistLlmSummaryEnabled: false,
    memoryAssistMcpEnabled: false,
    memoryAssistMaxInjectedItems: 5,
    memoryAssistWorkspaceMode: "project_plus_global",
    launchMode: "patch",
    relayBaseUrl: "",
    relayApiKey: "",
    relayProfiles: [{
      id: "default",
      name: "默认中转",
      protocol: "responses",
      relayMode: "official",
      officialMixApiKey: false,
      testModel: "",
      configContents: "",
      authContents: "",
      useCommonConfig: true,
      contextSelection: { mcpServers: [], skills: [], plugins: [] },
      contextSelectionInitialized: false,
      contextWindow: "",
      autoCompactLimit: "",
      modelList: "",
      userAgent: "",
    }],
    relayCommonConfigContents: "",
    relayContextConfigContents: "",
    activeRelayId: "default",
    relayTestModel: "gpt-5",
    cliWrapperEnabled: false,
    cliWrapperBaseUrl: "",
    cliWrapperApiKey: "",
    cliWrapperApiKeyEnv: "OPENAI_API_KEY",
  };
}

function previewSettingsResult(message = "预览模式设置。", settings = previewSettings()) {
  return ok(message, {
    settings,
    settings_path: "~\\.claude-codex-pro\\settings.json",
    user_scripts: { enabled: true, scripts: previewUserScripts() },
  });
}

function previewUserScripts() {
  return [
    {
      key: "preview-toolbar",
      name: "预览工具栏增强",
      version: "1.0.0",
      description: "预览模式本地脚本示例。",
      enabled: true,
      path: "~\\.claude-codex-pro\\scripts\\preview-toolbar.js",
      homepage: "https://github.com/DamonZS/Claude-Codex-Pro-ToolScriptMarket",
    },
  ];
}

function previewPluginItems(): PreviewPluginItem[] {
  return [
    {
      id: "official-files",
      name: "Files",
      description: "Claude 官方文件能力插件示例。",
      sourceId: "official",
      sourceLabel: "Claude 官方插件",
      sourceUrl: "https://github.com/anthropics/claude-plugins-official",
      category: "claude",
      author: "Anthropic",
      homepage: "https://github.com/anthropics/claude-plugins-official",
      license: "unknown",
      tags: ["official", "claude"],
      installKind: "claude_plugin_marketplace",
      installStatus: "notInstalled",
      installCommand: ["claude", "plugin", "marketplace", "install", "files"],
      configPreview: "",
      risk: "安装前应查看官方 marketplace 命令。",
      requirements: ["claude CLI"],
    },
    {
      id: "codex-github",
      name: "Codex GitHub Tools",
      description: "Codex 插件仓库中的 GitHub 工作流能力示例。",
      sourceId: "codex-plugins",
      sourceLabel: "Codex 插件仓库",
      sourceUrl: "https://github.com/openai/plugins",
      category: "codex",
      author: "OpenAI",
      homepage: "https://github.com/openai/plugins",
      license: "MIT",
      tags: ["codex", "github"],
      installKind: "resource_link",
      installStatus: "needsReview",
      installCommand: [],
      configPreview: "",
      risk: "社区/示例资源仅展示元数据，不自动执行脚本。",
      requirements: ["manual review"],
    },
    {
      id: "mcp-filesystem",
      name: "Filesystem MCP",
      description: "可审查安装到 Claude Desktop 的 MCP 文件系统能力。",
      sourceId: "mcp",
      sourceLabel: "GitHub MCP Registry",
      sourceUrl: "https://github.com/mcp",
      category: "mcp",
      author: "Community",
      homepage: "https://github.com/mcp",
      license: "unknown",
      tags: ["mcp", "filesystem"],
      installKind: "claude_desktop_mcp",
      installStatus: "notInstalled",
      installCommand: [],
      configPreview: "{\n  \"mcpServers\": {}\n}",
      risk: "安装前展示配置 diff，并写入前备份。",
      requirements: ["Claude Desktop"],
    },
    {
      id: "ponytail:codex-plugin",
      name: "Ponytail for Codex",
      description: "Ponytail lazy senior dev 模式，添加到 Codex 插件 marketplace 后在 /plugins 中安装并审查 hooks。",
      sourceId: "ponytail",
      sourceLabel: "Ponytail 多工具插件",
      sourceUrl: "https://github.com/DietrichGebert/ponytail",
      category: "codex-plugin",
      author: "Dietrich Gebert",
      homepage: "https://github.com/DietrichGebert/ponytail",
      license: "MIT",
      tags: ["ponytail", "codex", "plugin", "skills", "hooks"],
      installKind: "codex_plugin",
      installStatus: "notInstalled",
      installCommand: ["codex", "plugin", "marketplace", "add", "DietrichGebert/ponytail", "--json"],
      configPreview: "codex plugin marketplace add DietrichGebert/ponytail --json\ncodex plugin list --available --json\ncodex plugin add ponytail@ponytail --json\n\n安装后单独审查并信任 hooks。",
      risk: "真实安装会调用 Codex CLI；不会后台静默信任第三方 hooks。",
      requirements: ["codex CLI", "Node.js", "hooks 单独确认"],
    },
    {
      id: "ponytail:claude-code-plugin",
      name: "Ponytail for Claude Code",
      description: "Ponytail lazy senior dev 模式，安装到 Claude Code 插件市场。",
      sourceId: "ponytail",
      sourceLabel: "Ponytail 多工具插件",
      sourceUrl: "https://github.com/DietrichGebert/ponytail",
      category: "claude-code-plugin",
      author: "Dietrich Gebert",
      homepage: "https://github.com/DietrichGebert/ponytail",
      license: "MIT",
      tags: ["ponytail", "claude-code", "plugin", "skills", "hooks"],
      installKind: "claude_code_plugin",
      installStatus: "notInstalled",
      installCommand: ["claude", "plugin", "marketplace", "add", "DietrichGebert/ponytail"],
      configPreview: "/plugin marketplace add DietrichGebert/ponytail\n/plugin install ponytail@ponytail",
      risk: "真实安装会调用 Claude Code CLI；安装后请审查并信任 hooks。",
      requirements: ["claude CLI", "Node.js"],
    },
    {
      id: "ponytail:copilot-plugin",
      name: "Ponytail for GitHub Copilot CLI",
      description: "Ponytail lazy senior dev 模式，安装到 GitHub Copilot CLI 插件系统。",
      sourceId: "ponytail",
      sourceLabel: "Ponytail 多工具插件",
      sourceUrl: "https://github.com/DietrichGebert/ponytail",
      category: "copilot-plugin",
      author: "Dietrich Gebert",
      homepage: "https://github.com/DietrichGebert/ponytail",
      license: "MIT",
      tags: ["ponytail", "copilot", "plugin"],
      installKind: "copilot_plugin",
      installStatus: "notInstalled",
      installCommand: ["copilot", "plugin", "marketplace", "add", "DietrichGebert/ponytail"],
      configPreview: "copilot plugin marketplace add DietrichGebert/ponytail\ncopilot plugin install ponytail@ponytail",
      risk: "真实安装会调用 GitHub Copilot CLI；CLI 未登录时会返回错误输出。",
      requirements: ["copilot CLI", "Node.js"],
    },
    {
      id: "ponytail:claude-desktop-mcp",
      name: "Ponytail MCP for Claude Desktop",
      description: "把 Ponytail MCP server 注册到 Claude Desktop mcpServers。",
      sourceId: "ponytail",
      sourceLabel: "Ponytail 多工具插件",
      sourceUrl: "https://github.com/DietrichGebert/ponytail",
      category: "claude-desktop-mcp",
      author: "Dietrich Gebert",
      homepage: "https://github.com/DietrichGebert/ponytail",
      license: "MIT",
      tags: ["ponytail", "mcp", "claude-desktop"],
      installKind: "claude_desktop_mcp",
      installStatus: "notInstalled",
      installCommand: ["node", "~\\.claude-codex-pro\\plugin-hub\\repos\\ponytail\\ponytail-mcp\\index.js"],
      configPreview: "{\n  \"mcpServers\": {\n    \"ponytail\": { \"command\": \"node\", \"args\": [\"~\\\\.claude-codex-pro\\\\plugin-hub\\\\repos\\\\ponytail\\\\ponytail-mcp\\\\index.js\"] }\n  }\n}",
      risk: "真实安装会克隆 Ponytail、安装 MCP 依赖、备份 Claude Desktop 配置后写入。",
      requirements: ["Git", "Node.js", "Claude Desktop"],
    },
    {
      id: "ponytail:claude-desktop-org-plugin",
      name: "Ponytail Organization Plugin for Claude Desktop",
      description: "安装为 Claude Desktop 开发模式可读取的组织插件目录。",
      sourceId: "ponytail",
      sourceLabel: "Ponytail 多工具插件",
      sourceUrl: "https://github.com/DietrichGebert/ponytail",
      category: "claude-desktop-org-plugin",
      author: "Dietrich Gebert",
      homepage: "https://github.com/DietrichGebert/ponytail",
      license: "MIT",
      tags: ["ponytail", "claude-desktop", "organization-plugin", "skills"],
      installKind: "claude_desktop_org_plugin",
      installStatus: "notInstalled",
      installCommand: [],
      configPreview: "源：~\\.claude-codex-pro\\plugin-hub\\repos\\ponytail\\skills\\*\n目标：C:\\Program Files\\Claude\\org-plugins\\ponytail\\skills\\*",
      risk: "真实安装会写入 Claude Desktop 组织插件目录；普通权限不可写时会失败，不会调用 Claude CLI 登录，也不会静默信任 hooks。",
      requirements: ["Claude Desktop 3P / 开发模式", "Git", "管理员权限或可写目录", "本地写入 MCP/skills/组织插件目录"],
    },
    {
      id: "ponytail:codex-skills",
      name: "Ponytail Skills for Codex",
      description: "复制 Ponytail skills 到 Codex 用户技能目录。",
      sourceId: "ponytail",
      sourceLabel: "Ponytail 多工具插件",
      sourceUrl: "https://github.com/DietrichGebert/ponytail",
      category: "codex-skills",
      author: "Dietrich Gebert",
      homepage: "https://github.com/DietrichGebert/ponytail",
      license: "MIT",
      tags: ["ponytail", "codex", "skills"],
      installKind: "managed_skill_bundle",
      installStatus: "notInstalled",
      installCommand: [],
      configPreview: "源：~\\.claude-codex-pro\\plugin-hub\\repos\\ponytail\\skills\\*\n目标：~\\.codex\\skills\\*",
      risk: "真实安装会克隆 Ponytail，并在覆盖同名 skill 前备份。",
      requirements: ["Git", "Codex skills 目录"],
    },
  ];
}

function previewPluginCatalog(message = "预览模式插件目录。") {
  return ok(message, {
    catalog: {
      updatedAt: new Date().toISOString(),
      sources: [
        { id: "official", label: "Claude 官方插件", url: "https://github.com/anthropics/claude-plugins-official", status: "ok", message: "预览数据", itemCount: 2 },
        { id: "codex-plugins", label: "Codex 插件仓库", url: "https://github.com/openai/plugins", status: "ok", message: "预览数据", itemCount: 2 },
        { id: "ponytail", label: "Ponytail 多工具插件", url: "https://github.com/DietrichGebert/ponytail", status: "ok", message: "预览数据", itemCount: 6 },
        { id: "awesome", label: "awesome-claude-code", url: "https://github.com/hesreallyhim/awesome-claude-code", status: "ok", message: "预览数据", itemCount: 1 },
      ],
      items: previewPluginItems(),
    },
  });
}

function previewCodexPluginMarketplace(message = "预览模式 Codex OpenAI 插件仓库状态。") {
  return ok(message, {
    marketplace: {
      codexHome: "~\\.codex",
      marketplaceRoot: "~\\.codex\\.tmp\\plugins",
      configRegistered: true,
      needsRepair: false,
      localSourcesReady: true,
      runtimeConfirmation: "预览模式：本地来源已模拟就绪，待应用确认。",
      message: "预览模式：本地 openai-curated 与第三方 marketplace 已模拟注册。",
      repositories: [
        {
          label: "OpenAI 官方仓库",
          name: "openai-curated + openai-api-curated",
          sourceType: "local",
          source: "~\\.codex\\.tmp\\plugins",
          configured: true,
        },
        {
          label: "第三方插件仓库",
          name: "awesome-codex-plugins",
          sourceType: "git",
          source: "https://github.com/hashgraph-online/awesome-codex-plugins.git",
          configured: true,
        },
        {
          label: "Product Design Skill 仓库",
          name: "codex-skills-alternative",
          sourceType: "local",
          source: "~\\.codex\\plugins\\cache\\codex-skills-alternative-marketplace",
          configured: true,
        },
      ],
    },
  });
}

function previewPluginItem(id?: unknown) {
  const requested = typeof id === "string" ? id : "official-files";
  return previewPluginItems().find((item) => item.id === requested) ?? previewPluginItems()[0];
}

function previewMemoryItems(): PreviewMemoryItem[] {
  const stamp = now();
  return [
    {
      id: "preview-memory-1",
      text: "大改前先备份到 F:\\项目代码备份\\Claude-Codex-Pro-Tool-backup。",
      workspace: "global",
      category: "project",
      tags: ["backup"],
      source: "manager",
      sourceSessionId: "",
      createdAt: stamp,
      updatedAt: stamp,
      lastAccessedAt: stamp,
      accessCount: 5,
      tier: "active",
      strength: 1.0,
      archivedAt: 0,
      retention: 1.0,
      exempt: true,
    },
  ];
}

function previewMemoryCandidates(): PreviewMemoryCandidate[] {
  const stamp = now();
  return [
    {
      id: "preview-candidate-1",
      text: "新版前端采用 Linear 风格深色运维控制台。",
      workspace: "global",
      category: "ui",
      tags: ["linear"],
      source: "preview",
      reason: "用户明确选择此设计方向",
      sourceSessionId: "",
      status: "pending",
      createdAt: stamp,
      updatedAt: stamp,
    },
  ];
}

function previewMemoryStatus(message = "预览模式盘古记忆状态。") {
  return ok(message, {
    memory: {
      status: "ok",
      dbPath: "~\\.claude-codex-pro\\memory_assist.sqlite",
      totalItems: 12,
      pendingCandidates: 3,
      workspaces: [{ workspace: "global", itemCount: 4, pendingCount: 1 }],
      latestBackupPath: "~\\.claude-codex-pro\\backups\\memory-preview.json",
      enabled: true,
      injectEnabled: true,
      autoSuggestEnabled: true,
      runtimeStatus: "ok",
      runtimeMessage: "预览模式：盘古记忆正在监听 Codex 对话。",
      codexInjected: true,
      claudeInjected: false,
      codexWorkspace: "codex:repo:D:\\Project\\Claude-Codex-Pro-Tool",
      active: true,
      activeSource: "stream",
    },
  });
}

function previewScriptMarket(message = "预览模式脚本市场。") {
  return ok(message, {
    market: {
      status: "ok",
      message: "预览脚本目录已加载。",
      indexUrl: "https://github.com/DamonZS/Claude-Codex-Pro-ToolScriptMarket",
      updatedAt: new Date().toISOString(),
      scripts: [
        {
          id: "preview-toolbar",
          name: "Codex 工具栏增强",
          description: "预览模式脚本市场条目，用于验证安装按钮和来源按钮。",
          version: "1.0.0",
          author: "Claude Codex Pro",
          tags: ["codex", "ui"],
          homepage: "https://github.com/DamonZS/Claude-Codex-Pro-ToolScriptMarket",
          script_url: "https://example.invalid/preview-toolbar.js",
          sha256: "preview",
          installed: false,
          installedVersion: "",
          updateAvailable: false,
        },
      ],
    },
    user_scripts: { enabled: true, scripts: previewUserScripts() },
  });
}

function previewContextEntries(settings = previewSettings()) {
  const entries = {
    mcpServers: [
      {
        id: "codegraph",
        kind: "mcp",
        title: "codegraph",
        summary: "command = \"codegraph\"",
        tomlBody: "enabled = true\ntype = \"stdio\"\ncommand = \"codegraph\"\nargs = [\"serve\", \"--mcp\"]\n",
        enabled: true,
      },
      {
        id: "node_repl",
        kind: "mcp",
        title: "node_repl",
        summary: "command = \"node\"",
        tomlBody: "enabled = true\ntype = \"stdio\"\ncommand = \"node\"\nargs = [\"server.js\"]\n",
        enabled: true,
      },
    ],
    skills: [],
    plugins: previewPluginItems().slice(0, 20).map((item) => ({
      id: `${item.name.toLowerCase().replace(/[^a-z0-9]+/g, "-")}@${item.sourceId}`,
      kind: "plugin",
      title: `${item.name}@${item.sourceId}`,
      summary: item.description,
      tomlBody: "enabled = true\n",
      enabled: true,
    })),
  };
  return ok("预览模式工具与插件列表。", { settings, entries });
}

function previewClaudeContextEntries() {
  return ok("预览模式 Claude 工具与插件列表。", {
    configPath: "~\\AppData\\Roaming\\Claude\\claude_desktop_config.json",
    entries: {
      mcpServers: [
        {
          id: "claude-codex-pro-codex",
          kind: "mcp",
          title: "claude-codex-pro-codex",
          summary: "claude-codex-pro mcp",
          tomlBody: "{\n  \"command\": \"claude-codex-pro\",\n  \"args\": [\"mcp\"],\n  \"enabled\": true\n}\n",
          enabled: true,
        },
      ],
      skills: [
        {
          id: "ponytail",
          kind: "skill",
          title: "Ponytail Skills",
          summary: "预览模式组织插件已写入",
          tomlBody: "{\n  \"enabled\": true\n}\n",
          enabled: true,
        },
      ],
      plugins: [
        {
          id: "ponytail",
          kind: "plugin",
          title: "DietrichGebert/ponytail",
          summary: "预览模式 Claude 官方插件入口",
          tomlBody: "{\n  \"enabled\": true\n}\n",
          enabled: true,
        },
      ],
    },
  });
}

type PreviewUnifiedToolAssetState = {
  id: string;
  kind: "mcp" | "skill" | "plugin";
  title: string;
  summary: string;
  claudeEnabled: boolean;
  codexEnabled: boolean;
};

const previewUnifiedToolAssets: PreviewUnifiedToolAssetState[] = [
  { id: "codebase-memory-mcp", kind: "mcp", title: "codebase-memory-mcp", summary: "代码库知识图谱 MCP", claudeEnabled: true, codexEnabled: true },
  { id: "ask-matt", kind: "skill", title: "ask-matt", summary: "工程工作流路由技能", claudeEnabled: true, codexEnabled: true },
  { id: "github", kind: "plugin", title: "github", summary: "GitHub 工作流插件", claudeEnabled: false, codexEnabled: true },
];

function previewUnifiedToolInventory(message = "检测完成：已加载预览工具与插件。") {
  const assets = previewUnifiedToolAssets.map((item) => ({
    id: item.id,
    kind: item.kind,
    title: item.title,
    summary: item.summary,
    source: `~\\.claude\\${item.kind}s\\${item.id} | ~\\.codex\\${item.kind}s\\${item.id}`,
    claude: { enabled: item.claudeEnabled, available: true, toggleSupported: true, sourcePath: `~\\.claude\\${item.kind}s\\${item.id}` },
    codex: { enabled: item.codexEnabled, available: true, toggleSupported: true, sourcePath: `~\\.codex\\${item.kind}s\\${item.id}` },
  }));
  return ok(message, {
    inventory: {
      assets,
      counts: {
        total: assets.length,
        rawDiscoveries: 5,
        deduplicated: 2,
        mcp: assets.filter((item) => item.kind === "mcp").length,
        skills: assets.filter((item) => item.kind === "skill").length,
        plugins: assets.filter((item) => item.kind === "plugin").length,
        codexEnabled: assets.filter((item) => item.codex.enabled).length,
        claudeEnabled: assets.filter((item) => item.claude.enabled).length,
      },
      scannedSources: ["~\\.codex", "~\\.claude"],
      diagnostics: [],
    },
  });
}

const hasTauriInternals = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function invokeCommand<T>(command: string, args?: Record<string, unknown>) {
  if (hasTauriInternals()) return tauriInvoke<T>(command, args);
  return mockInvoke(command, args) as Promise<T>;
}

async function mockInvoke(command: string, _args?: Record<string, unknown>) {
  if (command === "open_external_url") return ok("预览模式不打开外部链接。", {});
  if (command === "launch_claude_codex_pro" || command === "restart_claude_codex_pro") {
    return ok(command === "launch_claude_codex_pro" ? "预览模式已模拟启动/重启 Codex。" : "预览模式已模拟重启 Codex。", {
      preview: true,
      action: command,
      startedAtMs: Date.now(),
    });
  }
  if (command === "open_claude_desktop") return ok("预览模式已模拟启动官方 Claude。", { preview: true });
  if (command === "load_overview") {
    return ok("预览模式已加载概览。", {
      codex_app: { status: "found", path: "D:\\Project\\Claude-Codex-Pro-Tool\\target\\debug\\claude-codex-pro-manager.exe" },
      codex_version: "preview",
      silent_shortcut: { status: "not_checked", path: null },
      management_shortcut: { status: "installed", path: "Desktop\\Claude Code Pro.lnk" },
      latest_launch: {
        status: "running",
        message: "preview bridge",
        started_at_ms: Date.now(),
        debug_port: 57321,
        helper_port: 57322,
        debug_port_online: true,
        helper_port_online: true,
        frontend_runtime_online: true,
        frontend_runtime_seen_at_ms: Date.now(),
        codex_app: "preview",
      },
      current_version: "V0.12",
      update_status: "preview",
      settings_path: "~\\.claude-codex-pro\\settings.json",
      logs_path: "~\\.claude-codex-pro\\logs\\manager.log",
    });
  }
  if (command === "load_ads") {
    return ok("预览模式已加载公告。", {
      version: announcementConfig.version,
      ads: announcementConfig.ads,
    });
  }
  if (command === "load_claude_desktop_status" || command === "load_claude_desktop_status_light") {
    return ok("预览模式 Claude 诊断。", {
      processCount: 0,
      executablePaths: [],
      installKind: "msix",
      cdpStatus: "blocked",
      frontendInjected: false,
      frontendStatus: "not_available",
      cdpBlocker: "官方 MSIX 窗口不可直接 DOM 注入",
      debugFlagsPresent: false,
      debugPorts: [],
      inspectorPorts: [],
      listeningPorts: [],
      debugEvidence: [],
      supportedIntegration: "wrapped_webview",
      integrityStatus: "not_modified",
      integrityMessage: "预览模式不修改官方 Claude。",
      executableAudits: [],
    });
  }
  if (command === "load_claude_chinese_window_status" || command === "open_claude_chinese_window") {
    return ok("预览模式 Claude 一键汉化状态。", {
      open: command === "open_claude_chinese_window",
      label: "Claude 一键汉化",
      defaultUrl: "https://claude.ai/new",
      injectionMode: "wrapped_webview",
      cdpStatus: "blocked",
      cdpBlocker: "官方 Claude Desktop 使用包装窗口替代注入",
      officialInstallKind: "msix",
    });
  }
  if (command === "load_claude_zh_patch_status") {
    return ok("预览模式本机汉化状态。", {
      status: {
        status: "not_checked",
        message: "预览模式不修改本机 Claude 文件。",
        installRoot: null,
        appRoot: null,
        installKind: "unknown",
        localeConfigPath: "~\\AppData\\Roaming\\Claude\\locale.json",
        backupDir: "~\\.claude-codex-pro\\claude-zh-backups",
        resourcesPresent: false,
        frontendI18nPresent: false,
        statsigI18nPresent: false,
        chunkPatchPresent: false,
        languageWhitelistPatched: false,
        localeConfigured: false,
        writable: false,
      },
      changedFiles: [],
      backupDir: "~\\.claude-codex-pro\\claude-zh-backups",
    });
  }
  if (command === "install_claude_zh_patch" || command === "install_claude_zh_patch_at_install_root" || command === "restore_claude_zh_patch") {
    const isInstall = command === "install_claude_zh_patch" || command === "install_claude_zh_patch_at_install_root";
    return ok(isInstall ? "预览模式已模拟 Claude 本机汉化。" : "预览模式已模拟恢复 Claude 官方文件。", {
      status: {
        status: "ok",
        message: "预览模式不会修改本机 Claude 文件。",
        installRoot: typeof _args?.installRoot === "string" ? _args.installRoot : null,
        appRoot: null,
        installKind: "unknown",
        localeConfigPath: "~\\AppData\\Roaming\\Claude\\locale.json",
        backupDir: "~\\.claude-codex-pro\\claude-zh-backups",
        resourcesPresent: true,
        frontendI18nPresent: isInstall,
        statsigI18nPresent: isInstall,
        chunkPatchPresent: isInstall,
        languageWhitelistPatched: isInstall,
        localeConfigured: isInstall,
        writable: true,
      },
      changedFiles: [],
      backupDir: "~\\.claude-codex-pro\\claude-zh-backups",
    });
  }
  if (command === "refresh_plugin_hub_catalog" || command === "get_plugin_hub_catalog") {
    return previewPluginCatalog();
  }
  if (command === "preview_plugin_hub_install") {
    const item = previewPluginItem((_args?.request as { id?: unknown } | undefined)?.id);
    const canInstall = ["claude_desktop_mcp", "claude_desktop_org_plugin", "claude_plugin_marketplace", "claude_code_plugin", "codex_plugin", "copilot_plugin", "managed_skill_bundle"].includes(item.installKind);
    return ok("预览模式安装预览。", {
      item,
      canInstall,
      action: item.installKind === "codex_plugin" ? "codex_cli_plugin" : item.installKind,
      command: item.installCommand,
      configDiff: item.configPreview || "",
      message: item.installKind === "codex_plugin" ? "真实环境会调用 Codex CLI 安装；hooks 需要单独审查后信任。" : item.installCommand.length ? `将执行：${item.installCommand.join(" ")}` : "该资源需要人工审查，预览模式不执行安装。",
    });
  }
  if (command === "install_plugin_hub_item") {
    const item = { ...previewPluginItem((_args?.request as { id?: unknown } | undefined)?.id), installStatus: "installed" };
    return ok("预览模式已模拟插件安装。", {
      item,
      preview: {},
      installed: true,
      installMessage: "预览模式不会写入 Claude Desktop 或执行 CLI。",
      stdout: "",
      stderr: "",
      backupPath: "~\\.claude-codex-pro\\backups\\plugin-preview.json",
    });
  }
  if (command === "uninstall_plugin_hub_item") {
    return previewPluginCatalog("预览模式已模拟撤销托管配置并更新插件记录。");
  }
  if (command === "preview_ponytail_codex_hooks") {
    return ok("预览模式已生成 Ponytail Codex hooks 审查列表。", {
      preview: {
        configPath: "~\\.codex\\config.toml",
        hooks: [
          {
            key: "ponytail@ponytail:hooks/claude-codex-hooks.json:session_start:0:0",
            eventName: "session_start",
            matcher: "startup|resume|clear|compact",
            command: "node ponytail-activate.js",
            statusMessage: "Loading ponytail mode...",
            currentHash: "sha256:preview",
            trusted: false,
            sourcePath: "~\\.codex\\plugins\\cache\\ponytail\\hooks\\claude-codex-hooks.json",
          },
        ],
        message: "预览模式：发现 1 个待信任 hook。",
      },
    });
  }
  if (command === "trust_ponytail_codex_hooks") {
    return ok("预览模式已模拟写入 hooks.state。", {
      preview: {
        configPath: "~\\.codex\\config.toml",
        hooks: [],
        message: "预览模式不会修改真实 Codex 配置。",
      },
    });
  }
  if (command === "generate_ponytail_mcpb_installer") {
    return ok("预览模式已模拟生成并打开 Ponytail MCPB。", {
      package: {
        mcpbPath: "~\\.claude-codex-pro\\plugin-hub\\mcpb\\ponytail-preview.mcpb",
        manifestPath: "~\\.claude-codex-pro\\plugin-hub\\mcpb\\ponytail-preview\\manifest.json",
        opened: true,
        message: "预览模式不会打开系统安装弹窗。",
      },
    });
  }
  if (command === "load_claude_desktop_org_plugin_status" || command === "open_claude_desktop_org_plugins_dir") {
    return ok("预览模式 Claude Desktop 组织插件目录可用。", {
      orgPluginStatus: {
        supported: true,
        orgPluginsDir: "C:\\Program Files\\Claude\\org-plugins",
        configLibraryDir: "~\\AppData\\Local\\Claude-3p\\configLibrary",
        profileMetaPath: "~\\AppData\\Local\\Claude-3p\\configLibrary\\_meta.json",
        ponytailPluginDir: "C:\\Program Files\\Claude\\org-plugins\\ponytail",
        ponytailInstalled: false,
        writable: true,
        message: "预览模式不会写入 Program Files。",
      },
    });
  }
  if (command === "load_claude_desktop_marketplace_status") {
    return ok("预览模式 Claude Desktop 插件仓库配置可用。", {
      marketplaceStatus: {
        supported: true,
        marketplace: "anthropics/claude-plugins-official, DietrichGebert/ponytail",
        plugin: "ponytail",
        deepLink: "claude://claude.ai/customize/plugins/new?marketplace=DietrichGebert%2Fponytail&plugin=ponytail",
        canAutoWrite: true,
        configPath: "~\\AppData\\Local\\Claude-3p\\claude_desktop_config.json",
        repositories: [
          { label: "Claude 官方插件仓库", repository: "anthropics/claude-plugins-official", url: "https://github.com/anthropics/claude-plugins-official", configured: true },
          { label: "Ponytail 插件仓库", repository: "DietrichGebert/ponytail", url: "https://github.com/DietrichGebert/ponytail", configured: true },
        ],
        message: "预览模式：插件仓库已模拟写入 Claude-3p 开发配置。",
      },
    });
  }
  if (command === "load_codex_plugin_marketplace_status") {
    return previewCodexPluginMarketplace();
  }
  if (command === "repair_codex_plugin_marketplace") {
    return ok("预览模式已模拟下载、校验并注册 Codex OpenAI 插件仓库。", {
      repair: {
        codexHome: "~\\.codex",
        marketplaceRoot: "~\\.codex\\.tmp\\plugins",
        initialized: true,
        configured: true,
        configRegistered: true,
        needsRepair: false,
        message: "预览模式不会下载 GitHub zip 或写入 config.toml。",
      },
      marketplace: {
        codexHome: "~\\.codex",
        marketplaceRoot: "~\\.codex\\.tmp\\plugins",
        configRegistered: true,
        needsRepair: false,
        localSourcesReady: true,
        runtimeConfirmation: "预览模式：本地来源已模拟就绪，待应用确认。",
        message: "预览模式：本地 openai-curated marketplace 已模拟下载并注册。",
        repositories: [
          {
            label: "OpenAI 官方仓库",
            name: "openai-curated + openai-api-curated",
            sourceType: "local",
            source: "~\\.codex\\.tmp\\plugins",
            configured: true,
          },
          {
            label: "第三方插件仓库",
            name: "awesome-codex-plugins",
            sourceType: "git",
            source: "https://github.com/hashgraph-online/awesome-codex-plugins.git",
            configured: true,
          },
          {
            label: "Product Design Skill 仓库",
            name: "codex-skills-alternative",
            sourceType: "local",
            source: "~\\.codex\\plugins\\cache\\codex-skills-alternative-marketplace",
            configured: true,
          },
        ],
      },
    });
  }
  if (command === "load_claude_desktop_dev_mode_status") {
    return ok("预览模式 Claude Desktop 开发模式未配置。", {
      devModeStatus: {
        supported: true,
        configured: false,
        normalConfigPath: "~\\AppData\\Roaming\\Claude\\claude_desktop_config.json",
        threepConfigPath: "~\\AppData\\Local\\Claude-3p\\claude_desktop_config.json",
        configLibraryDir: "~\\AppData\\Local\\Claude-3p\\configLibrary",
        profileMetaPath: "~\\AppData\\Local\\Claude-3p\\configLibrary\\_meta.json",
        appliedId: null,
        message: "预览模式不会写入 Claude Desktop 配置。",
      },
    });
  }
  if (command === "configure_claude_desktop_dev_mode" || command === "refresh_claude_third_party_config") {
    return ok(command === "refresh_claude_third_party_config" ? "预览模式已模拟刷新 Claude 第三方配置。" : "预览模式已模拟配置 Claude Desktop 开发模式。", {
      outcome: {
        configured: true,
        normalConfigPath: "~\\AppData\\Roaming\\Claude\\claude_desktop_config.json",
        threepConfigPath: "~\\AppData\\Local\\Claude-3p\\claude_desktop_config.json",
        profilePath: "~\\AppData\\Local\\Claude-3p\\configLibrary\\00000000-0000-4000-8000-000000157210.json",
        profileMetaPath: "~\\AppData\\Local\\Claude-3p\\configLibrary\\_meta.json",
        backupPaths: [],
        message: "预览模式不会写入 Claude Desktop 配置。",
      },
      devModeStatus: {
        supported: true,
        configured: true,
        normalConfigPath: "~\\AppData\\Roaming\\Claude\\claude_desktop_config.json",
        threepConfigPath: "~\\AppData\\Local\\Claude-3p\\claude_desktop_config.json",
        configLibraryDir: "~\\AppData\\Local\\Claude-3p\\configLibrary",
        profileMetaPath: "~\\AppData\\Local\\Claude-3p\\configLibrary\\_meta.json",
        appliedId: "00000000-0000-4000-8000-000000157210",
        message: "预览模式：开发模式已模拟配置。",
      },
    });
  }
  if (command === "open_ponytail_claude_desktop_marketplace_setup" || command === "repair_claude_desktop_marketplaces") {
    return ok("预览模式已模拟修复 Claude 插件仓库。", {
      outcome: {
        repaired: true,
        configPath: "~\\AppData\\Local\\Claude-3p\\claude_desktop_config.json",
        repositories: [
          { label: "Claude 官方插件仓库", repository: "anthropics/claude-plugins-official", url: "https://github.com/anthropics/claude-plugins-official", configured: true },
          { label: "Ponytail 插件仓库", repository: "DietrichGebert/ponytail", url: "https://github.com/DietrichGebert/ponytail", configured: true },
        ],
        message: "预览模式不会修改 Claude Desktop；真实环境会写入 extraKnownMarketplaces。",
      },
      marketplaceStatus: {
        supported: true,
        marketplace: "anthropics/claude-plugins-official, DietrichGebert/ponytail",
        plugin: "ponytail",
        deepLink: "claude://claude.ai/customize/plugins/new?marketplace=DietrichGebert%2Fponytail&plugin=ponytail",
        canAutoWrite: true,
        configPath: "~\\AppData\\Local\\Claude-3p\\claude_desktop_config.json",
        repositories: [
          { label: "Claude 官方插件仓库", repository: "anthropics/claude-plugins-official", url: "https://github.com/anthropics/claude-plugins-official", configured: true },
          { label: "Ponytail 插件仓库", repository: "DietrichGebert/ponytail", url: "https://github.com/DietrichGebert/ponytail", configured: true },
        ],
        message: "预览模式：插件仓库已模拟写入 Claude-3p 开发配置。",
      },
    });
  }
  if (command === "install_ponytail_claude_desktop_org_plugin") {
    return ok("预览模式已模拟安装 Ponytail 组织插件。", {
      outcome: {
        installed: true,
        orgPluginsDir: "C:\\Program Files\\Claude\\org-plugins",
        pluginDir: "C:\\Program Files\\Claude\\org-plugins\\ponytail",
        manifestPath: "C:\\Program Files\\Claude\\org-plugins\\ponytail\\manifest.json",
        pluginJsonPath: "C:\\Program Files\\Claude\\org-plugins\\ponytail\\.claude-plugin\\plugin.json",
        copiedSkills: ["C:\\Program Files\\Claude\\org-plugins\\ponytail\\skills\\ponytail"],
        backupPath: null,
        message: "预览模式不会修改 Claude Desktop。",
      },
      orgPluginStatus: {
        supported: true,
        orgPluginsDir: "C:\\Program Files\\Claude\\org-plugins",
        configLibraryDir: "~\\AppData\\Local\\Claude-3p\\configLibrary",
        profileMetaPath: "~\\AppData\\Local\\Claude-3p\\configLibrary\\_meta.json",
        ponytailPluginDir: "C:\\Program Files\\Claude\\org-plugins\\ponytail",
        ponytailInstalled: true,
        writable: true,
        message: "预览模式：Ponytail 组织插件已模拟安装。",
      },
    });
  }
  if (command === "install_ponytail_claude_desktop_local_bundle") {
    return ok("预览模式已模拟本地写入 Claude Desktop 开发模式插件包。", {
      outcome: {
        devMode: {
          configured: true,
          normalConfigPath: "~\\AppData\\Roaming\\Claude\\claude_desktop_config.json",
          threepConfigPath: "~\\AppData\\Local\\Claude-3p\\claude_desktop_config.json",
          profileMetaPath: "~\\AppData\\Local\\Claude-3p\\configLibrary\\_meta.json",
          backupPaths: [],
          message: "预览模式：开发模式已模拟配置。",
        },
        codexMcp: {
          item: { id: "desktop:claude-codex-pro-codex" },
          preview: null,
          installed: true,
          installMessage: "预览模式：已模拟写入 Codex MCP。",
          stdout: "",
          stderr: "",
          backupPath: null,
        },
        ponytailMcp: {
          item: { id: "ponytail:claude-desktop-mcp" },
          preview: null,
          installed: true,
          installMessage: "预览模式：已模拟写入 Ponytail MCP。",
          stdout: "",
          stderr: "",
          backupPath: null,
        },
        organizationPlugin: {
          installed: true,
          orgPluginsDir: "C:\\Program Files\\Claude\\org-plugins",
          pluginDir: "C:\\Program Files\\Claude\\org-plugins\\ponytail",
          manifestPath: "C:\\Program Files\\Claude\\org-plugins\\ponytail\\manifest.json",
          pluginJsonPath: "C:\\Program Files\\Claude\\org-plugins\\ponytail\\.claude-plugin\\plugin.json",
          copiedSkills: ["C:\\Program Files\\Claude\\org-plugins\\ponytail\\skills\\ponytail"],
          backupPath: null,
          message: "预览模式：已模拟本地复制组织插件 skills。",
        },
        message: "预览模式：开发模式、MCP 和组织插件 skills 均为本地写入链路，不调用 Claude CLI 登录。",
      },
      devModeStatus: {
        supported: true,
        configured: true,
        normalConfigPath: "~\\AppData\\Roaming\\Claude\\claude_desktop_config.json",
        threepConfigPath: "~\\AppData\\Local\\Claude-3p\\claude_desktop_config.json",
        configLibraryDir: "~\\AppData\\Local\\Claude-3p\\configLibrary",
        profileMetaPath: "~\\AppData\\Local\\Claude-3p\\configLibrary\\_meta.json",
        appliedId: "00000000-0000-4000-8000-000000157210",
        message: "预览模式：开发模式已模拟配置。",
      },
      orgPluginStatus: {
        supported: true,
        orgPluginsDir: "C:\\Program Files\\Claude\\org-plugins",
        configLibraryDir: "~\\AppData\\Local\\Claude-3p\\configLibrary",
        profileMetaPath: "~\\AppData\\Local\\Claude-3p\\configLibrary\\_meta.json",
        ponytailPluginDir: "C:\\Program Files\\Claude\\org-plugins\\ponytail",
        ponytailInstalled: true,
        writable: true,
        message: "预览模式：Ponytail 组织插件已模拟安装。",
      },
    });
  }
  if (command === "load_memory_assist_status") {
    return previewMemoryStatus();
  }
  if (command === "list_memory_assist_items") {
    return ok("预览模式记忆列表。", { items: previewMemoryItems() });
  }
  if (command === "list_memory_assist_candidates") {
    return ok("预览模式待确认记忆。", { candidates: previewMemoryCandidates() });
  }
  if (command === "learn_memory_assist_item") {
    const request = (_args?.request ?? {}) as { text?: string; workspace?: string; category?: string; source?: string };
    const stamp = now();
    return ok("预览模式已模拟保存记忆。", {
      item: {
        id: `preview-memory-${stamp}`,
        text: request.text || "预览记忆",
        workspace: request.workspace || "global",
        category: request.category || "manual",
        tags: [],
        source: request.source || "manager",
        sourceSessionId: "",
        createdAt: stamp,
        updatedAt: stamp,
        lastAccessedAt: stamp,
        accessCount: 0,
      },
    });
  }
  if (command === "query_memory_assist") {
    const request = (_args?.request ?? {}) as { query?: string; workspace?: string };
    return ok("预览模式已模拟搜索记忆。", {
      memory: {
        query: request.query || "",
        workspace: request.workspace || "__all__",
        results: previewMemoryItems().map((item) => ({ item, score: 0.92, matchedKeywords: ["preview", "backup"] })),
      },
    });
  }
  if (command === "update_memory_assist_item") {
    const request = (_args?.request ?? {}) as { id?: string; item?: Partial<PreviewMemoryItem> };
    const stamp = now();
    return ok("预览模式已模拟更新记忆。", {
      item: {
        ...previewMemoryItems()[0],
        ...request.item,
        id: request.id || request.item?.id || "preview-memory-1",
        updatedAt: stamp,
        lastAccessedAt: stamp,
      },
    });
  }
  if (command === "delete_memory_assist_item") {
    return ok("预览模式已模拟删除记忆。", { item: previewMemoryItems()[0] });
  }
  if (command === "archive_memory_assist_item") {
    return ok("预览模式已模拟归档记忆。", {
      item: { ...previewMemoryItems()[0], tier: "archived", archivedAt: Math.floor(Date.now() / 1000), retention: 0.05 },
    });
  }
  if (command === "restore_memory_assist_item") {
    return ok("预览模式已模拟恢复记忆。", {
      item: { ...previewMemoryItems()[0], tier: "active", archivedAt: 0, retention: 1, strength: 1 },
    });
  }
  if (command === "approve_memory_assist_candidate") {
    return ok("预览模式已模拟确认待确认记忆。", { item: { ...previewMemoryItems()[0], id: "approved-preview-memory" } });
  }
  if (command === "reject_memory_assist_candidate") {
    return ok("预览模式已模拟忽略待确认记忆。", { candidate: { ...previewMemoryCandidates()[0], status: "rejected" } });
  }
  if (command === "run_memory_assist_selfcheck") {
    return ok("预览模式已模拟盘古记忆自检。", {
      report: {
        status: "ok",
        repaired: false,
        backupPath: "~\\.claude-codex-pro\\backups\\memory-selfcheck-preview.json",
        checks: [
          { name: "sqlite", status: "ok", message: "预览数据库可打开。" },
          { name: "schema", status: "ok", message: "预览表结构完整。" },
        ],
      },
    });
  }
  if (command === "export_memory_assist") {
    return ok("预览模式已生成记忆导出包。", {
      data: {
        schemaVersion: "memory-assist/v1",
        exportedAt: now(),
        items: previewMemoryItems(),
        candidates: previewMemoryCandidates(),
      },
    });
  }
  if (command === "import_memory_assist") {
    return previewMemoryStatus("预览模式已模拟导入记忆。");
  }
  if (command === "list_local_sessions") {
    return ok("预览模式会话列表。", {
      dbPath: "~\\.codex\\sessions.db",
      dbPaths: ["~\\.codex\\sessions.db"],
      sessions: [
        {
          id: "preview-session",
          title: "Claude Codex Pro 前端重设计",
          cwd: "D:\\Project\\Claude-Codex-Pro-Tool",
          modelProvider: "codex",
          archived: false,
          updatedAtMs: Date.now(),
          rolloutPath: "",
          dbPath: "~\\.codex\\sessions.db",
        },
      ],
    });
  }
  if (command === "delete_local_session") {
    return ok("预览模式已模拟删除 Codex 会话。", {
      sessionId: "preview-session",
      undoToken: "preview-undo-token",
      backupPath: "~\\.codex\\backups\\preview-session.json",
    });
  }
  if (command === "load_settings") {
    return previewSettingsResult();
  }
  if (command === "save_settings") {
    return previewSettingsResult("预览模式已模拟保存设置。", (_args?.settings as ReturnType<typeof previewSettings> | undefined) ?? previewSettings());
  }
  if (command === "list_context_entries" || command === "upsert_context_entry" || command === "delete_context_entry") {
    const request = _args?.request as { settings?: ReturnType<typeof previewSettings> } | undefined;
    return previewContextEntries(request?.settings ?? previewSettings());
  }
  if (command === "read_live_context_entries" || command === "sync_live_context_entries") {
    const preview = previewContextEntries();
    return ok(command === "sync_live_context_entries" ? "预览模式已模拟同步当前 Codex 配置。" : "预览模式已模拟读取当前 Codex 配置。", {
      entries: preview.entries,
    });
  }
  if (command === "list_claude_context_entries" || command === "upsert_claude_context_entry" || command === "delete_claude_context_entry") {
    return previewClaudeContextEntries();
  }
  if (command === "scan_unified_tool_inventory") {
    return previewUnifiedToolInventory();
  }
  if (command === "toggle_unified_tool_asset") {
    const request = _args?.request as {
      id?: string;
      kind?: PreviewUnifiedToolAssetState["kind"];
      app?: "claude" | "codex";
      enabled?: boolean;
    } | undefined;
    const asset = previewUnifiedToolAssets.find((item) => item.id === request?.id && item.kind === request?.kind);
    if (!asset || !request?.app || typeof request.enabled !== "boolean") {
      const result = previewUnifiedToolInventory();
      return { ...result, status: "failed", message: "预览模式切换请求无效。" };
    }
    if (request.app === "claude") asset.claudeEnabled = request.enabled;
    else asset.codexEnabled = request.enabled;
    return previewUnifiedToolInventory(
      `预览模式已为 ${request.app === "claude" ? "Claude" : "Codex"}${request.enabled ? "启用" : "关闭"} ${asset.title}。`,
    );
  }
  if (command === "import_ccswitch_codex_providers") {
    return ok("已从 cc-switch 导入供应商配置：4 个。", {
      dbPath: "~\\.cc-switch\\cc-switch.db",
      scanned: 4,
      profiles: ["kuaipao", "Gpt-pro", "gpt-plus", "Claude-krio"].map((name) => ({
        id: `${name.toLowerCase().replace(/[^a-z0-9]+/g, "-")}-ccswitch`,
        name: `${name} (ccswitch)`,
        model: "gpt-5.5",
        baseUrl: name === "kuaipao" ? "https://kuaipao.ai/v1" : "https://api.toporeduce.cn/v1",
        upstreamBaseUrl: name === "kuaipao" ? "https://kuaipao.ai/v1" : "https://api.toporeduce.cn/v1",
        apiKey: "sk-preview",
        protocol: "responses",
        relayMode: "pureApi",
        officialMixApiKey: false,
        testModel: "gpt-5.5",
        configContents: "",
        authContents: "{\"OPENAI_API_KEY\":\"sk-preview\"}\n",
        useCommonConfig: true,
        contextSelection: { mcpServers: [], skills: [], plugins: [] },
        contextSelectionInitialized: false,
        contextWindow: "",
        autoCompactLimit: "",
        modelList: "gpt-5.5",
        userAgent: "ccswitch",
      })),
    });
  }
  if (command === "repair_backend") {
    return previewSettingsResult("预览模式已模拟修复后端。");
  }
  if (command === "repair_frontend_connection") {
    return ok("预览模式已模拟修复前端连接。", {
      target: "codex",
      frontendInjected: true,
      backendOnline: false,
      codexFrontendInjected: true,
      codexBackendOnline: false,
      claudeBackendOnline: false,
      debugPort: 57321,
      helperPort: 57322,
      claudeProxyPort: 57331,
      details: ["预览模式前端连接正常。"],
    });
  }
  if (command === "repair_backend_service") {
    return ok("预览模式已模拟修复后端服务。", {
      target: "local_backends",
      frontendInjected: false,
      backendOnline: true,
      codexFrontendInjected: false,
      codexBackendOnline: true,
      claudeBackendOnline: true,
      debugPort: 57321,
      helperPort: 57322,
      claudeProxyPort: 57331,
      details: ["预览模式后端服务正常。"],
    });
  }
  if (command === "reset_settings") {
    return previewSettingsResult("预览模式已模拟重置设置。");
  }
  if (command === "reset_image_overlay_settings") {
    const settings = { ...previewSettings(), codexAppImageOverlayEnabled: false, codexAppImageOverlayPath: "", codexAppImageOverlayOpacity: 70 };
    return previewSettingsResult("预览模式已模拟重置图片覆盖。", settings);
  }
  if (command === "sync_providers_now") {
    return ok("预览模式已模拟历史会话修复。", {
      syncStatus: "ok",
      targetProvider: "preview",
      changedSessionFiles: 1,
      skippedLockedRolloutFiles: [],
      sqliteRowsUpdated: 3,
      sqliteProviderRowsUpdated: 1,
      sqliteUserEventRowsUpdated: 1,
      sqliteCwdRowsUpdated: 1,
      updatedWorkspaceRoots: ["D:\\Project\\Claude-Codex-Pro-Tool"],
      backupDir: "~\\.codex\\backups",
      syncMessage: "预览模式不会修改真实会话。",
    });
  }
  if (command === "refresh_script_market") {
    return previewScriptMarket();
  }
  if (command === "install_market_script") {
    return previewScriptMarket("预览模式已模拟安装脚本。");
  }
  if (command === "install_entrypoints" || command === "uninstall_entrypoints" || command === "repair_shortcuts") {
    return ok("预览模式已模拟入口维护。", {
      silent_shortcut: { installed: command !== "uninstall_entrypoints", path: "Desktop\\Claude Code Pro.lnk" },
      management_shortcut: { installed: command !== "uninstall_entrypoints", path: "Desktop\\Claude Code Pro 管理工具.lnk" },
    });
  }
  if (command === "load_watcher_state" || command === "install_watcher" || command === "enable_watcher" || command === "disable_watcher" || command === "uninstall_watcher") {
    const enabled = command === "enable_watcher" || command === "install_watcher" || command === "load_watcher_state";
    return ok("预览模式 Watcher 状态。", {
      enabled,
      disabled_flag: enabled ? "" : "~\\.claude-codex-pro\\watcher.disabled",
    });
  }
  if (command === "read_latest_logs") {
    return ok("预览模式日志。", {
      path: "~\\.claude-codex-pro\\logs\\manager.log",
      text: [
        "[preview] 管理工具预览日志",
        "[preview] 按钮烟测不会修改真实系统",
        "[preview] 所有命令均返回可渲染响应",
      ].join("\n"),
      lines: 3,
    });
  }
  if (command === "check_update") {
    return ok("预览模式已模拟检查更新。", {
      currentVersion: "V0.12",
      latestVersion: "V0.12",
      releaseSummary: "预览模式：当前已是最新版本。",
      assetName: "claude-codex-pro-0.12-windows-x64-setup.exe",
      assetUrl: "https://example.invalid/claude-codex-pro-0.12-windows-x64-setup.exe",
      updateAvailable: false,
      progress: 0,
    });
  }
  if (command === "perform_update") {
    return ok("预览模式已模拟下载并运行安装包。", {
      currentVersion: "V0.12",
      latestVersion: "V0.12",
      releaseSummary: "预览模式不会下载真实安装包。",
      assetName: "claude-codex-pro-0.12-windows-x64-setup.exe",
      assetUrl: "https://example.invalid/claude-codex-pro-0.12-windows-x64-setup.exe",
      updateAvailable: false,
      progress: 100,
      installedPath: "~\\.claude-codex-pro\\updates\\preview-installer.exe",
      launched: true,
    });
  }
  if (command === "apply_relay_injection" || command === "apply_pure_api_injection" || command === "clear_relay_injection") {
    return ok("预览模式已模拟 API 模式切换。", {
      mode: command,
      path: "~\\.claude-codex-pro\\relay",
    });
  }
  return {
    status: "not_implemented",
    message: `当前是无 Tauri 预览环境，命令未执行：${command}`,
  } as CommandResult<Record<string, unknown>>;
}

function ok<T extends Record<string, unknown>>(message: string, payload: T): CommandResult<T> {
  return { status: "ok", message, ...payload };
}
