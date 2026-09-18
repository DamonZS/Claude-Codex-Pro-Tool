// @vitest-environment node
import { describe, expect, it, vi } from "vitest";
import { MulticaApiAdapter } from "./multica-api-adapter";
import { type JsonRecord } from "./multica-adapter-dto";
import { ListWebhookDeliveriesResponseSchema, WebhookDeliveryResponseSchema } from "../vendor/multica/packages/core/api/schemas";

const date = "2026-09-18T00:00:00.000Z";
const init = (method: string, body: JsonRecord = {}, key?: string): RequestInit => ({ method, body: JSON.stringify(body), ...(key ? { headers: { "Idempotency-Key": key } } : {}) });
const delivery = { id: "delivery-1", workspace_id: "workspace-1", autopilot_id: "auto-1", trigger_id: "trigger-1", provider: "github", event: "push", dedupe_key: "delivery-hash", dedupe_source: "x-github-delivery", signature_status: "valid", status: "queued", attempt_count: 1, dispatch_attempts: 0, available_at: date, content_type: "application/json", response_status: 202, autopilot_run_id: "run-1", replayed_from_delivery_id: null, error: null, reason_code: null, replay_idempotency_key: null, received_at: date, last_attempt_at: date, created_at: date };

function fixture() {
  const autopilot: JsonRecord = { id: "auto-1", workspace_id: "workspace-1", title: "Webhook", assignee_id: "agent-1", revision: 1, triggers: [] };
  const receipts = new Map<string, { signature: unknown; result: unknown }>();
  const credentials = new Map<string, number>();
  const provisioned = new Set<string>();
  const rotations = new Map<string, { target: string; signature: unknown }>();
  const trigger = (id: string, token: string | null) => ({ ...(autopilot.triggers as JsonRecord[]).find((item) => item.id === id), id, autopilot_id: "auto-1", credential_revision: credentials.get(id), webhook_token: token, webhook_path: `/multica/webhooks/ingress/auto-1/${id}`, webhook_url: `http://127.0.0.1:43127/multica/webhooks/ingress/auto-1/${id}`, created_at: date, updated_at: date });
  const postJson = vi.fn(async (path: string, payload: JsonRecord): Promise<unknown> => {
    if (path === "/multica/workspace/bootstrap") return { status: "ok", workspace: { id: "workspace-1" }, user: { id: "user-1" } };
    if (path === "/multica/workspace/query") return { status: "ok", items: payload.resource === "autopilots" ? [structuredClone(autopilot)] : [], total: payload.resource === "autopilots" ? 1 : 0 };
    if (path === "/multica/workspace/command") {
      const receipt = receipts.get(String(payload.commandId));
      if (receipt && receipt.signature !== payload.commandSignature) return { status: "failed", code: "idempotency_conflict" };
      return { status: "ok", found: !!receipt, result: receipt?.result };
    }
    if (path === "/multica/workspace/upsert") {
      if (autopilot.revision !== payload.expectedRevision) return { status: "failed", code: "revision_conflict" };
      Object.assign(autopilot, payload.entity, { revision: Number(autopilot.revision) + 1 });
      const result = { status: "ok", entity: structuredClone(autopilot) };
      receipts.set(String(payload.commandId), { signature: payload.commandSignature, result });
      return result;
    }
    if (path === "/multica/webhooks/provision") {
      const id = String(payload.triggerId), key = String(payload.commandId);
      if (provisioned.has(key)) return trigger(id, null);
      if (credentials.has(id)) return { status: "failed", code: "webhook_revision_conflict" };
      provisioned.add(key); credentials.set(id, 1);
      return trigger(id, "fixture-one-time-credential");
    }
    if (path === "/multica/webhooks/trigger") return trigger(String(payload.triggerId), null);
    if (path === "/multica/webhooks/rotate") {
      const id = String(payload.triggerId);
      const key = String(payload.commandId), target = `${payload.autopilotId}:${id}`;
      const receipt = rotations.get(key);
      if (receipt) {
        if (receipt.target !== target || receipt.signature !== payload.commandSignature) return { status: "failed", code: "webhook_command_conflict" };
        return { ...trigger(id, null), credential_replay: true };
      }
      if (credentials.get(id) !== payload.expectedRevision) return { status: "failed", code: "webhook_revision_conflict" };
      credentials.set(id, Number(payload.expectedRevision) + 1);
      rotations.set(key, { target, signature: payload.commandSignature });
      return trigger(id, "fixture-rotated-credential");
    }
    if (path === "/multica/webhooks/deliveries") return { deliveries: [delivery], total: 1 };
    if (path === "/multica/webhooks/delivery") return { ...delivery, raw_body: "{}", selected_headers: {}, response_body: null };
    if (path === "/multica/webhooks/replay") return { ...delivery, id: "delivery-replayed", replayed_from_delivery_id: "delivery-1" };
    return { status: "failed", code: "capability_unavailable" };
  });
  const bridge = { postJson };
  return { autopilot, postJson, bridge, credentials, adapter: new MulticaApiAdapter(bridge) };
}

describe("Core webhook management DTOs", () => {
  it("reloads the configured helper URL from Core without caching a provisioned secret", async () => {
    const f = fixture();
    const created = await (await f.adapter.transport("/api/autopilots/auto-1/triggers", init("POST", { kind: "webhook" }, "trigger-1"))).json();
    expect(created.webhook_token).toBeTruthy();
    const reloaded = await (await new MulticaApiAdapter(f.bridge).transport("/api/autopilots/auto-1")).json();
    expect(reloaded.triggers[0]).toMatchObject({ webhook_url: "http://127.0.0.1:43127/multica/webhooks/ingress/auto-1/trigger-1", webhook_token: null });
    expect(JSON.stringify(reloaded)).not.toContain(created.webhook_token);
    expect(JSON.stringify(f.autopilot)).not.toContain(created.webhook_token);
  });
  it("persists trigger filters then provisions a genuine backend credential; replay never regenerates it", async () => {
    const f = fixture();
    const request = init("POST", { kind: "webhook", event_filters: [{ event: "push", actions: ["created"] }] }, "trigger-1");
    const created = await f.adapter.transport("/api/autopilots/auto-1/triggers", request);
    expect(created.status).toBe(200);
    expect(await created.json()).toMatchObject({ id: "trigger-1", event_filters: [{ event: "push", actions: ["created"] }], webhook_token: "fixture-one-time-credential", credential_revision: 1 });
    expect(f.postJson.mock.calls.at(-1)).toEqual(["/multica/webhooks/provision", { autopilotId: "auto-1", triggerId: "trigger-1", commandId: expect.stringMatching(/^[a-f0-9]{64}$/) }]);
    const replayed = await (await new MulticaApiAdapter(f.bridge).transport("/api/autopilots/auto-1/triggers", request)).json();
    expect(replayed.webhook_token).toBeNull();
    expect(f.credentials.get("trigger-1")).toBe(1);
    expect(JSON.stringify(f.autopilot)).not.toContain("fixture-one-time-credential");
    const patch = await f.adapter.transport("/api/autopilots/auto-1/triggers/trigger-1", init("PATCH", { enabled: false }));
    expect(patch.status).toBe(200);
    expect(await patch.json()).toMatchObject({ enabled: false, webhook_token: null, credential_revision: 1 });
    expect(f.postJson.mock.calls.filter(([path]) => path === "/multica/webhooks/provision")).toHaveLength(2);
  });
  it("rotates using credential revision rather than the autopilot entity revision", async () => {
    const f = fixture();
    await f.adapter.transport("/api/autopilots/auto-1/triggers", init("POST", { kind: "api" }, "trigger-1"));
    const result = await f.adapter.transport("/api/autopilots/auto-1/triggers/trigger-1/rotate-webhook-token", init("POST", {}, "rotate-1"));
    expect(result.status).toBe(200);
    expect(await result.json()).toMatchObject({ credential_revision: 2, webhook_token: "fixture-rotated-credential" });
    expect(f.postJson.mock.calls.at(-1)).toEqual(["/multica/webhooks/rotate", { autopilotId: "auto-1", triggerId: "trigger-1", expectedRevision: 1, commandId: "rotate-1", commandSignature: expect.stringMatching(/^[a-f0-9]{64}$/) }]);
    expect((await f.adapter.transport("/api/autopilots/auto-1/triggers/trigger-1/rotate-webhook-token", init("POST", { expected_revision: 1 }))).status).toBe(409);
  });
  it("replays a rotation after adapter recreation without rotating again or recovering plaintext", async () => {
    const f = fixture();
    await f.adapter.transport("/api/autopilots/auto-1/triggers", init("POST", { kind: "api" }, "trigger-1"));
    const path = "/api/autopilots/auto-1/triggers/trigger-1/rotate-webhook-token";
    const request = init("POST", {}, "rotate-once");
    expect((await f.adapter.transport(path, request)).status).toBe(200);
    const replay = await new MulticaApiAdapter(f.bridge).transport(path, request);
    expect(replay.status).toBe(200);
    expect(await replay.json()).toMatchObject({ credential_revision: 2, webhook_token: null, credential_replay: true });
    expect(f.credentials.get("trigger-1")).toBe(2);
    const writes = f.postJson.mock.calls.filter(([route]) => route === "/multica/webhooks/rotate").map(([, payload]) => payload);
    expect(writes.map((payload) => payload.expectedRevision)).toEqual([1, 2]);
    expect(writes[0].commandSignature).toMatch(/^[a-f0-9]{64}$/);
    expect(writes[1].commandSignature).toBe(writes[0].commandSignature);
    const changed = await new MulticaApiAdapter(f.bridge).transport(path, init("POST", { expected_revision: 2 }, "rotate-once"));
    expect(changed.status).toBe(409);
    expect(f.credentials.get("trigger-1")).toBe(2);
  });
  it("preserves upstream delivery schemas and replay identity without claiming execution completed", async () => {
    const f = fixture();
    const listed = await (await f.adapter.transport("/api/autopilots/auto-1/deliveries?limit=20&offset=0")).json();
    expect(ListWebhookDeliveriesResponseSchema.safeParse(listed).success).toBe(true);
    expect(f.postJson.mock.calls.at(-1)).toEqual(["/multica/webhooks/deliveries", { autopilotId: "auto-1", limit: 20, offset: 0 }]);
    expect(listed.deliveries[0]).not.toHaveProperty("raw_body");
    const detail = await (await f.adapter.transport("/api/autopilots/auto-1/deliveries/delivery-1")).json();
    expect(WebhookDeliveryResponseSchema.safeParse(detail).success).toBe(true);
    expect(detail.raw_body).toBe("{}");
    const replay = await (await f.adapter.transport("/api/autopilots/auto-1/deliveries/delivery-1/replay", init("POST", {}, "replay-1"))).json();
    expect(replay).toMatchObject({ id: "delivery-replayed", replayed_from_delivery_id: "delivery-1", status: "queued" });
    expect(f.postJson.mock.calls.at(-1)).toEqual(["/multica/webhooks/replay", { autopilotId: "auto-1", deliveryId: "delivery-1", commandId: "replay-1" }]);
  });
  it("rejects cross-workspace rows, malformed credential replies and backend replay rejection", async () => {
    const f = fixture(), original = f.postJson.getMockImplementation()!;
    f.postJson.mockImplementation(async (path, payload) => path === "/multica/webhooks/delivery" ? { ...delivery, workspace_id: "other" } : path === "/multica/webhooks/replay" ? { status: "failed", code: "webhook_replay_invalid" } : path === "/multica/webhooks/provision" ? {} : original(path, payload));
    expect((await f.adapter.transport("/api/autopilots/auto-1/deliveries/delivery-1")).status).toBe(502);
    expect((await f.adapter.transport("/api/autopilots/auto-1/deliveries/delivery-1/replay", init("POST"))).status).toBe(400);
    const partial = await f.adapter.transport("/api/autopilots/auto-1/triggers", init("POST", { kind: "webhook" }, "trigger-1"));
    expect(partial.status).toBe(502);
    expect(await partial.json()).toMatchObject({ code: "invalid_webhook_trigger", trigger_id: "trigger-1", trigger_saved: true });
  });
  it("returns a real failed delivery as domain data rather than mistaking it for a bridge failure", async () => {
    const f = fixture(), original = f.postJson.getMockImplementation()!;
    f.postJson.mockImplementation(async (path, payload) => path === "/multica/webhooks/delivery" ? { ...delivery, status: "failed", error: "codex_host_unavailable" } : original(path, payload));
    const result = await f.adapter.transport("/api/autopilots/auto-1/deliveries/delivery-1");
    expect(result.status).toBe(200);
    expect(await result.json()).toMatchObject({ id: "delivery-1", status: "failed", error: "codex_host_unavailable" });
  });
});
