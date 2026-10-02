import { useCallback, useEffect, useState } from "react";
import { Database, FolderSearch, HardDriveDownload, RotateCcw, ScrollText, Terminal } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";

import { useI18n } from "@/lib/i18n";
import type { MessageKey } from "@/lib/locales/zh";
import { invokeCommand } from "@/tauriBridge";

import {
  AGENT_APPS,
  SETTINGS_COMMANDS,
  type AgentAppId,
  type AppPreferences,
  type AppPreferencesResult,
  type BackupListResult,
  type DataTransferResult,
  type LogLevel,
} from "./contract";
import { SettingsSection, ToggleRow } from "./SettingsControls";

type Notify = (tone: "ok" | "error", text: string) => void;
const INTERVALS = [0, 6, 12, 24, 48, 168];
const RETAIN = [3, 5, 10, 15, 20, 30, 50];
const LOG_LEVELS: LogLevel[] = ["error", "warn", "info", "debug", "trace"];

async function pickDirectory(title: string) {
  try {
    const selected = await open({ directory: true, multiple: false, title });
    return typeof selected === "string" ? selected : "";
  } catch {
    return "";
  }
}

export function AdvancedTab({
  preferences,
  resolvedDirs,
  update,
  notify,
  extraArgs,
  onSaveExtraArgs,
  logs,
  onResetSettings,
}: {
  preferences: AppPreferences;
  resolvedDirs: AppPreferencesResult["resolvedDirs"];
  update: (patch: Partial<AppPreferences>) => Promise<boolean>;
  notify: Notify;
  extraArgs: string[];
  onSaveExtraArgs: (args: string[]) => Promise<void>;
  /** Existing log viewer (LogsScreen). */
  logs: React.ReactNode;
  onResetSettings: () => Promise<void>;
}) {
  const { t } = useI18n();
  const [dirs, setDirs] = useState<Partial<Record<AgentAppId, string>>>(preferences.configDirs);
  const [argsText, setArgsText] = useState(extraArgs.join("\n"));
  const [backups, setBackups] = useState<BackupListResult | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => setDirs(preferences.configDirs), [preferences.configDirs]);
  useEffect(() => setArgsText(extraArgs.join("\n")), [extraArgs]);

  const loadBackups = useCallback(async () => {
    try {
      setBackups(await invokeCommand<BackupListResult>(SETTINGS_COMMANDS.listBackups));
    } catch {
      setBackups(null);
    }
  }, []);
  useEffect(() => {
    void loadBackups();
  }, [loadBackups]);

  const runBackup = async (command: string, args?: Record<string, unknown>, okText?: string) => {
    setBusy(true);
    try {
      const result = await invokeCommand<BackupListResult & DataTransferResult>(command, args);
      if (result?.status !== "ok") throw new Error(result?.message || t("settings.saveFailed"));
      if (okText) notify("ok", okText.replace("{id}", result.safetyBackupId ?? "").replace("{path}", result.path ?? ""));
      if (result.backups) setBackups(result);
      else void loadBackups();
    } catch (error) {
      notify("error", error instanceof Error && error.message ? error.message : t("settings.saveFailed"));
    } finally {
      setBusy(false);
    }
  };

  const saveDirs = async () => {
    const configDirs = Object.fromEntries(Object.entries(dirs).map(([key, value]) => [key, (value ?? "").trim()]).filter(([, value]) => value)) as Partial<Record<AgentAppId, string>>;
    if (await update({ configDirs })) notify("ok", t("settings.configDir.restartRequired"));
  };

  const exportData = async () => {
    const dir = await pickDirectory(t("settings.data.export"));
    if (dir) await runBackup(SETTINGS_COMMANDS.exportData, { request: { dir } }, t("settings.data.exported"));
  };

  const importData = async () => {
    try {
      const selected = await open({ multiple: false, directory: false, filters: [{ name: "CCP backup", extensions: ["db", "json"] }] });
      if (typeof selected !== "string") return;
      if (!window.confirm(t("settings.data.importConfirm"))) return;
      await runBackup(SETTINGS_COMMANDS.importData, { request: { path: selected } }, t("settings.data.imported"));
    } catch {
      // Dialog unavailable (preview); nothing to import.
    }
  };

  const intervalLabel = (hours: number) =>
    hours === 0 ? t("settings.backup.disabled") : hours % 24 === 0 ? t("settings.backup.days", { days: hours / 24 }) : t("settings.backup.hours", { hours });

  return (
    <div className="st-tab-body">
      <SettingsSection description={t("settings.configDir.description")} icon={FolderSearch} title={t("settings.configDir.title")} tone="var(--workspace-blue)">
        <label className="st-input">
          <span>{t("settings.configDir.app")}</span>
          <input readOnly value={resolvedDirs.app} />
        </label>
        <p className="st-muted"><strong>{t("settings.configDir.override")}</strong> · {t("settings.configDir.overrideDescription")}</p>
        {AGENT_APPS.filter((app) => app.id !== "claude-desktop").map((app) => (
          <label className="st-input" key={app.id}>
            <span>{t("settings.configDir.agent", { agent: app.label })}</span>
            <div className="st-inline">
              <input onChange={(event) => setDirs({ ...dirs, [app.id]: event.target.value })} placeholder={resolvedDirs[app.id] ?? ""} value={dirs[app.id] ?? ""} />
              <button className="st-icon-button" onClick={async () => { const dir = await pickDirectory(t("settings.configDir.browse")); if (dir) setDirs({ ...dirs, [app.id]: dir }); }} title={t("settings.configDir.browse")} type="button"><FolderSearch aria-hidden="true" /></button>
              <button className="st-icon-button" onClick={() => setDirs({ ...dirs, [app.id]: "" })} title={t("settings.configDir.reset")} type="button"><RotateCcw aria-hidden="true" /></button>
            </div>
          </label>
        ))}
        <div className="st-actions"><button className="st-button primary" onClick={() => void saveDirs()} type="button">{t("settings.save")}</button></div>
      </SettingsSection>

      <SettingsSection description={t("settings.data.description")} icon={Database} title={t("settings.data.title")} tone="var(--workspace-blue)">
        <p className="st-muted">{t("settings.data.hint")}</p>
        <div className="st-actions">
          <button className="st-button" disabled={busy} onClick={() => void exportData()} type="button">{t("settings.data.export")}</button>
          <button className="st-button" disabled={busy} onClick={() => void importData()} type="button">{t("settings.data.import")}</button>
        </div>
      </SettingsSection>

      <SettingsSection description={t("settings.backup.description")} icon={HardDriveDownload} title={t("settings.backup.title")} tone="var(--workspace-warning)">
        <div className="st-grid-2">
          <label className="st-input">
            <span>{t("settings.backup.interval")}</span>
            <select className="st-select" onChange={(event) => void update({ backupIntervalHours: Number(event.target.value) })} value={preferences.backupIntervalHours}>
              {INTERVALS.map((hours) => <option key={hours} value={hours}>{intervalLabel(hours)}</option>)}
            </select>
          </label>
          <label className="st-input">
            <span>{t("settings.backup.retain")}</span>
            <select className="st-select" onChange={(event) => void update({ backupRetainCount: Number(event.target.value) })} value={preferences.backupRetainCount}>
              {RETAIN.map((count) => <option key={count} value={count}>{count}</option>)}
            </select>
          </label>
        </div>
        <div className="st-actions"><button className="st-button" disabled={busy} onClick={() => void runBackup(SETTINGS_COMMANDS.createBackup)} type="button">{t("settings.backup.create")}</button></div>
        {backups?.backups?.length ? (
          <ul className="st-backups">
            {backups.backups.map((backup) => (
              <li key={backup.id}>
                <span><strong>{backup.name}</strong><small>{new Date(backup.createdAt).toLocaleString()} · {Math.max(1, Math.round(backup.sizeBytes / 1024))} KB</small></span>
                <button className="st-button" disabled={busy} onClick={() => { if (window.confirm(t("settings.backup.restoreConfirm"))) void runBackup(SETTINGS_COMMANDS.restoreBackup, { request: { id: backup.id } }); }} type="button">{t("settings.backup.restore")}</button>
                <button className="st-button" disabled={busy} onClick={() => { const name = window.prompt(t("settings.backup.renamePrompt"), backup.name); if (name?.trim()) void runBackup(SETTINGS_COMMANDS.renameBackup, { request: { id: backup.id, name: name.trim() } }); }} type="button">{t("settings.backup.rename")}</button>
                <button className="st-button danger" disabled={busy} onClick={() => { if (window.confirm(t("settings.backup.deleteConfirm"))) void runBackup(SETTINGS_COMMANDS.deleteBackup, { request: { id: backup.id } }); }} type="button">{t("settings.backup.delete")}</button>
              </li>
            ))}
          </ul>
        ) : <p className="st-muted">{t("settings.backup.empty")}</p>}
      </SettingsSection>

      <SettingsSection description={t("settings.launch.description")} icon={Terminal} title={t("settings.launch.title")} tone="var(--workspace-success)">
        <label className="st-input">
          <span>{t("settings.launch.extraArgs")}</span>
          <textarea onChange={(event) => setArgsText(event.target.value)} rows={4} value={argsText} />
          <small>{t("settings.launch.extraArgsHint")}</small>
        </label>
        <div className="st-actions"><button className="st-button primary" onClick={() => void onSaveExtraArgs(argsText.split("\n").map((line) => line.trim()).filter(Boolean))} type="button">{t("settings.save")}</button></div>
      </SettingsSection>

      <SettingsSection description={t("settings.log.description")} icon={ScrollText} title={t("settings.log.title")} tone="oklch(0.72 0.12 200)">
        <ToggleRow checked={preferences.logEnabled} description={t("settings.log.enabledDescription")} onChange={(value) => void update({ logEnabled: value })} title={t("settings.log.enabled")} />
        <label className="st-input">
          <span>{t("settings.log.level")}</span>
          <select className="st-select" disabled={!preferences.logEnabled} onChange={(event) => void update({ logLevel: event.target.value as LogLevel })} value={preferences.logLevel}>
            {LOG_LEVELS.map((level) => <option key={level} value={level}>{t(`settings.log.level.${level}` as MessageKey)}</option>)}
          </select>
          <small>{t("settings.log.levelDescription")}</small>
        </label>
        {logs}
      </SettingsSection>

      <SettingsSection description={t("settings.reset.description")} icon={RotateCcw} title={t("settings.reset.title")} tone="var(--workspace-danger)">
        <div className="st-actions"><button className="st-button danger" onClick={() => { if (window.confirm(t("settings.reset.confirm"))) void onResetSettings(); }} type="button">{t("settings.reset.action")}</button></div>
      </SettingsSection>
    </div>
  );
}
