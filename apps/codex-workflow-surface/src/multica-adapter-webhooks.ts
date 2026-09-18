import { AdapterError, integer, keys, record, timestamp, type JsonRecord } from "./multica-adapter-dto";
import { WebhookDeliveryResponseSchema } from "../vendor/multica/packages/core/api/schemas";

export type WebhookPath = `/multica/webhooks/${"provision" | "trigger" | "rotate" | "deliveries" | "delivery" | "replay" | "revoke"}`;
type Context = { workspaceId: string; commandId: string; commandSignature?: string; signal?: AbortSignal | null };
const controls = ["command_id", "idempotency_key", "expected_revision"];
const response = (body: unknown) => new Response(JSON.stringify(body), { headers: { "Content-Type": "application/json" } });

export function webhookTrigger(value: unknown, autopilotId: string, triggerId: string): JsonRecord {
  const trigger = record(value);
  if (trigger.id !== triggerId || trigger.autopilot_id !== autopilotId || !["webhook", "api"].includes(String(trigger.kind)) || typeof trigger.enabled !== "boolean" || !Number.isSafeInteger(trigger.credential_revision) || Number(trigger.credential_revision) < 1 || !(trigger.webhook_token === null || typeof trigger.webhook_token === "string" && trigger.webhook_token.length > 0) || !timestamp(trigger.created_at) || !timestamp(trigger.updated_at)) throw new AdapterError(502, "invalid_webhook_trigger");
  return trigger;
}

export function validateEventFilters(value: unknown): void {
  if (value == null) return;
  if (!Array.isArray(value) || value.length > 32) throw new AdapterError(400, "invalid_event_filters");
  for (const item of value) {
    if (!item || typeof item !== "object" || Array.isArray(item)) throw new AdapterError(400, "invalid_event_filters");
    const filter = record(item);
    keys(filter, ["event", "actions"]);
    if (typeof filter.event !== "string" || !filter.event.trim() || filter.event.length > 128 || filter.actions !== undefined && (!Array.isArray(filter.actions) || filter.actions.length > 32 || !filter.actions.every((action) => typeof action === "string" && action.length > 0 && action.length <= 128))) throw new AdapterError(400, "invalid_event_filters");
  }
}

export class MulticaWebhookAdapter {
  constructor(private readonly call: (path: WebhookPath, payload: JsonRecord, signal?: AbortSignal | null) => Promise<JsonRecord>) {}

  private delivery(value: unknown, autopilotId: string, context: Context): JsonRecord {
    const parsed = WebhookDeliveryResponseSchema.safeParse(value);
    if (!parsed.success || !parsed.data.id || !timestamp(parsed.data.created_at) || parsed.data.workspace_id !== context.workspaceId || parsed.data.autopilot_id !== autopilotId) throw new AdapterError(502, "invalid_webhook_delivery");
    return record(value);
  }

  async route(parts: string[], method: string, params: JsonRecord, data: JsonRecord, context: Context): Promise<Response | null> {
    const [, autopilotId, action, childId, childAction] = parts;
    if (action === "triggers" && childAction === "rotate-webhook-token" && parts.length === 5 && method === "POST") {
      keys(params, []); keys(data, controls);
      const current = webhookTrigger(await this.call("/multica/webhooks/trigger", { autopilotId, triggerId: childId }, context.signal), autopilotId, childId);
      const rotated = await this.call("/multica/webhooks/rotate", { autopilotId, triggerId: childId, expectedRevision: integer(data.expected_revision ?? current.credential_revision, 0, Number.MAX_SAFE_INTEGER), commandId: context.commandId, commandSignature: context.commandSignature }, context.signal);
      return response(webhookTrigger(rotated, autopilotId, childId));
    }
    if (action !== "deliveries") return null;
    if (method === "GET" && parts.length === 3) {
      keys(params, ["limit", "offset"]);
      const limit = integer(params.limit, 50, 100), offset = integer(params.offset, 0, 256);
      if (!limit) throw new AdapterError(400, "invalid_limit");
      const result = await this.call("/multica/webhooks/deliveries", { autopilotId, limit, offset }, context.signal);
      if (!Array.isArray(result.deliveries) || !Number.isSafeInteger(result.total) || Number(result.total) < result.deliveries.length || result.deliveries.length > limit) throw new AdapterError(502, "invalid_webhook_deliveries");
      return response({ deliveries: result.deliveries.map((item) => this.delivery(item, autopilotId, context)), total: result.total });
    }
    if (parts.length === 4 && method === "GET" || parts.length === 5 && method === "POST" && childAction === "replay") {
      keys(params, []); keys(data, controls);
      const replay = method === "POST";
      const result = await this.call(replay ? "/multica/webhooks/replay" : "/multica/webhooks/delivery", { autopilotId, deliveryId: childId, ...(replay ? { commandId: context.commandId } : {}) }, context.signal);
      const delivery = this.delivery(result, autopilotId, context);
      if (replay ? delivery.replayed_from_delivery_id !== childId || delivery.id === childId : delivery.id !== childId) throw new AdapterError(502, "invalid_webhook_delivery");
      return response(delivery);
    }
    throw new AdapterError(503, "capability_unavailable");
  }
}
