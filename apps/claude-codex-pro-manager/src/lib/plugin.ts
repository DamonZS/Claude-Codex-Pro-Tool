import type { PluginInstallKind, PluginInstallStatus } from "@/types";

export function pluginKindLabel(kind: PluginInstallKind) {
  if (kind === "claude_desktop_mcp") return "Claude Desktop MCP";
  if (kind === "claude_desktop_org_plugin") return "Claude Desktop 组织插件";
  if (kind === "claude_code_plugin") return "Claude Code 插件";
  if (kind === "codex_plugin") return "Codex 插件";
  if (kind === "copilot_plugin") return "GitHub Copilot CLI 插件";
  if (kind === "managed_skill_bundle") return "托管 Skill Bundle";
  if (kind === "claude_plugin_marketplace") return "Claude Code 插件";
  const labels: Partial<Record<PluginInstallKind, string>> = {
    claude_plugin_marketplace: "Claude 插件",
    mcp_server: "MCP 服务器",
    skill_bundle: "Skill Bundle",
    resource_link: "资源链接",
  };
  return labels[kind] ?? kind;
}

export function pluginCanInstall(kind: PluginInstallKind) {
  return [
    "claude_desktop_mcp",
    "claude_desktop_org_plugin",
    "claude_plugin_marketplace",
    "claude_code_plugin",
    "codex_plugin",
    "copilot_plugin",
    "managed_skill_bundle",
  ].includes(kind);
}

export function pluginInstallButtonLabel(kind: PluginInstallKind) {
  const labels: Partial<Record<PluginInstallKind, string>> = {
    claude_desktop_mcp: "安装到 Claude Desktop",
    claude_desktop_org_plugin: "安装到 Claude Desktop",
    claude_plugin_marketplace: "用 Claude CLI 安装",
    claude_code_plugin: "安装到 Claude Code",
    codex_plugin: "安装到 Codex",
    copilot_plugin: "安装到 Copilot CLI",
    managed_skill_bundle: "安装技能包",
  };
  return labels[kind] ?? "安装";
}

export function pluginStatusLabel(status: PluginInstallStatus) {
  const labels: Record<PluginInstallStatus, string> = {
    notInstalled: "未安装",
    installed: "已安装",
    needsReview: "需审查",
    unsupported: "仅浏览",
  };
  return labels[status] ?? status;
}
