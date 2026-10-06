// Session Index API - TypeScript types and functions

export interface SessionIndex {
  session_id: string;
  project: string;
  title: string;
  source: "claude" | "codex";
  created_at: number; // Unix timestamp (ms)
  updated_at: number;
  message_count: number;
  file_path: string;
  file_mtime: number;
  indexed_at: number;
}

export interface MessageIndex {
  message_id: string;
  session_id: string;
  role: "user" | "assistant";
  created_at: number;
  content_preview: string | null;
  token_estimate: number;
  line_number: number;
}

export interface ProjectStats {
  project_name: string;
  session_count: number;
  message_count: number;
  last_activity: number;
  source_mask: number; // 1=claude, 2=codex, 3=both
}

export interface SessionFilter {
  project?: string;
  source?: "claude" | "codex";
  time_range?: [number, number]; // [start, end] Unix timestamps
  search_query?: string;
}

export interface PagedResult<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export interface ScanResult {
  files_indexed: number;
  sessions_indexed: number;
  messages_indexed: number;
  files_failed: number;
}

// Tauri command imports
import { invoke } from "@tauri-apps/api/core";

/**
 * Initialize session index database
 */
export async function initSessionIndex(): Promise<string> {
  return await invoke("init_session_index");
}

/**
 * Perform initial scan of all sessions
 */
export async function scanSessions(): Promise<ScanResult> {
  return await invoke("scan_sessions");
}

/**
 * Query sessions with filters and pagination
 */
export async function querySessions(
  filter: SessionFilter,
  page: number = 0,
  pageSize: number = 20
): Promise<PagedResult<SessionIndex>> {
  return await invoke("query_sessions", {
    filter,
    page,
    pageSize,
  });
}

/**
 * Get messages for a specific session
 */
export async function getSessionMessages(
  sessionId: string
): Promise<MessageIndex[]> {
  return await invoke("get_session_messages", { sessionId });
}

/**
 * Get project statistics
 */
export async function getProjectStats(): Promise<ProjectStats[]> {
  return await invoke("get_project_stats");
}

/**
 * Get a single session by ID
 */
export async function getSession(
  sessionId: string
): Promise<SessionIndex | null> {
  return await invoke("get_session", { sessionId });
}

/**
 * Index a specific session file (for incremental updates)
 */
export async function indexSessionFile(
  filePath: string,
  source: "claude" | "codex"
): Promise<ScanResult> {
  return await invoke("index_session_file", { filePath, source });
}

/**
 * Format timestamp to human-readable string
 */
export function formatTimestamp(ts: number): string {
  return new Date(ts).toLocaleString();
}

/**
 * Format relative time (e.g., "2 hours ago")
 */
export function formatRelativeTime(ts: number): string {
  const now = Date.now();
  const diff = now - ts;
  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days > 0) return `${days} 天前`;
  if (hours > 0) return `${hours} 小时前`;
  if (minutes > 0) return `${minutes} 分钟前`;
  return "刚刚";
}

/**
 * Get source label
 */
export function getSourceLabel(source: "claude" | "codex"): string {
  return source === "claude" ? "Claude Desktop" : "Codex";
}

/**
 * Get source mask label
 */
export function getSourceMaskLabel(mask: number): string {
  if (mask === 3) return "Claude + Codex";
  if (mask === 2) return "Codex";
  if (mask === 1) return "Claude";
  return "Unknown";
}
