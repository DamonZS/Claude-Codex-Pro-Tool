/**
 * Adapted from AITRACKER's modules/distillation/presentation/distill/materials.ts.
 * The workbench keeps selection as session refs and derives project rows from
 * the same range-filtered set, so a project selection never leaks hidden days.
 */
import type { DistillationWorkbenchSession } from "@/types";

export type DistillationTimeRange = "today" | "7" | "30" | "all";
export type DistillationMaterialGranularity = "session" | "project";

export interface DistillationProjectMaterial {
  readonly key: string;
  readonly projectKey: string;
  readonly sources: readonly string[];
  readonly sessions: readonly DistillationWorkbenchSession[];
  readonly last: string;
}

export const EST_TOKENS_PER_TURN = 900;

function startOfLocalDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

function startOfNextLocalDay(date: Date): Date {
  const next = startOfLocalDay(date);
  next.setDate(next.getDate() + 1);
  return next;
}

export function filterDistillationSessions(
  sessions: readonly DistillationWorkbenchSession[],
  range: DistillationTimeRange,
  now = new Date(),
): readonly DistillationWorkbenchSession[] {
  if (range === "all") return sessions;
  const today = startOfLocalDay(now);
  const tomorrow = startOfNextLocalDay(now);
  const days = range === "today" ? 0 : Number(range) - 1;
  const cutoff = new Date(today);
  cutoff.setDate(cutoff.getDate() - days);
  return sessions.filter((session) => {
    const startedAt = new Date(session.startedAt ?? session.updatedAt);
    return (
      !Number.isNaN(startedAt.getTime()) &&
      startedAt >= cutoff &&
      startedAt < tomorrow
    );
  });
}

export function groupDistillationSessionsByProject(
  sessions: readonly DistillationWorkbenchSession[],
): readonly DistillationProjectMaterial[] {
  const groups = new Map<string, DistillationProjectMaterial>();
  for (const session of sessions) {
    if (session.isGitProject !== true) continue;
    const projectKey = session.projectKey ?? session.project;
    if (!projectKey.trim()) continue;
    const current = groups.get(projectKey);
    if (current) {
      groups.set(projectKey, {
        ...current,
        sources: [...new Set([...current.sources, session.agent])],
        sessions: [...current.sessions, session],
        last:
          session.startedAt && session.startedAt > current.last
            ? session.startedAt
            : current.last,
      });
    } else {
      groups.set(projectKey, {
        key: projectKey,
        projectKey,
        sources: [session.agent],
        sessions: [session],
        last: session.startedAt ?? session.updatedAt,
      });
    }
  }
  return [...groups.values()];
}

export function materialKeyOf(item: {
  readonly agent: string;
  readonly sessionId: string;
}): string {
  return `${item.agent}:${item.sessionId}`;
}

export function toggleMaterialSelection(
  current: ReadonlySet<string>,
  key: string,
): ReadonlySet<string> {
  const next = new Set(current);
  if (next.has(key)) next.delete(key);
  else next.add(key);
  return next;
}

export function toggleProjectSelection(
  current: ReadonlySet<string>,
  sessionKeys: readonly string[],
): ReadonlySet<string> {
  const uniqueKeys = [...new Set(sessionKeys)];
  const selectedEverySession =
    uniqueKeys.length > 0 && uniqueKeys.every((key) => current.has(key));
  const next = new Set(current);
  if (selectedEverySession) {
    for (const key of uniqueKeys) next.delete(key);
  } else {
    for (const key of uniqueKeys) next.add(key);
  }
  return next;
}
