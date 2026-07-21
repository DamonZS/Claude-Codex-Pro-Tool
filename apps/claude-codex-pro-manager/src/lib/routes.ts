import {
  Boxes,
  Info,
  LayoutDashboard,
  MessageSquare,
  Network,
  PackageSearch,
  Palette,
  FileText,
  Settings,
  ShieldCheck,
  Wrench,
  type LucideIcon,
} from "lucide-react";

import type { Route } from "@/types";

export type RouteItem = {
  id: Route;
  label: string;
  icon: LucideIcon;
  description: string;
  keywords: string[];
};

// `routes` is intentionally limited to the visible primary navigation
// entries. Compatibility routes remain in `routeCatalog` so external links and
// older launcher builds can still open them directly.
export const routes: RouteItem[] = [
  { id: "overview", label: "概览", icon: LayoutDashboard, description: "服务状态、异常与近期操作", keywords: ["首页", "状态", "dashboard"] },
  { id: "supplier", label: "供应商与路由", icon: Network, description: "第三方 API、模型映射、代理与故障转移", keywords: ["provider", "api", "model", "protocol", "模型", "协议", "代理", "中转"] },
  { id: "clients", label: "客户端与增强", icon: Boxes, description: "Codex、Claude 与本地增强状态", keywords: ["codex", "claude", "启动", "注入"] },
  { id: "themes", label: "主题中心", icon: Palette, description: "导入、应用与恢复 Codex 主题", keywords: ["theme", "主题", "皮肤", "外观"] },
  { id: "prompts", label: "系统提示词", icon: FileText, description: "管理 Codex 指令模板与生效方式", keywords: ["prompt", "instructions", "提示词", "指令"] },
  { id: "sessions", label: "会话与记忆", icon: MessageSquare, description: "会话迁移、项目接续与盘古记忆", keywords: ["session", "memory", "盘古"] },
  { id: "tools", label: "插件、Skills 与 MCP", icon: PackageSearch, description: "跨 Agent 扩展与依赖管理", keywords: ["plugin", "skill", "mcp", "扩展"] },
  { id: "maintenance", label: "维护与诊断", icon: Wrench, description: "入口、Watcher、日志与修复", keywords: ["repair", "watcher", "日志", "诊断"] },
  { id: "settings", label: "设置", icon: Settings, description: "偏好设置、更新与产品信息", keywords: ["配置", "about", "update"] },
];

export const compatibilityRoutes: RouteItem[] = [
  { id: "memory", label: "盘古记忆", icon: ShieldCheck, description: "长期记忆、召回证据与项目接续", keywords: ["memory", "记忆", "召回"] },
  { id: "about", label: "关于与更新", icon: Info, description: "版本、Release 与联系方式", keywords: ["about", "update", "版本"] },
];

export const routeCatalog: RouteItem[] = [...routes, ...compatibilityRoutes];

export function isRoute(value: unknown): value is Route {
  return routeCatalog.some((item) => item.id === value);
}

export function routeLabel(route: Route) {
  return routeCatalog.find((item) => item.id === route)?.label ?? "概览";
}

export function primaryRoute(route: Route): Route {
  if (route === "memory") return "sessions";
  if (route === "about") return "settings";
  return route;
}

export function routeBreadcrumb(route: Route) {
  const primary = primaryRoute(route);
  if (primary === route) return ["CCP", routeLabel(route)];
  return [routeLabel(primary), routeLabel(route)];
}

export function routeDomainTabs(route: Route): Array<{ id: Route; label: string }> {
  if (route === "sessions" || route === "memory") {
    return [
      { id: "sessions", label: "会话" },
      { id: "memory", label: "盘古记忆" },
    ];
  }
  if (route === "settings" || route === "about") {
    return [
      { id: "settings", label: "偏好设置" },
      { id: "about", label: "关于与更新" },
    ];
  }
  return [];
}

export function initialRoute(): Route {
  const injectedRoute = normalizeRoute(window.__CLAUDE_CODEX_PRO_INITIAL_ROUTE);
  if (isRoute(injectedRoute)) return injectedRoute;
  try {
    const view = normalizeRoute(new URLSearchParams(window.location.search).get("view"));
    if (isRoute(view)) return view;
  } catch {
    // Fall back to overview when running outside a normal browser URL.
  }
  return "overview";
}

export function normalizeRoute(value: unknown): unknown {
  if (value === "pluginHub" || value === "context" || value === "scripts") return "tools";
  if (value === "memoryAssist") return "memory";
  if (value === "logs") return "settings";
  if (value === "relay" || value === "models") return "supplier";
  return value;
}

export function routeSubtitle(route: Route) {
  const subtitles: Record<Route, string> = {
    overview: "服务健康、当前配置、异常与近期运行状态。",
    supplier: "管理第三方 API、目标应用、本地代理和路由策略。",
    clients: "管理 Codex、Claude Desktop、Claude Code 与本地增强。",
    themes: "浏览、导入、应用与恢复本机 Codex 主题。",
    prompts: "管理 Codex 系统提示词、分类与当前生效方式。",
    tools: "统一管理插件、Skills、MCP、来源、风险与依赖。",
    sessions: "查看本地会话、项目归属、迁移与供应商同步。",
    memory: "管理项目接续、长期记忆、召回证据与跨 Agent 共享。",
    maintenance: "检查入口、Watcher、后端、日志并执行明确修复。",
    settings: "调整本地偏好、增强开关和运行参数。",
    about: "检查版本、Release 更新与产品联系信息。",
  };
  return subtitles[route];
}

export function routeDocumentTitle(route: Route) {
  return route === "overview" ? "CCP 管理工具" : `${routeLabel(route)} - CCP 管理工具`;
}
