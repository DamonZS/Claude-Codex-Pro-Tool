import { compactPath, statusFailed, statusOk } from "@/lib/helpers";
import type { ClaudeDesktopResult, UpdateReleasePayload, UpdateResult } from "@/types";

export function updateInfoToRelease(updateInfo: UpdateResult | null): UpdateReleasePayload | null {
  if (!updateInfo?.latestVersion) return null;
  return {
    expectedVersion: updateInfo.latestVersion,
  };
}

export function updateStatusLabel(updateInfo: UpdateResult | null) {
  if (!updateInfo) return "未检查";
  if (updateInfo.status === "running") return updateProgressLabel(updateInfo.phase, updateInfo.progress);
  if (statusFailed(updateInfo.status)) return "检查失败";
  if (updateInfo.updateAvailable) return "有可用更新";
  if (statusOk(updateInfo.status)) return "已是最新";
  return "未检查";
}

export function updateProgressLabel(phase?: string, progress?: number) {
  switch (phase) {
    case "checking":
      return "检查中";
    case "connecting":
      return "正在连接下载源";
    case "downloading":
      return typeof progress === "number" ? `下载中 ${Math.round(progress)}%` : "正在下载";
    case "launching":
      return "正在启动安装包";
    case "complete":
      return "安装包已启动";
    case "failed":
      return "更新失败";
    default:
      return "处理中";
  }
}

export function formatDownloadBytes(bytes?: number | null) {
  if (typeof bytes !== "number" || !Number.isFinite(bytes) || bytes < 0) return "未知";
  if (bytes < 1024) return `${Math.round(bytes)} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function displayAssetName(name?: string | null) {
  if (!name) return "未检测";
  return name
    .replace(/CodexPlusPlus/gi, "Claude Codex Pro")
    .replace(/claude-codex-pro/gi, "Claude Codex Pro");
}

export function claudeDesktopVersionLabel(claudeDesktop: ClaudeDesktopResult | null) {
  if (!claudeDesktop) return "未检测";
  const install = claudeDesktop.installKind || "未知安装";
  const path = claudeDesktop.executablePaths?.[0] ? compactPath(claudeDesktop.executablePaths[0]) : "未检测到路径";
  return `${install} · ${path}`;
}
