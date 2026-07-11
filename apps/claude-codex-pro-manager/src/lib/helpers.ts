import { PLUGIN_REPOSITORY_REPAIR_PROMPT_KEY_PREFIX } from "@/constants";
import type {
  BackendSettings,
  ClaudeSession,
  ClaudeSessionProjectGroup,
  ClaudeDesktopMarketplaceStatusResult,
  ClaudeDesktopResult,
  ClaudeZhPatchResult,
  CodexPluginMarketplaceStatusResult,
  LocalSession,
  LocalSessionProjectGroup,
  MemorySelfCheckResult,
  MemoryStatusResult,
  OverviewResult,
  StatusChip,
} from "@/types";

export function statusOk(status?: string | null) {
  return status === "ok" || status === "accepted" || status === "found" || status === "installed" || status === "running" || status === "idle";
}

export function statusFailed(status?: string | null) {
  return status === "failed" || status === "not_implemented";
}

export function compactPath(path?: string | null) {
  if (!path) return "未设置";
  if (path.length <= 58) return path;
  return `${path.slice(0, 24)}...${path.slice(-28)}`;
}

export function pathTail(path?: string | null) {
  const value = (path || "").trim().replace(/[\\/]+$/, "");
  if (!value) return "";
  const parts = value.split(/[\\/]+/).filter(Boolean);
  return parts.at(-1) || value;
}

export function localSessionProjectLabel(session: LocalSession) {
  return pathTail(session.cwd) || pathTail(session.rolloutPath) || pathTail(session.dbPath) || "未归类项目";
}

export function formatSessionRelativeTime(updatedAtMs?: number | null) {
  if (!updatedAtMs) return "未知";
  const diffMs = Math.max(0, Date.now() - updatedAtMs);
  const minute = 60 * 1000;
  const hour = 60 * minute;
  const day = 24 * hour;
  const week = 7 * day;
  if (diffMs < minute) return "刚刚";
  if (diffMs < hour) return `${Math.max(1, Math.floor(diffMs / minute))} 分钟`;
  if (diffMs < day) return `${Math.max(1, Math.floor(diffMs / hour))} 小时`;
  if (diffMs < week) return `${Math.max(1, Math.floor(diffMs / day))} 天`;
  return `${Math.max(1, Math.floor(diffMs / week))} 周`;
}

export function groupLocalSessionsByProject(sessions: LocalSession[]) {
  const groups = new Map<string, LocalSessionProjectGroup>();
  for (const session of sessions) {
    const label = localSessionProjectLabel(session);
    const key = (session.cwd || session.rolloutPath || session.dbPath || label).toLowerCase();
    const group = groups.get(key) ?? {
      key,
      label,
      subtitle: session.cwd || session.rolloutPath || session.dbPath || "",
      sessions: [],
    };
    group.sessions.push(session);
    groups.set(key, group);
  }
  return Array.from(groups.values())
    .map((group) => ({
      ...group,
      sessions: group.sessions
        .slice()
        .sort((left, right) => (right.updatedAtMs ?? 0) - (left.updatedAtMs ?? 0)),
    }))
    .sort((left, right) => (right.sessions[0]?.updatedAtMs ?? 0) - (left.sessions[0]?.updatedAtMs ?? 0));
}

export function claudeSessionProjectLabel(session: ClaudeSession) {
  return pathTail(session.cwd) || pathTail(session.sourcePath) || "未归类项目";
}

export function groupClaudeSessionsByProject(sessions: ClaudeSession[]) {
  const groups = new Map<string, ClaudeSessionProjectGroup>();
  for (const session of sessions) {
    const label = claudeSessionProjectLabel(session);
    const key = (session.cwd || session.sourcePath || label).toLowerCase();
    const group = groups.get(key) ?? {
      key,
      label,
      subtitle: session.cwd || session.sourcePath || "",
      sessions: [],
    };
    group.sessions.push(session);
    groups.set(key, group);
  }
  return Array.from(groups.values())
    .map((group) => ({
      ...group,
      sessions: group.sessions
        .slice()
        .sort((left, right) => (right.updatedAtMs ?? 0) - (left.updatedAtMs ?? 0)),
    }))
    .sort((left, right) => (right.sessions[0]?.updatedAtMs ?? 0) - (left.sessions[0]?.updatedAtMs ?? 0));
}

export function codexPluginMarketplaceNeedsRepair(result?: CodexPluginMarketplaceStatusResult | null) {
  const status = result?.marketplace;
  return Boolean(result && (status?.needsRepair || !status?.configRegistered || !status?.marketplaceRoot));
}

export function claudeDesktopMarketplaceNeedsRepair(result?: ClaudeDesktopMarketplaceStatusResult | null) {
  const status = result?.marketplaceStatus;
  const repositories = status?.repositories ?? [];
  return Boolean(
    result &&
      status?.canAutoWrite &&
      (repositories.length === 0 || repositories.some((repository) => !repository.configured)),
  );
}

export function pluginRepositoryRepairPromptKey(
  codex: CodexPluginMarketplaceStatusResult | null,
  claude: ClaudeDesktopMarketplaceStatusResult | null,
) {
  const codexStatus = codex?.marketplace;
  const claudeStatus = claude?.marketplaceStatus;
  const claudeMissing = (claudeStatus?.repositories ?? [])
    .filter((repository) => !repository.configured)
    .map((repository) => repository.repository)
    .join(",");
  return [
    PLUGIN_REPOSITORY_REPAIR_PROMPT_KEY_PREFIX,
    codexStatus?.needsRepair ? "codex-needs-repair" : "codex-ok",
    codexStatus?.marketplaceRoot ?? "codex-no-root",
    codexStatus?.configRegistered ? "codex-registered" : "codex-unregistered",
    claudeMissing || "claude-ok",
    claudeStatus?.configPath ?? "claude-no-config",
  ].join("|");
}

export function pluginRepositoryRepairPromptMessage(
  codex: CodexPluginMarketplaceStatusResult | null,
  claude: ClaudeDesktopMarketplaceStatusResult | null,
) {
  const items: string[] = [];
  if (codexPluginMarketplaceNeedsRepair(codex)) {
    items.push("Codex OpenAI 插件仓库未下载、未注册或配置异常。");
  }
  if (claudeDesktopMarketplaceNeedsRepair(claude)) {
    const missing = (claude?.marketplaceStatus.repositories ?? [])
      .filter((repository) => !repository.configured)
      .map((repository) => repository.label || repository.repository)
      .join("、");
    items.push(`Claude 插件仓库配置缺失${missing ? `：${missing}` : ""}。`);
  }
  return `检测到插件仓库配置异常，是否立即修复？\n\n${items.join("\n")}`;
}

export function displayProductPath(path?: string | null) {
  if (!path) return "";
  return path
    .replace(/Codex\+\+/g, "Claude Code Pro")
    .replace(/Claude Codex Pro 管理工具/g, "Claude Code Pro 管理工具")
    .replace(/Claude Codex Pro/g, "Claude Code Pro");
}

export function compactDisplayPath(path?: string | null) {
  const display = displayProductPath(path);
  return display ? compactPath(display) : "未设置";
}

export function codexOverviewStatus(overview: OverviewResult | null) {
  const launch = overview?.latest_launch;
  const launchStatus = launch?.status ?? "not_checked";
  const running = launchStatus === "running" || launchStatus === "degraded";
  const failed = statusFailed(launchStatus);
  const frontendOnline = Boolean(launch?.frontend_runtime_online || launch?.debug_port_online);
  const backendOnline = Boolean(launch?.helper_port_online);
  const items: StatusChip[] = [
    { label: failed ? "运行异常" : running ? "运行中" : "未运行", tone: failed ? "warn" : running ? "ok" : "muted" },
    { label: launchStatus === "degraded" || failed ? "注入异常" : running ? "注入成功" : "未注入", tone: launchStatus === "degraded" || failed ? "warn" : running ? "ok" : "muted" },
    { label: frontendOnline ? "前端在线" : launch?.debug_port ? "CDP 离线" : "前端未检测", tone: frontendOnline ? "ok" : running ? "warn" : "muted" },
    { label: backendOnline ? "后端在线" : "后端离线", tone: backendOnline ? "ok" : running ? "warn" : "muted" },
  ];
  const status = items.some((item) => item.tone === "warn") ? "failed" : running ? "running" : "not_checked";
  return { status, items };
}

export function claudeOverviewStatus(claudeDesktop: ClaudeDesktopResult | null, claudeZhPatch: ClaudeZhPatchResult | null) {
  const hasProcess = (claudeDesktop?.processCount ?? 0) > 0;
  const cdpStatus = claudeDesktop?.cdpStatus ?? "not_checked";
  const detectFailed = !!claudeDesktop && statusFailed(claudeDesktop.status);
  const injected = !!claudeZhPatch?.status.localeConfigured && !!claudeZhPatch?.status.frontendI18nPresent && !!claudeZhPatch?.status.chunkPatchPresent;
  const inspectorReady = cdpStatus === "node_inspector_ready" || (claudeDesktop?.inspectorPorts?.length ?? 0) > 0;
  const cdpWarn = !inspectorReady && cdpStatus === "failed";
  const items: StatusChip[] = [
    { label: detectFailed ? "检测异常" : hasProcess ? "运行中" : "未运行", tone: detectFailed ? "warn" : hasProcess ? "ok" : "muted" },
    { label: injected ? "汉化已注入" : "汉化未注入", tone: injected ? "ok" : "muted" },
  ];
  if (inspectorReady || cdpStatus === "ok" || cdpStatus === "failed") {
    items.push({
      label: inspectorReady ? "Inspector 在线" : cdpStatus === "failed" ? "CDP 异常" : "CDP 在线",
      tone: cdpWarn ? "warn" : "ok",
    });
  }
  const status = detectFailed ? "failed" : hasProcess ? "running" : "not_checked";
  return { status, items };
}

export function memoryOverviewStatus(memoryAssist: MemoryStatusResult | null, settings: BackendSettings | null) {
  const memory = memoryAssist?.memory;
  const enabled = memory?.enabled ?? Boolean(settings?.memoryAssistEnabled);
  const injectEnabled = memory?.injectEnabled ?? Boolean(settings?.memoryAssistInjectEnabled);
  const autoSuggest = memory?.autoSuggestEnabled ?? Boolean(settings?.memoryAssistAutoSuggestEnabled);
  const healthy = memory?.status === "ok";
  const hasDb = Boolean(memory?.dbPath);
  const runtimeStatus = memory?.runtimeStatus ?? "not_checked";
  const codexInjected = Boolean(memory?.codexInjected);
  const listening = Boolean(memory?.active);
  const items: StatusChip[] = [
    { label: enabled ? "开关已开启" : "开关已关闭", tone: enabled ? "ok" : "muted" },
    { label: healthy ? "运行正常" : memoryAssist ? "运行异常" : "未检测", tone: healthy ? "ok" : memoryAssist ? "warn" : "muted" },
    { label: codexInjected ? "Codex 已注入" : injectEnabled ? "等待 Codex 注入" : "注入已关闭", tone: codexInjected ? "ok" : enabled && injectEnabled ? "warn" : "muted" },
    { label: listening ? "对话监控运行中" : codexInjected && autoSuggest ? "等待会话变化" : autoSuggest ? "等待 Codex 注入" : "对话监控关闭", tone: listening || (codexInjected && autoSuggest) ? "ok" : enabled && autoSuggest ? "warn" : "muted" },
    { label: hasDb ? "数据库在线" : "数据库未检测", tone: hasDb ? "ok" : enabled ? "warn" : "muted" },
  ];
  const status = items.some((item) => item.tone === "warn")
    ? "failed"
    : enabled && healthy && (codexInjected || runtimeStatus === "disabled")
      ? "running"
      : "not_checked";
  return { status, items };
}

export function codexLaunchRequestFromOverview(overview: OverviewResult | null) {
  return {
    appPath: overview?.codex_app.path || overview?.latest_launch?.codex_app || "",
  };
}

export function zhPatchNoticeMessage(result: ClaudeZhPatchResult) {
  const status = result.status;
  const patchStatus = result.status.status;
  const installKind = result.status.installKind || "unknown";
  const writable = result.status.writable ? "可写" : "不可写/需管理员授权";
  if (!statusFailed(status)) {
    return `${result.message}\n资源状态：${patchStatus} / ${installKind} / ${writable}\n已自动启动/重启 Claude Desktop，请验证界面语言。`;
  }
  const logPath = result.logsPath || "~\\.claude-codex-pro\\claude-codex-pro.log";
  return `${result.message}\n资源状态：${patchStatus} / ${installKind} / ${writable}\n诊断日志：${logPath}`;
}

export function afterFirstPaint(task: () => void, delayMs = 0) {
  window.requestAnimationFrame(() => {
    window.requestAnimationFrame(() => {
      window.setTimeout(task, delayMs);
    });
  });
}

export function stringifyError(error: unknown) {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return JSON.stringify(error);
}

export function waitForPaint() {
  return new Promise<void>((resolve) => {
    window.requestAnimationFrame(() => window.requestAnimationFrame(() => resolve()));
  });
}

export function memoryRefineSummary(result: MemorySelfCheckResult): string {
  const history = result.report.checks.find((check) => check.name === "history");
  const historyMessage = history?.message || "未返回历史扫描结果。";
  const failedChecks = result.report.checks.filter((check) => !statusOk(check.status));
  const failedSummary = failedChecks.length
    ? ` 需关注：${failedChecks.map((check) => `${check.name}:${check.status}`).join(" / ")}。`
    : "";
  return `使用 Codex 本地 SQLite、rollout 会话文件和 memory_assist.sqlite 遍历工作区与会话。结果：${historyMessage}.${failedSummary}`;
}

export function buttonLogLabel(button: HTMLButtonElement): string {
  const label = (button.getAttribute("aria-label") || button.title || button.textContent || "")
    .replace(/\s+/g, " ")
    .trim();
  return (label || "unlabeled-button").slice(0, 120);
}
