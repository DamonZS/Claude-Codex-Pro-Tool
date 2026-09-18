import { fireEvent, render, cleanup, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { I18nProvider } from "@multica/core/i18n/provider";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import en from "../vendor/multica/packages/views/locales/en/autopilots.json";
import type { AutopilotTrigger } from "@multica/core/types";
import { buildAutopilotWebhookUrl } from "@multica/core/autopilots/webhook";
import { WebhookCreatedPanel } from "@multica/views/autopilots/components/autopilot-dialog";
import { AddTriggerDialog, TriggerRow } from "@multica/views/autopilots/components/autopilot-detail-page";
import { WebhookCredentialField } from "@multica/views/autopilots/components/webhook-url-field";
import { connectWorkflowHost, disconnectWorkflowHost } from "./upstream-host";

const mutation = vi.hoisted(() => ({ mutateAsync: vi.fn(), reset: vi.fn(), isPending: false }));
vi.mock("@multica/core/hooks", async (importOriginal) => ({
  ...await importOriginal<typeof import("@multica/core/hooks")>(),
  useWorkspaceId: () => "workspace-1",
}));
vi.mock("@multica/views/autopilots/components/schedule-editor/schedule-editor", () => ({ ScheduleEditor: () => null }));
vi.mock("@multica/core/autopilots/mutations", async (importOriginal) => ({
  ...await importOriginal<typeof import("@multica/core/autopilots/mutations")>(),
  useDeleteAutopilotTrigger: () => ({ mutateAsync: vi.fn() }),
  useRotateAutopilotTriggerWebhookToken: () => mutation,
  useCreateAutopilotTrigger: () => mutation,
}));

const url = "http://127.0.0.1:43127/multica/webhooks/ingress/auto-1/trigger-1";
const token = "synthetic-one-time-webhook-credential";
const trigger: AutopilotTrigger = {
  id: "trigger-1", autopilot_id: "auto-1", kind: "webhook", enabled: true,
  label: null,
  webhook_url: url, webhook_path: "/multica/webhooks/ingress/auto-1/trigger-1", webhook_token: null,
  cron_expression: null, timezone: null, next_run_at: null, last_fired_at: null,
  created_at: "2026-09-18T00:00:00Z", updated_at: "2026-09-18T00:00:00Z",
};
const copy = vi.fn().mockResolvedValue(undefined);
const client = new QueryClient();
const wrapper = ({ children }: { children: React.ReactNode }) => <QueryClientProvider client={client}><I18nProvider locale="en" resources={{ en: { autopilots: en } }}>{children}</I18nProvider></QueryClientProvider>;
beforeEach(() => {
  mutation.mutateAsync.mockReset(); mutation.reset.mockReset(); copy.mockClear();
  Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: copy } });
  connectWorkflowHost(async () => new Response("{}"), document.body);
});
afterEach(() => { cleanup(); disconnectWorkflowHost(); vi.restoreAllMocks(); });

describe("upstream webhook credential controls", () => {
  it("uses only the authoritative helper URL, with no app-origin or token fallback", () => {
    expect(buildAutopilotWebhookUrl({ trigger, currentOrigin: "app://codex" })).toBe(url);
    for (const value of [null, "app://codex/hook", `${url}?token=secret`, "https://external.invalid/hook"]) {
      expect(buildAutopilotWebhookUrl({ trigger: { ...trigger, webhook_url: value, webhook_token: token }, currentOrigin: "app://codex" })).toBeNull();
    }
  });

  it("provision panel copies the non-secret URL and token separately, masking until explicit reveal", async () => {
    const onClose = vi.fn();
    const ui = render(<WebhookCreatedPanel trigger={{ ...trigger, webhook_token: token }} onClose={onClose} />, { wrapper });
    expect(ui.container.textContent).toContain(url);
    expect(ui.container.innerHTML).not.toContain(token);
    fireEvent.click(ui.getByRole("button", { name: en.trigger_row.copy_url }));
    await waitFor(() => expect(copy).toHaveBeenCalledWith(url));
    fireEvent.click(ui.getByRole("button", { name: "Copy token" }));
    await waitFor(() => expect(copy).toHaveBeenCalledWith(token));
    expect(ui.container.innerHTML).not.toContain(token);
    fireEvent.click(ui.getByRole("button", { name: "Show token" }));
    expect(ui.container.textContent).toContain(token);
    fireEvent.click(ui.getByRole("button", { name: "Dismiss token" }));
    expect(onClose).toHaveBeenCalledOnce();
    ui.unmount();
    const reopened = render(<WebhookCreatedPanel trigger={trigger} onClose={onClose} />, { wrapper });
    expect(reopened.queryByRole("button", { name: "Copy token" })).toBeNull();
    expect(reopened.container.innerHTML).not.toContain(token);
  });

  it("rotation consumes the mutation result once and dismiss/reload remove the secret", async () => {
    mutation.mutateAsync.mockResolvedValue({ ...trigger, webhook_token: token });
    const ui = render(<TriggerRow trigger={trigger} autopilotId="auto-1" canWrite />, { wrapper });
    fireEvent.click(ui.getByTitle(en.trigger_row.rotate_url));
    fireEvent.click(await ui.findByRole("button", { name: en.trigger_row.rotate_confirm_action }));
    const copyToken = await ui.findByRole("button", { name: "Copy token" });
    expect(mutation.reset).toHaveBeenCalledOnce();
    expect(ui.container.innerHTML).not.toContain(token);
    fireEvent.click(copyToken);
    await waitFor(() => expect(copy).toHaveBeenCalledWith(token));
    fireEvent.click(ui.getByRole("button", { name: "Dismiss token" }));
    expect(ui.queryByRole("button", { name: "Copy token" })).toBeNull();
    ui.unmount();
    const reopened = render(<TriggerRow trigger={trigger} autopilotId="auto-1" canWrite />, { wrapper });
    expect(reopened.queryByRole("button", { name: "Copy token" })).toBeNull();
  });

  it("adding a webhook to an existing autopilot retains its provision receipt until close", async () => {
    mutation.mutateAsync.mockResolvedValue({ ...trigger, webhook_token: token });
    const onOpenChange = vi.fn();
    const ui = render(<AddTriggerDialog open autopilotId="auto-1" onOpenChange={onOpenChange} />, { wrapper });
    fireEvent.click(ui.getByRole("button", { name: en.add_trigger_dialog.type_webhook }));
    fireEvent.click(ui.getByRole("button", { name: en.add_trigger_dialog.submit }));
    fireEvent.click(await ui.findByRole("button", { name: "Copy token" }));
    await waitFor(() => expect(copy).toHaveBeenCalledWith(token));
    expect(ui.baseElement.innerHTML).not.toContain(token);
    expect(ui.baseElement.textContent).toContain(url);
    expect(mutation.reset).toHaveBeenCalledOnce();
    fireEvent.click(ui.getByRole("button", { name: en.dialog.webhook_created_done }));
    expect(onOpenChange).toHaveBeenCalledWith(false);
    expect(ui.queryByRole("button", { name: "Copy token" })).toBeNull();
  });

  it("does not reveal replacement tokens or log clipboard failures", async () => {
    const errors = vi.spyOn(console, "error").mockImplementation(() => undefined);
    const logs = vi.spyOn(console, "log").mockImplementation(() => undefined);
    const ui = render(<WebhookCredentialField token={token} onDismiss={vi.fn()} />, { wrapper });
    fireEvent.click(ui.getByRole("button", { name: "Show token" }));
    ui.rerender(<WebhookCredentialField token="replacement-credential" onDismiss={vi.fn()} />);
    expect(ui.container.innerHTML).not.toContain("replacement-credential");
    copy.mockRejectedValueOnce(new Error("clipboard-failed"));
    fireEvent.click(ui.getByRole("button", { name: "Copy token" }));
    await waitFor(() => expect(copy).toHaveBeenCalled());
    expect(errors).not.toHaveBeenCalled(); expect(logs).not.toHaveBeenCalled();
  });
});
