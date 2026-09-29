import type { RequestRecord } from "../types";

export function overviewPeriod(range: "24h" | "7d" | "30d", now: number) {
  const count = range === "24h" ? 24 : range === "7d" ? 7 : 30;
  const start = new Date(now);
  if (range === "24h") {
    start.setMinutes(0, 0, 0);
    start.setHours(start.getHours() - 23);
  } else {
    start.setHours(0, 0, 0, 0);
    start.setDate(start.getDate() - count + 1);
  }
  const boundaries = Array.from({ length: count + 1 }, (_, index) => {
    const date = new Date(start);
    if (range === "24h") date.setHours(date.getHours() + index);
    else date.setDate(date.getDate() + index);
    return date.getTime();
  });
  const previous = new Date(start);
  if (range === "24h") previous.setHours(previous.getHours() - count);
  else previous.setDate(previous.getDate() - count);
  return { start: start.getTime(), previousStart: previous.getTime(), end: now, boundaries };
}

export function overviewTrend(records: readonly RequestRecord[], boundaries: readonly number[]) {
  return boundaries.slice(0, -1).map((start, index) => {
    const rows = records.filter((record) => record.timestamp_ms >= start && record.timestamp_ms < boundaries[index + 1]);
    return {
      requests: rows.length,
      tokens: rows.reduce((sum, record) => sum + (record.total_tokens ?? (record.input_tokens ?? 0) + (record.cached_tokens ?? 0) + (record.cache_creation_tokens ?? 0) + (record.output_tokens ?? 0)), 0),
      input: rows.reduce((sum, record) => sum + (record.input_tokens ?? 0) + (record.cache_creation_tokens ?? 0), 0),
      cached: rows.reduce((sum, record) => sum + (record.cached_tokens ?? 0), 0),
      output: rows.reduce((sum, record) => sum + (record.output_tokens ?? 0), 0),
    };
  });
}

export function overviewBarPosition(index: number, count: number) {
  const slot = 100 / Math.max(1, count);
  return { center: (index + .5) * slot, left: (index + .2) * slot, width: slot * .6 };
}
