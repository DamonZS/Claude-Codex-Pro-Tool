import { invoke as tauriInvoke } from "@tauri-apps/api/core";

type Status = "ok" | "failed" | "not_checked" | string;

type CommandResult<T extends Record<string, unknown>> = T & {
  status: Status;
  message: string;
};

const hasTauriInternals = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function invokeCommand<T>(command: string, args?: Record<string, unknown>) {
  if (hasTauriInternals()) return tauriInvoke<T>(command, args);
  return mockInvoke(command, args) as Promise<T>;
}

async function mockInvoke(command: string, args?: Record<string, unknown>) {
  switch (command) {
    case "load_overview":
      return ok("浏览器预览数据已加载。", {
        codex_app: { status: "found", path: "C:/Users/Damon/AppData/Local/Programs/Codex/Codex.exe" },
        codex_version: "preview",
        silent_shortcut: { status: "found", path: "Desktop/Claude Codex Pro.lnk" },
        management_shortcut: { status: "found", path: "Desktop/Claude Codex Pro 管理工具.lnk" },
        latest_launch: { status: "running", message: "预览模式", started_at_ms: Date.now(), debug_port: 9229, helper_port: 14567, codex_app: "Codex.exe" },
        current_version: "1.2.9",
        update_status: "not_checked",
        settings_path: "C:/Users/Damon/.claude-codex-pro/settings.json",
        logs_path: "C:/Users/Damon/.claude-codex-pro/claude-codex-pro.log",
      });
    case "load_claude_desktop_status":
      return ok("Claude Desktop 诊断已读取。", {
        processCount: 1,
        executablePaths: ["C:/Program Files/WindowsApps/AnthropicClaude/Claude.exe"],
        installKind: "msix",
        cdpStatus: "blocked",
        cdpBlocker: "官方 MSIX 窗口不做 DOM 注入，中文化使用包装 WebView。",
        debugFlagsPresent: false,
        debugPorts: [],
        listeningPorts: [],
        debugEvidence: [],
        supportedIntegration: "wrapped_webview",
        integrityStatus: "untouched",
        integrityMessage: "未修改官方安装包。",
        executableAudits: [],
      });
    case "load_claude_chinese_window_status":
      return ok("Claude 中文窗口状态已读取。", {
        open: false,
        label: "claude-chinese",
        defaultUrl: "https://claude.ai/new",
        injectionMode: "wrapped_webview",
        cdpStatus: "blocked",
        cdpBlocker: "预览模式",
        officialInstallKind: "msix",
      });
    case "refresh_plugin_hub_catalog":
    case "get_plugin_hub_catalog":
      return ok("插件中心目录已加载。", mockPluginHub());
    case "preview_plugin_hub_install":
      return ok("安装预览已生成。", mockPreview(String((args?.request as { id?: string } | undefined)?.id || "official:agent-sdk-dev")));
    case "install_plugin_hub_item":
      return ok("预览模式不会执行安装。", { ...mockPreview(String((args?.request as { id?: string } | undefined)?.id || "official:agent-sdk-dev")), installed: false, stdout: "", stderr: "", backupPath: null });
    case "uninstall_plugin_hub_item":
      return ok("插件中心安装记录已移除。", mockPluginHub());
    case "load_settings":
      return ok("设置已加载。", mockSettings());
    case "refresh_script_market":
      return ok("脚本市场已加载。", {
        market: { status: "ok", message: "预览目录", indexUrl: "", updatedAt: "", scripts: [] },
        user_scripts: { enabled: true, scripts: [] },
      });
    case "read_latest_logs":
      return ok("日志已读取。", { path: "preview.log", text: "manager.start\nplugin_hub.refresh\n", lines: 2 });
    case "open_plugin_hub_window":
      return ok("插件中心已在管理工具内打开。", { open: true, label: "main" });
    case "open_prompt_optimizer_window":
      return ok("提示词优化器控制窗口已打开。", {
        open: true,
        label: "prompt-optimizer",
        defaultUrl: "https://prompt.always200.com",
        integrationMode: "internal_launcher_external_browser",
        license: "AGPL-3.0-only",
      });
    case "open_external_url":
    case "open_claude_chinese_window":
    case "open_claude_desktop":
    case "restart_claude_codex_pro":
    case "launch_claude_codex_pro":
    case "apply_relay_injection":
    case "apply_pure_api_injection":
    case "clear_relay_injection":
    case "repair_shortcuts":
    case "repair_backend":
    case "install_market_script":
      return ok("预览模式已拦截该动作。", {});
    default:
      return ok(`预览模式未实现命令：${command}`, {});
  }
}

function ok<T extends Record<string, unknown>>(message: string, payload: T): CommandResult<T> {
  return { status: "ok", message, ...payload };
}

function mockPluginHub() {
  return {
    catalog: {
      updatedAt: String(Math.floor(Date.now() / 1000)),
      sources: [
        { id: "desktop", label: "Claude Desktop MCP", url: "file://claude_desktop_config.json", status: "ok", message: "已加载", itemCount: 1 },
        { id: "official", label: "Claude 官方插件市场", url: "https://github.com/anthropics/claude-plugins-official", status: "ok", message: "已加载", itemCount: 2 },
        { id: "awesome", label: "Awesome Claude Code", url: "https://github.com/hesreallyhim/awesome-claude-code", status: "ok", message: "已加载", itemCount: 2 },
        { id: "github-mcp", label: "GitHub MCP Registry", url: "https://github.com/mcp", status: "ok", message: "已加载", itemCount: 1 },
      ],
      items: [
        pluginItem("desktop:claude-codex-pro-codex", "Claude Code / Codex MCP", "将 Claude Code 的 MCP 服务器注册到 Claude Desktop。", "desktop", "Claude Desktop MCP", "claude_desktop_mcp", "notInstalled", "codex"),
        pluginItem("official:agent-sdk-dev", "agent-sdk-dev", "Development kit for working with the Claude Agent SDK.", "official", "Claude 官方插件市场", "claude_plugin_marketplace", "notInstalled", "development"),
        pluginItem("official:typescript-lsp", "typescript-lsp", "TypeScript/JavaScript language server for enhanced code intelligence.", "official", "Claude 官方插件市场", "claude_plugin_marketplace", "notInstalled", "development"),
        pluginItem("awesome:skill-ca8cbc21", "AgentSys", "Workflow automation system for Claude with plugins, agents, and skills.", "awesome", "Awesome Claude Code", "skill_bundle", "needsReview", "Agent Skills"),
        pluginItem("awesome:mcp-demo", "示例 MCP Server", "社区 MCP 条目，安装前只生成配置草案。", "awesome", "Awesome Claude Code", "mcp_server", "needsReview", "MCP"),
        pluginItem("github-mcp:registry", "GitHub MCP Registry", "GitHub MCP Registry 入口，可浏览具体 MCP 服务器。", "github-mcp", "GitHub MCP Registry", "resource_link", "unsupported", "mcp"),
      ],
    },
  };
}

function pluginItem(id: string, name: string, description: string, sourceId: string, sourceLabel: string, installKind: string, installStatus: string, category: string) {
  return {
    id,
    name,
    description,
    sourceId,
    sourceLabel,
    sourceUrl: "https://example.com",
    category,
    author: sourceId === "official" ? "Anthropic / Partner" : "Community",
    homepage: "https://github.com/anthropics/claude-plugins-official",
    license: sourceId === "awesome" ? "MIT" : "",
    tags: [sourceId],
    installKind,
    installStatus,
    installCommand: installKind === "claude_plugin_marketplace" ? ["claude", "plugin", "install", `${name}@claude-plugins-official`] : [],
    configPreview: "",
    risk: sourceId === "official" ? "官方市场条目，安装前仍会显示 CLI 命令。" : "社区资源默认只展示，安装前需要人工审查仓库结构。",
    requirements: installKind === "claude_plugin_marketplace" ? ["claude CLI", "网络访问"] : ["人工审查"],
  };
}

function mockPreview(id: string) {
  const item = mockPluginHub().catalog.items.find((entry) => entry.id === id) ?? mockPluginHub().catalog.items[0];
  const isDesktopMcp = item.installKind === "claude_desktop_mcp";
  return {
    item,
    canInstall: item.installStatus !== "unsupported",
    action: isDesktopMcp || item.installKind === "mcp_server" ? "claude_desktop_mcp_config" : "claude_plugin_cli",
    command: isDesktopMcp ? ["claude", "mcp", "serve"] : item.installCommand,
    configDiff: isDesktopMcp ? "{\n  \"mcpServers\": {\n    \"claude-codex-pro-codex\": {\n      \"command\": \"claude\",\n      \"args\": [\"mcp\", \"serve\"]\n    }\n  }\n}" : item.installKind === "mcp_server" ? "[mcp_servers.demo]\ncommand = \"npx\"\nargs = [\"-y\", \"<package-or-command>\"]\nenabled = false\n" : "",
    message: "预览模式展示命令或配置 diff，不会执行写入。",
  };
}

function mockSettings() {
  return {
    settings_path: "C:/Users/Damon/.claude-codex-pro/settings.json",
    user_scripts: { enabled: true, scripts: [] },
    settings: {
      codexAppPath: "",
      codexExtraArgs: [],
      providerSyncEnabled: true,
      providerSyncSavedProviders: [],
      providerSyncManualProviders: [],
      providerSyncLastSelectedProvider: "",
      relayProfilesEnabled: true,
      enhancementsEnabled: true,
      computerUseGuardEnabled: true,
      codexAppPluginEntryUnlock: true,
      codexAppPluginMarketplaceUnlock: true,
      codexAppForcePluginInstall: false,
      codexAppModelWhitelistUnlock: false,
      codexAppSessionDelete: true,
      codexAppMarkdownExport: true,
      codexAppProjectMove: true,
      codexAppConversationTimeline: true,
      codexAppConversationView: true,
      codexAppThreadScrollRestore: true,
      codexAppZedRemoteOpen: true,
      zedRemoteOpenStrategy: "uri",
      zedRemoteProjectRegistryEnabled: true,
      zedRemoteSyncToZedSettings: false,
      codexAppUpstreamWorktreeCreate: false,
      codexAppNativeMenuPlacement: true,
      claudeAppChineseOverlayEnabled: true,
      codexAppServiceTierControls: true,
      codexAppImageOverlayEnabled: false,
      codexAppImageOverlayPath: "",
      codexAppImageOverlayOpacity: 0.8,
      codexGoalsEnabled: true,
      launchMode: "patch",
      relayBaseUrl: "https://api.toporeduce.cn",
      relayApiKey: "",
      relayProfiles: [{ id: "official", name: "官方中转站", model: "Claude Opus 4.8", baseUrl: "https://api.toporeduce.cn", upstreamBaseUrl: "", apiKey: "", protocol: "responses", relayMode: "relay", officialMixApiKey: false, testModel: "Claude Opus 4.8", configContents: "", authContents: "", useCommonConfig: true, contextSelection: { mcpServers: [], skills: [], plugins: [] }, contextSelectionInitialized: true, contextWindow: "", autoCompactLimit: "", modelList: "", userAgent: "" }],
      relayCommonConfigContents: "",
      relayContextConfigContents: "",
      activeRelayId: "official",
      relayTestModel: "Claude Opus 4.8",
      cliWrapperEnabled: true,
      cliWrapperBaseUrl: "https://api.toporeduce.cn",
      cliWrapperApiKey: "",
      cliWrapperApiKeyEnv: "",
    },
  };
}
