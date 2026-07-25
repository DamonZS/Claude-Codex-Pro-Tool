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
  if (statusFailed(updateInfo.status)) return updateInfo.phase === "failed" ? "应用内下载失败" : "检查失败";
  if (updateInfo.updateAvailable) return "有可用更新";
  if (statusOk(updateInfo.status)) return "已是最新";
  return "未检查";
}

export function trustedUpdateAssetUrl(updateInfo: UpdateResult | null): string | null {
  const version = updateInfo?.latestVersion?.trim();
  const name = updateInfo?.assetName?.trim();
  const value = updateInfo?.assetUrl?.trim();
  if (!version || !name || !value) return null;

  const assetVersion = version.replace(/^[vV]/, "");
  const supportedNames = [
    `claude-codex-pro-${assetVersion}-windows-x64-setup.exe`,
    `claude-codex-pro-${assetVersion}-macos-x64.dmg`,
    `claude-codex-pro-${assetVersion}-macos-arm64.dmg`,
  ];
  if (!supportedNames.includes(name)) return null;

  try {
    const url = new URL(value);
    const expectedPath = `/DamonZS/Claude-Codex-Pro-Tool/releases/download/${version}/${name}`;
    const trustedPrefix = "https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases/download";
    if (
      url.protocol !== "https:"
      || url.hostname !== "github.com"
      || url.port !== ""
      || url.username !== ""
      || url.password !== ""
      || url.search !== ""
      || url.hash !== ""
      || !url.toString().startsWith(`${trustedPrefix}/`)
      || url.pathname !== expectedPath
    ) return null;
    return url.toString();
  } catch {
    return null;
  }
}

export function compactUpdateError(message?: string | null) {
  const value = message?.trim() ?? "";
  if (!value) return "应用内下载失败，可改用系统浏览器下载安装包。";
  if (
    value.includes("安装包下载源")
    || value.includes("error sending request")
    || value.includes("system proxy")
    || value.includes("direct:")
  ) {
    return "应用内无法连接安装包下载源，可改用系统浏览器下载。";
  }
  return value.length > 180 ? `${value.slice(0, 177)}...` : value;
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
