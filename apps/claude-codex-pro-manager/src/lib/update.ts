import { compactPath, statusFailed, statusOk } from "@/lib/helpers";
import type { ClaudeDesktopResult, UpdateReleasePayload, UpdateResult } from "@/types";

export function updateInfoToRelease(updateInfo: UpdateResult | null): UpdateReleasePayload | null {
  if (!updateInfo?.latestVersion) return null;
  return {
    version: updateInfo.latestVersion,
    url: "",
    body: updateInfo.releaseSummary ?? "",
    asset_name: updateInfo.assetName ?? null,
    asset_url: updateInfo.assetUrl ?? null,
  };
}

export function updateStatusLabel(updateInfo: UpdateResult | null) {
  if (!updateInfo) return "未检查";
  if (updateInfo.status === "running") return "检查中";
  if (statusFailed(updateInfo.status)) return "检查失败";
  if (updateInfo.updateAvailable) return "有可用更新";
  if (statusOk(updateInfo.status)) return "已是最新";
  return "未检查";
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
