// Applied to pinned upstream sources by vendor-upstream.mjs.
export function adaptWebhookUi(name, text) {
  const replace = (before, after) => {
    if (!text.includes(before)) throw new Error(`Webhook source changed: ${name}`);
    text = text.replace(before, after);
  };
  if (/^packages\/views\/locales\/(en|zh-Hans)\/autopilots\.json$/.test(name)) {
    const value = JSON.parse(text);
    const zh = name.includes("/zh-Hans/");
    Object.assign(value.trigger_row, {
      rotate_url: zh ? "\u8f6e\u6362 Token" : "Rotate token",
      rotate_confirm_title: zh ? "\u8f6e\u6362 Webhook Token" : "Rotate webhook token",
      rotate_confirm_description: zh ? "\u65e7 Token \u5c06\u7acb\u5373\u5931\u6548\uff0cURL \u4fdd\u6301\u4e0d\u53d8\u3002" : "The previous token expires immediately. The URL stays unchanged.",
      rotate_confirm_action: zh ? "\u8f6e\u6362" : "Rotate token",
      toast_rotated: zh ? "Webhook Token \u5df2\u8f6e\u6362" : "Webhook token rotated",
      toast_rotate_failed: zh ? "Token \u8f6e\u6362\u5931\u8d25" : "Token rotation failed",
      copy_token: zh ? "\u590d\u5236 Token" : "Copy token",
      show_token: zh ? "\u663e\u793a Token" : "Show token",
      hide_token: zh ? "\u9690\u85cf Token" : "Hide token",
      dismiss_token: zh ? "\u5173\u95ed Token" : "Dismiss token",
      token_copy_failed: zh ? "Token \u590d\u5236\u5931\u8d25" : "Token copy failed",
      one_time_credential: zh ? "\u4e00\u6b21\u6027 Webhook \u51ed\u636e" : "One-time webhook credential",
    });
    const description = zh ? "\u672c\u673a helper \u5730\u5740\uff0cBearer \u51ed\u636e\u9a8c\u8bc1\u3002" : "Local helper endpoint. Bearer authentication required.";
    value.add_trigger_dialog.webhook_help = description;
    for (const key of ["webhook_help", "webhook_help_create", "webhook_help_edit", "webhook_created_description"]) value.dialog[key] = description;
    value.dialog.webhook_created_warning = zh ? "Token \u4ec5\u672c\u6b21\u663e\u793a\uff0c\u5173\u95ed\u540e\u6e05\u9664\u3002" : "One-time token. Cleared on close.";
    text = JSON.stringify(value, null, 2) + "\n";
  } else if (name === "packages/core/autopilots/webhook.ts") {
    const docStart = text.indexOf("/**");
    const functionStart = text.indexOf("export function buildAutopilotWebhookUrl");
    text = text.slice(0, docStart) + "/** Only the configured Core helper address is usable in the embedded page. */\n" + text.slice(functionStart);
    const start = text.indexOf("  const { trigger, apiBaseUrl, currentOrigin } = params;");
    const end = text.indexOf("/** Fixed-width run");
    if (start < 0 || end < start) throw new Error("Webhook URL source changed");
    text = text.slice(0, start) + `  const { trigger } = params;
  if (trigger.kind !== "webhook" || !trigger.webhook_url) return null;
  try {
    const url = new URL(trigger.webhook_url);
    if (url.protocol !== "http:" || url.hostname !== "127.0.0.1" ||
        url.username || url.password || url.search || url.hash ||
        !/^\\/multica\\/webhooks\\/ingress\\/[A-Za-z0-9_.:-]+\\/[A-Za-z0-9_.:-]+$/.test(url.pathname)) return null;
    return url.href;
  } catch { return null; }
}

` + text.slice(end);
  } else if (name === "packages/core/autopilots/mutations.ts") {
    for (const hook of ["useCreateAutopilotTrigger", "useRotateAutopilotTriggerWebhookToken"]) {
      const marker = `export function ${hook}() {\n  const qc = useQueryClient();\n  const wsId = useWorkspaceId();\n  return useMutation({`;
      replace(marker, marker + "\n    gcTime: 0,");
    }
  } else if (name === "packages/views/autopilots/components/webhook-url-field.tsx") {
    replace('import { Check, Copy, Eye, EyeOff }', 'import { Check, Copy, Eye, EyeOff, X }');
    replace("  const revealed = revealedUrl === url;", '  const localUrl = url.startsWith("http://127.0.0.1:");\n  const revealed = localUrl || revealedUrl === url;');
    replace('      <Button\n        size="icon"\n        variant={s.buttonVariant}', '      {!localUrl && <Button\n        size="icon"\n        variant={s.buttonVariant}');
    replace('      </Button>\n      <Button', '      </Button>}\n      <Button');
    text += `
/** This transient credential is separate from the non-secret helper URL. */
export function WebhookCredentialField({ token, onDismiss }: { token: string | null | undefined; onDismiss: () => void }) {
  const { t } = useT("autopilots");
  const [revealedToken, setRevealedToken] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const revealed = !!token && revealedToken === token;
  if (!token) return null;
  const copy = async () => {
    const ok = await copyText(token);
    setCopied(ok);
    if (!ok) toast.error(t(($) => $.trigger_row.token_copy_failed));
  };
  return (
    <div className="mt-3 space-y-1" role="group" aria-label={t(($) => $.trigger_row.one_time_credential)}>
      <div className="text-caption text-muted-foreground">Authorization: Bearer</div>
      <div className="flex items-center gap-1.5 min-w-0">
        <code className="flex-1 min-w-0 break-all rounded bg-muted px-2 py-1 text-caption font-mono">{revealed ? token : "********"}</code>
        <Button size="icon" variant="ghost" className="h-7 w-7 shrink-0" aria-label={revealed ? t(($) => $.trigger_row.hide_token) : t(($) => $.trigger_row.show_token)} title={revealed ? t(($) => $.trigger_row.hide_token) : t(($) => $.trigger_row.show_token)} onClick={() => setRevealedToken(revealed ? null : token)}>
          {revealed ? <EyeOff className="h-3.5 w-3.5" /> : <Eye className="h-3.5 w-3.5" />}
        </Button>
        <Button size="icon" variant="ghost" className="h-7 w-7 shrink-0" aria-label={t(($) => $.trigger_row.copy_token)} title={t(($) => $.trigger_row.copy_token)} onClick={copy}>
          {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
        </Button>
        <Button size="icon" variant="ghost" className="h-7 w-7 shrink-0" aria-label={t(($) => $.trigger_row.dismiss_token)} title={t(($) => $.trigger_row.dismiss_token)} onClick={() => { setRevealedToken(null); onDismiss(); }}>
          <X className="h-3.5 w-3.5" />
        </Button>
      </div>
    </div>
  );
}
`;
  } else if (name === "packages/views/autopilots/components/autopilot-dialog.tsx") {
    replace('import { api } from "@multica/core/api";\n', '');
    replace('      apiBaseUrl: api.getBaseUrl(),\n      currentOrigin: typeof window !== "undefined" ? window.location.origin : undefined,\n', '');
    replace('import { WebhookUrlField }', 'import { WebhookUrlField, WebhookCredentialField }');
    replace("  const { open, onOpenChange } = props;", `  const { open } = props;
  const onOpenChange = (next: boolean) => {
    if (!next) { setCreatedWebhookTrigger(null); createTrigger.reset(); }
    props.onOpenChange(next);
  };`);
    replace("          setCreatedWebhookTrigger(webhookTrigger);", "          setCreatedWebhookTrigger(webhookTrigger);\n          createTrigger.reset();");
    replace("function WebhookCreatedPanel({", "export function WebhookCreatedPanel({");
    replace('            <WebhookUrlField url={url} size="md" />', '            <WebhookUrlField url={url} size="md" />\n            <WebhookCredentialField token={trigger.webhook_token} onDismiss={onClose} />');
  } else if (name === "packages/views/autopilots/components/autopilot-detail-page.tsx") {
    replace('import { api, clientErrorMessage, dispatchReasonCode }', 'import { clientErrorMessage, dispatchReasonCode }');
    replace('        apiBaseUrl: api.getBaseUrl(),\n        currentOrigin: typeof window !== "undefined" ? window.location.origin : undefined,\n', '');
    replace('import { WebhookUrlField }', 'import { WebhookUrlField, WebhookCredentialField }');
    replace("function TriggerRow({", "export function TriggerRow({");
    replace("  const [rotateOpen, setRotateOpen] = useState(false);", "  const [rotateOpen, setRotateOpen] = useState(false);\n  const [oneTimeToken, setOneTimeToken] = useState<string | null>(null);");
    replace("      await rotateToken.mutateAsync({ autopilotId, triggerId: trigger.id });", "      const rotated = await rotateToken.mutateAsync({ autopilotId, triggerId: trigger.id });\n      setOneTimeToken(rotated.webhook_token ?? null);\n      rotateToken.reset();");
    replace("            />\n          </div>\n        )}\n      </div>\n      {!showWebhookUrlRow", "            />\n            <WebhookCredentialField key={oneTimeToken ?? 'empty'} token={oneTimeToken} onDismiss={() => setOneTimeToken(null)} />\n          </div>\n        )}\n      </div>\n      {!showWebhookUrlRow");
    replace("function AddTriggerDialog({\n  open,\n  onOpenChange,", "export function AddTriggerDialog({\n  open,\n  onOpenChange: changeOpen,");
    replace('  const createTrigger = useCreateAutopilotTrigger();', `  const createTrigger = useCreateAutopilotTrigger();
  const [createdTrigger, setCreatedTrigger] = useState<AutopilotTrigger | null>(null);
  const onOpenChange = (next: boolean) => {
    if (!next) { setCreatedTrigger(null); createTrigger.reset(); }
    changeOpen(next);
  };`);
    replace('        await createTrigger.mutateAsync({\n          autopilotId,\n          kind: "webhook",', '        const created = await createTrigger.mutateAsync({\n          autopilotId,\n          kind: "webhook",');
    replace('        toast.success(t(($) => $.add_trigger_dialog.toast_added_webhook));', '        setCreatedTrigger(created);\n        createTrigger.reset();\n        toast.success(t(($) => $.add_trigger_dialog.toast_added_webhook));\n        return;');
    const marker = '  return (\n    <Dialog open={open} onOpenChange={onOpenChange}>';
    replace(marker, `  if (createdTrigger) return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-sm">
        <DialogTitle>{t(($) => $.dialog.webhook_created_title)}</DialogTitle>
        <WebhookUrlField url={buildAutopilotWebhookUrl({ trigger: createdTrigger }) ?? ""} />
        <WebhookCredentialField token={createdTrigger.webhook_token} onDismiss={() => onOpenChange(false)} />
        <Button onClick={() => onOpenChange(false)}>{t(($) => $.dialog.webhook_created_done)}</Button>
      </DialogContent>
    </Dialog>
  );

${marker}`);
  } else return null;
  return { text, change: "Use the configured local helper URL and separately copyable, transient one-time credentials" };
}
