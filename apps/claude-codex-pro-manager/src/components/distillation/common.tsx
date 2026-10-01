import claudeLogo from "@/assets/claude.svg";
import codexLogo from "@/assets/openai.svg";
import cursorLogo from "@/assets/agent-brands/cursor.svg";
import deepseekLogo from "@/assets/agent-brands/deepseek.svg";
import openclawLogo from "@/assets/agent-brands/openclaw.svg";
import workbuddyLogo from "@/assets/agent-brands/workbuddy.svg";
import type { DistillationCandidate, DistillationWorkbenchSession } from "@/types";

export const EST_TOKENS_PER_TURN = 900;

const AGENT_LOGOS: Record<string, string> = {
  codex: codexLogo,
  "claude-code": claudeLogo,
  "claude-desktop": claudeLogo,
  workbuddy: workbuddyLogo,
  cursor: cursorLogo,
  "deepseek-harness": deepseekLogo,
  openclaw: openclawLogo,
};

const AGENT_LABELS: Record<string, string> = {
  codex: "Codex",
  "claude-code": "Claude Code",
  "claude-desktop": "Claude Desktop",
  workbuddy: "WorkBuddy",
  cursor: "Cursor",
  openclaw: "OpenClaw",
  "deepseek-harness": "DeepSeek",
};

/** Per-agent brand colour used for drawer accents (pill / bubble / range chip). */
const AGENT_COLORS: Record<string, string> = {
  codex: "var(--dw-chart-2)",
  "claude-code": "var(--dw-chart-3)",
  "claude-desktop": "var(--dw-chart-3)",
  cursor: "var(--dw-chart-4)",
  workbuddy: "var(--dw-chart-1)",
  openclaw: "var(--dw-chart-5)",
};

export function agentLabel(agent: string) {
  return AGENT_LABELS[agent.toLocaleLowerCase()] ?? agent;
}

export function agentColor(agent: string) {
  return AGENT_COLORS[agent.toLocaleLowerCase()] ?? "var(--dw-chart-1)";
}

export function AgentIcon({ agent, className = "" }: { agent: string; className?: string }) {
  const logo = AGENT_LOGOS[agent.toLocaleLowerCase()];
  return logo
    ? <img alt="" aria-hidden="true" className={`dw-agent-icon ${className}`} src={logo} />
    : <span aria-hidden="true" className={`dw-agent-icon dw-agent-mark ${className}`}>{agentLabel(agent).slice(0, 1).toUpperCase()}</span>;
}

export function materialKeyOf(item: { agent: string; sessionId: string }) {
  return `${item.agent}:${item.sessionId}`;
}

export function sessionTurns(session: DistillationWorkbenchSession) {
  return session.turns ?? session.events ?? 0;
}

export function sessionProjectKey(session: DistillationWorkbenchSession) {
  return session.projectKey || session.project || "unknown";
}

export function sessionStartedAt(session: DistillationWorkbenchSession) {
  return session.startedAt || session.updatedAt;
}

export function timestampValue(value: string | number | null | undefined) {
  if (typeof value === "number") return Number.isFinite(value) ? value : Number.NaN;
  const normalized = value?.trim();
  if (!normalized) return Number.NaN;
  return /^\d+(?:\.\d+)?$/.test(normalized) ? Number(normalized) : Date.parse(normalized);
}

export function formatDateTime(value: string | number | null | undefined) {
  const date = new Date(timestampValue(value));
  return Number.isNaN(date.getTime())
    ? "时间未知"
    : date.toLocaleString("zh-CN", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit", hour12: false });
}

export function formatDate(value: string | number | null | undefined) {
  const date = new Date(timestampValue(value));
  return Number.isNaN(date.getTime())
    ? "时间未知"
    : date.toLocaleDateString("zh-CN", { year: "numeric", month: "2-digit", day: "2-digit" });
}

function compactTokenValue(value: number) {
  return value.toFixed(value >= 100 ? 0 : value >= 10 ? 1 : 2).replace(/\.?0+$/, "");
}

export function formatTokens(value: number) {
  if (value >= 1_000_000_000) return `${compactTokenValue(value / 1_000_000_000)}B`;
  if (value >= 1_000_000) return `${compactTokenValue(value / 1_000_000)}M`;
  if (value >= 1_000) return `${compactTokenValue(value / 1_000)}K`;
  return Math.round(value).toLocaleString("zh-CN");
}

/** Resolves a candidate's source sessions into project / agent / title lines. */
export function resolveCandidateSource(candidate: DistillationCandidate, sessions: readonly DistillationWorkbenchSession[]) {
  const byKey = new Map(sessions.map((session) => [materialKeyOf(session), session]));
  const projects: string[] = [];
  const sources: string[] = [];
  const titles: string[] = [];
  const refs = candidate.sourceRefs?.length ? candidate.sourceRefs : [{ agent: candidate.agent, sessionId: candidate.sessionId, project: "" }];
  for (const ref of refs) {
    const item = byKey.get(materialKeyOf(ref));
    const source = agentLabel(ref.agent);
    if (!sources.includes(source)) sources.push(source);
    const project = item ? sessionProjectKey(item) : ref.project;
    if (project && !projects.includes(project)) projects.push(project);
    if (item?.title && !titles.includes(item.title)) titles.push(item.title);
  }
  return { projectKeys: projects, sources, sessionTitles: titles };
}

export function candidateRefs(candidate: DistillationCandidate) {
  return candidate.sourceRefs?.length ? candidate.sourceRefs : [{ agent: candidate.agent, sessionId: candidate.sessionId, project: "", startIndex: 0, endIndex: 0 }];
}
