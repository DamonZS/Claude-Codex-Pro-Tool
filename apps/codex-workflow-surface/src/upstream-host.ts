export type WorkflowTransport = (path: string, init: RequestInit) => Promise<Response>;

let transport: WorkflowTransport | undefined;
let portal: HTMLElement | undefined;
let forwardedRequests = new WeakMap<RequestInit, RequestInit>();
const safeHeaders = ["content-type", "accept", "idempotency-key", "x-request-id", "x-workspace-slug"];

export function connectWorkflowHost(next: WorkflowTransport, container: HTMLElement): void {
  transport = next;
  portal = container;
}

export function disconnectWorkflowHost(): void {
  transport = undefined;
  portal = undefined;
  forwardedRequests = new WeakMap();
}

export async function workflowFetch(input: string | URL | Request, init: RequestInit = {}): Promise<Response> {
  const path = typeof input === "string" ? input : input instanceof URL ? input.href : input.url;
  if (!transport || !path.startsWith("/api/") || path.includes("\\")) {
    return new Response(JSON.stringify({ code: "capability_unavailable", error: "Workflow capability unavailable" }), {
      status: 503, headers: { "Content-Type": "application/json" },
    });
  }
  const headers = new Headers();
  const originalHeaders = new Headers(init.headers);
  for (const name of safeHeaders) {
    const value = originalHeaders.get(name);
    if (value !== null) headers.set(name, value);
  }
  // Preserve retry identity while stripping credentials and unrelated metadata.
  const forwarded = forwardedRequests.get(init) ?? {};
  Object.assign(forwarded, { method: init.method ?? "GET", body: init.body, signal: init.signal, headers });
  forwardedRequests.set(init, forwarded);
  return transport(path, forwarded);
}

export function workflowPortal(): HTMLElement {
  if (!portal) throw new Error("Workflow surface is not mounted");
  return portal;
}

export function blockedSocket(): WebSocket {
  throw new Error("capability_unavailable: workflow events use the local host");
}
