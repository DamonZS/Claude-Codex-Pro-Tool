import type {
  ClaudeDesktopDevModeStatusResult,
  ClaudeDesktopMarketplaceStatusResult,
  ClaudeDesktopOrgPluginStatusResult,
  ContextEntries,
  ContextEntry,
  ContextKind,
} from "@/types";

import { compactPath } from "@/lib/helpers";

export function emptyContextEntries(): ContextEntries {
  return { mcpServers: [], skills: [], plugins: [] };
}

export function normalizeContextKind(kind?: string | null): ContextKind {
  if (kind === "skill" || kind === "skills") return "skill";
  if (kind === "plugin" || kind === "plugins") return "plugin";
  return "mcp";
}

export function contextKindLabel(kind: ContextKind) {
  if (kind === "skill") return "Skills";
  if (kind === "plugin") return "插件";
  return "MCP";
}

export function contextEntriesByKind(entries: ContextEntries, kind: ContextKind) {
  if (kind === "skill") return entries.skills;
  if (kind === "plugin") return entries.plugins;
  return entries.mcpServers;
}

export function mergeContextEntries(managed: ContextEntries, live?: ContextEntries) {
  if (!live) return managed;
  const merge = (left: ContextEntry[], right: ContextEntry[]) => {
    const map = new Map<string, ContextEntry>();
    for (const entry of right) map.set(entry.id, entry);
    for (const entry of left) map.set(entry.id, entry);
    return [...map.values()];
  };
  return {
    mcpServers: merge(managed.mcpServers, live.mcpServers),
    skills: merge(managed.skills, live.skills),
    plugins: merge(managed.plugins, live.plugins),
  };
}

export function defaultContextToml(kind: ContextKind) {
  if (kind === "skill") return "enabled = true\npath = \"~/.codex/skills/example\"\n";
  if (kind === "plugin") return "enabled = true\n";
  return "enabled = true\ntype = \"stdio\"\ncommand = \"node\"\nargs = [\"server.js\"]\n";
}

export function setContextEnabled(tomlBody: string, enabled: boolean) {
  const lines = tomlBody.trimEnd().split(/\r?\n/);
  const nextLine = `enabled = ${enabled ? "true" : "false"}`;
  const index = lines.findIndex((line) => line.trim().startsWith("enabled"));
  if (index >= 0) {
    lines[index] = nextLine;
  } else {
    lines.unshift(nextLine);
  }
  return `${lines.join("\n")}\n`;
}

export function setJsonEnabled(jsonBody: string, enabled: boolean) {
  try {
    const parsed = JSON.parse(jsonBody || "{}");
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      return `${JSON.stringify({ ...parsed, enabled }, null, 2)}\n`;
    }
  } catch {
    // Keep the user's body intact if it is not valid JSON yet.
  }
  return jsonBody;
}

export function defaultClaudeContextBody(kind: ContextKind) {
  if (kind === "mcp") {
    return "{\n  \"command\": \"node\",\n  \"args\": [\"server.js\"],\n  \"enabled\": true\n}\n";
  }
  if (kind === "skill") {
    return "{\n  \"enabled\": true,\n  \"skills\": []\n}\n";
  }
  return "{\n  \"enabled\": true\n}\n";
}

export function claudeStatusContextEntries(
  devMode: ClaudeDesktopDevModeStatusResult | null,
  marketplace: ClaudeDesktopMarketplaceStatusResult | null,
  orgPlugin: ClaudeDesktopOrgPluginStatusResult | null,
): ContextEntries {
  const org = orgPlugin?.orgPluginStatus;
  const market = marketplace?.marketplaceStatus;
  const dev = devMode?.devModeStatus;
  return {
    mcpServers: [
      {
        id: "claude-codex-pro-codex",
        kind: "mcp",
        title: "Claude Code Pro MCP",
        summary: dev?.configured ? "开发模式配置已写入" : "等待写入 Claude 开发模式配置",
        tomlBody: "enabled = true\ncommand = \"claude-codex-pro\"\n",
        enabled: Boolean(dev?.configured),
      },
      {
        id: "ponytail",
        kind: "mcp",
        title: "Ponytail MCP",
        summary: org?.ponytailInstalled ? "本地组织插件已安装" : "可通过一键写入本地插件安装",
        tomlBody: "enabled = true\ncommand = \"node\"\n",
        enabled: Boolean(org?.ponytailInstalled),
      },
    ],
    skills: [
      {
        id: "ponytail",
        kind: "skill",
        title: "Ponytail Skills",
        summary: org?.ponytailPluginDir ? compactPath(org.ponytailPluginDir) : "组织插件 Skills 目录未检测",
        tomlBody: "enabled = true\n",
        enabled: Boolean(org?.ponytailInstalled),
      },
    ],
    plugins: [
      {
        id: market?.plugin || "ponytail",
        kind: "plugin",
        title: market?.marketplace || "DietrichGebert/ponytail",
        summary: market?.message || "Claude 官方插件仓库入口，可选。",
        tomlBody: "enabled = true\n",
        enabled: Boolean(market?.supported),
      },
    ],
  };
}

export function claudeContextSummary(
  devMode: ClaudeDesktopDevModeStatusResult | null,
  marketplace: ClaudeDesktopMarketplaceStatusResult | null,
  orgPlugin: ClaudeDesktopOrgPluginStatusResult | null,
) {
  const dev = devMode?.devModeStatus.configured ? "开发模式已配置" : "开发模式未配置";
  const org = orgPlugin?.orgPluginStatus.ponytailInstalled ? "组织插件已安装" : "组织插件未安装";
  const market = marketplace?.marketplaceStatus.supported ? "官方插件入口可用" : "官方插件入口未检测";
  return `${dev}；${org}；${market}。`;
}
