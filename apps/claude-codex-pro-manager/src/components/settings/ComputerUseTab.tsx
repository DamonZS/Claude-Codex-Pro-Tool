import { useCallback, useEffect, useState } from "react";
import { History, MonitorSmartphone, RefreshCw, ShieldAlert } from "lucide-react";

import type { AppActions } from "@/lib/actions";
import { useI18n } from "@/lib/i18n";
import { invokeCommand } from "@/tauriBridge";
import type { ClaudeDesktopComputerUseStatusResult, CommandResult } from "@/types";

import { SettingsSection, ToggleRow } from "./SettingsControls";

type ComputerUseLogEntry = {
  timestampMs?: number;
  tool?: string;
  ok?: boolean;
  error?: string;
  x?: number;
  y?: number;
  keys?: string[];
  textLength?: number;
};

const TOOL_KEYS = ["screenshot", "click", "move_mouse", "drag", "scroll", "type_text", "press_keys", "cursor_position", "wait"] as const;

function compactPath(path: string) {
  return path.replace(/^[A-Za-z]:[\\/]Users[\\/][^\\/]+(?=[\\/])/, "~");
}

/** Claude Desktop Computer Use: one switch, registration state and recent calls. */
export function ComputerUseTab({ actions, notify }: { actions: AppActions; notify: (tone: "ok" | "error", text: string) => void }) {
  const { t } = useI18n();
  const [status, setStatus] = useState<ClaudeDesktopComputerUseStatusResult | null>(null);
  const [log, setLog] = useState<ComputerUseLogEntry[]>([]);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    const [result, calls] = await Promise.all([
      actions.refreshClaudeDesktopComputerUse(),
      invokeCommand<CommandResult<{ entries: ComputerUseLogEntry[] }>>("get_claude_desktop_computer_use_log").catch(() => null),
    ]);
    if (result) setStatus(result);
    setLog(calls?.entries ?? []);
  }, [actions]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const toggle = async (enabled: boolean) => {
    setBusy(true);
    try {
      const result = await actions.setClaudeDesktopComputerUse(enabled);
      if (result?.status === "ok") {
        setStatus(result);
        notify("ok", enabled ? t("settings.computerUse.enabledHint") : t("settings.computerUse.disabledHint"));
      } else {
        notify("error", result?.message || t("settings.saveFailed"));
      }
      await refresh();
    } finally {
      setBusy(false);
    }
  };

  const supported = status?.supported ?? true;
  const enabled = !!status?.enabled;
  const registered = status?.registeredPaths ?? [];
  const toolLabel = (tool?: string) =>
    (TOOL_KEYS as readonly string[]).includes(tool ?? "") ? t(`settings.computerUse.tool.${tool}` as Parameters<typeof t>[0]) : tool || "—";
  const summary = (entry: ComputerUseLogEntry) => {
    const parts: string[] = [];
    if (typeof entry.x === "number" && typeof entry.y === "number") parts.push(`(${entry.x}, ${entry.y})`);
    if (entry.keys?.length) parts.push(entry.keys.join("+"));
    if (typeof entry.textLength === "number") parts.push(t("settings.computerUse.chars", { count: entry.textLength }));
    if (!entry.ok && entry.error) parts.push(entry.error);
    return parts.join(" · ");
  };

  return (
    <div className="st-tab-body">
      <SettingsSection
        badge={<span className={`st-badge${enabled ? " on" : ""}`}>{enabled ? t("settings.computerUse.on") : t("settings.computerUse.off")}</span>}
        defaultOpen
        description={t("settings.computerUse.description")}
        icon={MonitorSmartphone}
        title={t("settings.computerUse.title")}
        tone="var(--workspace-blue)"
      >
        <ToggleRow
          checked={enabled}
          description={
            supported
              ? t("settings.computerUse.toggleHint")
              : t("settings.computerUse.unsupported", { platform: status?.platform ?? "?" })
          }
          disabled={busy || !supported || !status}
          onChange={(value) => void toggle(value)}
          title={t("settings.computerUse.toggle")}
        />
        <div className="st-rows">
          <p className="st-muted">
            {registered.length
              ? t("settings.computerUse.registered", { count: registered.length })
              : t("settings.computerUse.notRegistered")}
          </p>
          {registered.map((path) => (
            <p className="st-muted cu-path" key={path} title={path}>{compactPath(path)}</p>
          ))}
        </div>
        <ol className="cu-steps">
          <li>{t("settings.computerUse.step1")}</li>
          <li>{t("settings.computerUse.step2")}</li>
          <li>{t("settings.computerUse.step3")}</li>
        </ol>
      </SettingsSection>

      <SettingsSection
        defaultOpen
        description={t("settings.computerUse.safetyDescription")}
        icon={ShieldAlert}
        title={t("settings.computerUse.safety")}
        tone="var(--workspace-warning)"
      >
        <ul className="cu-notes">
          <li>{t("settings.computerUse.stop")}</li>
          <li>{t("settings.computerUse.realMouse")}</li>
          {status?.platform === "macos" ? <li>{t("settings.computerUse.macPermission")}</li> : null}
        </ul>
      </SettingsSection>

      <SettingsSection
        badge={<span className="st-badge">{log.length}</span>}
        defaultOpen
        description={t("settings.computerUse.logDescription")}
        icon={History}
        title={t("settings.computerUse.log")}
        tone="var(--workspace-text-secondary)"
      >
        <div className="st-actions">
          <button className="st-button" disabled={busy} onClick={() => void refresh()} type="button">
            <RefreshCw aria-hidden="true" />
            {t("settings.computerUse.refresh")}
          </button>
        </div>
        {log.length ? (
          <ul className="cu-log">
            {log.map((entry, index) => (
              <li className={entry.ok ? "ok" : "failed"} key={`${entry.timestampMs ?? 0}-${index}`}>
                <time>{entry.timestampMs ? new Date(entry.timestampMs).toLocaleString(undefined, { hour12: false }) : "—"}</time>
                <span>{toolLabel(entry.tool)}</span>
                <em>{entry.ok ? t("settings.computerUse.success") : t("settings.computerUse.failed")}</em>
                <small title={summary(entry)}>{summary(entry)}</small>
              </li>
            ))}
          </ul>
        ) : (
          <p className="st-muted">{t("settings.computerUse.logEmpty")}</p>
        )}
      </SettingsSection>
    </div>
  );
}
