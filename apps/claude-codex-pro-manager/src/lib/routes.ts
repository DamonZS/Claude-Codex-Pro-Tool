import { Info, LayoutDashboard, MessageSquare, Network, PackageSearch, Settings, ShieldCheck, Wrench, type LucideIcon } from "lucide-react";

import type { Route } from "@/types";

export const routes: Array<{ id: Route; label: string; icon: LucideIcon }> = [
  { id: "overview", label: "概览", icon: LayoutDashboard },
  { id: "supplier", label: "供应商", icon: Network },
  { id: "tools", label: "工具与插件", icon: PackageSearch },
  { id: "sessions", label: "会话管理", icon: MessageSquare },
  { id: "memory", label: "盘古记忆", icon: ShieldCheck },
  { id: "maintenance", label: "维护", icon: Wrench },
  { id: "settings", label: "设置", icon: Settings },
  { id: "about", label: "关于", icon: Info },
];

export function isRoute(value: unknown): value is Route {
  return routes.some((item) => item.id === value);
}

export function routeLabel(route: Route) {
  return routes.find((item) => item.id === route)?.label ?? "概览";
}

export function initialRoute(): Route {
  const injectedRoute = normalizeRoute(window.__CLAUDE_CODEX_PRO_INITIAL_ROUTE);
  if (routes.some((item) => item.id === injectedRoute)) return injectedRoute as Route;
  try {
    const view = normalizeRoute(new URLSearchParams(window.location.search).get("view"));
    if (routes.some((item) => item.id === view)) return view as Route;
  } catch {
    // Fall back to overview when running outside a normal browser URL.
  }
  return "overview";
}

export function normalizeRoute(value: unknown): unknown {
  if (value === "pluginHub" || value === "context" || value === "promptOptimizer" || value === "scripts") return "tools";
  if (value === "memoryAssist") return "memory";
  if (value === "logs") return "settings";
  if (value === "relay") return "supplier";
  return value;
}

export function routeSubtitle(route: Route) {
  const subtitles: Record<Route, string> = {
    overview: "运行状态、启动动作和 Claude 一键汉化诊断。",
    supplier: "Codex 中转配置与 Claude Desktop 开发模式供应商写入。",
    tools: "插件目录、MCP 配置和启动入口。",
    sessions: "历史会话修复、盘古记忆和会话诊断。",
    memory: "三层链路、增量采集、经验教训注入手册与 MCP 共享。",
    maintenance: "入口、快捷方式、后端和 Watcher 维护。",
    settings: "全局开关、配置摘要和运行日志。",
    about: "版本信息、项目地址和 GitHub Release 更新。",
  };
  return subtitles[route];
}

export function routeDocumentTitle(route: Route) {
  return route === "overview" ? "Claude Codex Pro 管理工具" : `${routeLabel(route)} - Claude Codex Pro 管理工具`;
}
