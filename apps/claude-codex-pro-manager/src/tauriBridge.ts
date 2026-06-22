import { invoke as tauriInvoke } from "@tauri-apps/api/core";

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
    memoryAssistMaxInjectedItems: 5,
    memoryAssistWorkspaceMode: "project_plus_global",
    launchMode: "patch",
    relayBaseUrl: "",
    relayApiKey: "",
    relayProfiles: [],
    relayCommonConfigContents: "",
    relayContextConfigContents: "",
    activeRelayId: "preview",
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
  ];
}

function previewPluginCatalog(message = "预览模式插件目录。") {
  return ok(message, {
    catalog: {
      updatedAt: new Date().toISOString(),
      sources: [
        { id: "official", label: "Claude 官方插件", url: "https://github.com/anthropics/claude-plugins-official", status: "ok", message: "预览数据", itemCount: 2 },
        { id: "codex-plugins", label: "Codex 插件仓库", url: "https://github.com/openai/plugins", status: "ok", message: "预览数据", itemCount: 2 },
        { id: "awesome", label: "awesome-claude-code", url: "https://github.com/hesreallyhim/awesome-claude-code", status: "ok", message: "预览数据", itemCount: 1 },
      ],
      items: previewPluginItems(),
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

function previewMemoryStatus(message = "预览模式记忆辅助状态。") {
  return ok(message, {
    memory: {
      status: "ok",
      dbPath: "~\\.claude-codex-pro\\memory_assist.sqlite",
      totalItems: 12,
      pendingCandidates: 3,
      workspaces: [{ workspace: "global", itemCount: 4, pendingCount: 1 }],
      latestBackupPath: "~\\.claude-codex-pro\\backups\\memory-preview.json",
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

const hasTauriInternals = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function invokeCommand<T>(command: string, args?: Record<string, unknown>) {
  if (hasTauriInternals()) return tauriInvoke<T>(command, args);
  return mockInvoke(command, args) as Promise<T>;
}

async function mockInvoke(command: string, _args?: Record<string, unknown>) {
  if (command === "open_external_url") return ok("预览模式不打开外部链接。", {});
  if (command === "launch_claude_codex_pro" || command === "restart_claude_codex_pro") {
    return ok(command === "launch_claude_codex_pro" ? "预览模式已模拟启动 Codex。" : "预览模式已模拟重启 Codex。", {
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
      management_shortcut: { status: "installed", path: "Desktop\\Claude Codex Pro.lnk" },
      latest_launch: {
        status: "running",
        message: "preview bridge",
        started_at_ms: Date.now(),
        debug_port: 57321,
        helper_port: 57322,
        codex_app: "preview",
      },
      current_version: "1.2.9-preview",
      update_status: "preview",
      settings_path: "~\\.claude-codex-pro\\settings.json",
      logs_path: "~\\.claude-codex-pro\\logs\\manager.log",
    });
  }
  if (command === "load_claude_desktop_status") {
    return ok("预览模式 Claude 诊断。", {
      processCount: 0,
      executablePaths: [],
      installKind: "msix",
      cdpStatus: "blocked",
      cdpBlocker: "官方 MSIX 窗口不可直接 DOM 注入",
      debugFlagsPresent: false,
      debugPorts: [],
      listeningPorts: [],
      debugEvidence: [],
      supportedIntegration: "wrapped_webview",
      integrityStatus: "not_modified",
      integrityMessage: "预览模式不修改官方 Claude。",
      executableAudits: [],
    });
  }
  if (command === "load_claude_chinese_window_status" || command === "open_claude_chinese_window") {
    return ok("预览模式 Claude 中文窗口状态。", {
      open: command === "open_claude_chinese_window",
      label: "Claude 中文窗口",
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
  if (command === "install_claude_zh_patch" || command === "restore_claude_zh_patch") {
    return ok(command === "install_claude_zh_patch" ? "预览模式已模拟 Claude 本机汉化。" : "预览模式已模拟恢复 Claude 官方文件。", {
      status: {
        status: "ok",
        message: "预览模式不会修改本机 Claude 文件。",
        installRoot: null,
        appRoot: null,
        installKind: "unknown",
        localeConfigPath: "~\\AppData\\Roaming\\Claude\\locale.json",
        backupDir: "~\\.claude-codex-pro\\claude-zh-backups",
        resourcesPresent: true,
        frontendI18nPresent: command === "install_claude_zh_patch",
        statsigI18nPresent: command === "install_claude_zh_patch",
        chunkPatchPresent: command === "install_claude_zh_patch",
        languageWhitelistPatched: command === "install_claude_zh_patch",
        localeConfigured: command === "install_claude_zh_patch",
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
    return ok("预览模式安装预览。", {
      item,
      canInstall: item.installKind === "claude_desktop_mcp" || item.installKind === "claude_plugin_marketplace",
      action: item.installKind,
      command: item.installCommand,
      configDiff: item.configPreview || "",
      message: item.installCommand.length ? `将执行：${item.installCommand.join(" ")}` : "该资源需要人工审查，预览模式不执行安装。",
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
    return previewPluginCatalog("预览模式已模拟移除插件记录。");
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
  if (command === "delete_memory_assist_item") {
    return ok("预览模式已模拟删除记忆。", { item: previewMemoryItems()[0] });
  }
  if (command === "approve_memory_assist_candidate") {
    return ok("预览模式已模拟确认待确认记忆。", { item: { ...previewMemoryItems()[0], id: "approved-preview-memory" } });
  }
  if (command === "reject_memory_assist_candidate") {
    return ok("预览模式已模拟忽略待确认记忆。", { candidate: { ...previewMemoryCandidates()[0], status: "rejected" } });
  }
  if (command === "run_memory_assist_selfcheck") {
    return ok("预览模式已模拟记忆辅助自检。", {
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
  if (command === "repair_backend") {
    return previewSettingsResult("预览模式已模拟修复后端。");
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
      silent_shortcut: { installed: command !== "uninstall_entrypoints", path: "Desktop\\Claude Codex Pro Silent.lnk" },
      management_shortcut: { installed: command !== "uninstall_entrypoints", path: "Desktop\\Claude Codex Pro.lnk" },
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
