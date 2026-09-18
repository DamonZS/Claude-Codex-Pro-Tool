/* CCP modification: Use the configured local helper URL and separately copyable, transient one-time credentials. Upstream attribution: vendor/multica/NOTICE. */
import type { AutopilotTrigger } from "../types";

/** Only the configured Core helper address is usable in the embedded page. */
export function buildAutopilotWebhookUrl(params: {
  trigger: Pick<AutopilotTrigger, "kind" | "webhook_token" | "webhook_path" | "webhook_url">;
  apiBaseUrl?: string;
  currentOrigin?: string;
}): string | null {
  const { trigger } = params;
  if (trigger.kind !== "webhook" || !trigger.webhook_url) return null;
  try {
    const url = new URL(trigger.webhook_url);
    if (url.protocol !== "http:" || url.hostname !== "127.0.0.1" ||
        url.username || url.password || url.search || url.hash ||
        !/^\/multica\/webhooks\/ingress\/[A-Za-z0-9_.:-]+\/[A-Za-z0-9_.:-]+$/.test(url.pathname)) return null;
    return url.href;
  } catch { return null; }
}

/** Fixed-width run — never derived from the token, so the mask leaks no length. */
const WEBHOOK_URL_MASK = "••••••••••••";

/**
 * Mask the secret part of a webhook URL for display.
 *
 * Only the trailing token segment is a credential: anyone holding it can fire
 * the autopilot. The origin and the `/api/webhooks/autopilots/` prefix carry no
 * secret, so they stay readable and the value is still recognizable as this
 * trigger's URL while hidden. Falls back to the bare mask when the URL has no
 * separable last segment.
 */
export function maskAutopilotWebhookUrl(url: string): string {
  const cut = url.lastIndexOf("/");
  if (cut < 0 || cut === url.length - 1) return WEBHOOK_URL_MASK;
  return url.slice(0, cut + 1) + WEBHOOK_URL_MASK;
}
