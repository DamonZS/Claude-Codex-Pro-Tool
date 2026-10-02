import { useState } from "react";
import { EyeOff, FolderOpen, History, KeyRound, Laptop, Monitor, MonitorUp, Moon, Power, Sun } from "lucide-react";

import { useI18n } from "@/lib/i18n";
import { applyThemePreference, readThemePreference, type ThemePreference } from "@/lib/theme";

import { AGENT_APPS, type AgentAppId, type AppPreferences, type SkillStorageLocation } from "./contract";
import { Segmented, SettingsField, ToggleRow } from "./SettingsControls";

export function GeneralTab({
  preferences,
  update,
  appPaths,
}: {
  preferences: AppPreferences;
  update: (patch: Partial<AppPreferences>) => Promise<boolean>;
  /** Codex / Claude path & launch panel moved from the maintenance page. */
  appPaths: React.ReactNode;
}) {
  const { t } = useI18n();
  const [theme, setTheme] = useState<ThemePreference>(readThemePreference);

  const toggleApp = (id: AgentAppId) => {
    const visible = preferences.visibleApps.includes(id)
      ? preferences.visibleApps.filter((item) => item !== id)
      : [...preferences.visibleApps, id];
    // At least one app must stay visible.
    if (visible.length > 0) void update({ visibleApps: visible });
  };

  const changeSkillStorage = (value: SkillStorageLocation) => {
    if (value === preferences.skillStorageLocation) return;
    if (window.confirm(t("settings.skillStorage.confirm"))) void update({ skillStorageLocation: value });
  };

  return (
    <div className="st-tab-body">
      <SettingsField hint={t("settings.languageHint")} title={t("settings.language")}>
        <Segmented
          label={t("settings.language")}
          onChange={(value) => void update({ language: value })}
          options={[{ value: "zh", label: t("settings.languageZh") }, { value: "en", label: t("settings.languageEn") }]}
          value={preferences.language}
        />
      </SettingsField>

      <SettingsField hint={t("settings.themeHint")} title={t("settings.theme")}>
        <Segmented
          label={t("settings.theme")}
          onChange={(value) => { setTheme(value); applyThemePreference(value); }}
          options={[
            { value: "light", label: t("settings.themeLight"), icon: Sun },
            { value: "dark", label: t("settings.themeDark"), icon: Moon },
            { value: "system", label: t("settings.themeSystem"), icon: Laptop },
          ]}
          value={theme}
        />
      </SettingsField>

      <SettingsField hint={t("settings.appVisibility.description")} title={t("settings.appVisibility.title")}>
        <div className="st-chip-group">
          {AGENT_APPS.map((app) => {
            const on = preferences.visibleApps.includes(app.id);
            return <button aria-pressed={on} className={`st-app-chip${on ? " on" : ""}`} key={app.id} onClick={() => toggleApp(app.id)} type="button">{app.label}</button>;
          })}
        </div>
        <ToggleRow
          checked={preferences.showProjectSwitcher}
          description={t("settings.appVisibility.showProjectSwitcherDescription")}
          icon={FolderOpen}
          onChange={(value) => void update({ showProjectSwitcher: value })}
          title={t("settings.appVisibility.showProjectSwitcher")}
          tone="var(--workspace-success)"
        />
      </SettingsField>

      <SettingsField
        footnote={preferences.skillStorageLocation === "ccp" ? t("settings.skillStorage.ccpHint") : t("settings.skillStorage.unifiedHint")}
        hint={t("settings.skillStorage.description")}
        title={t("settings.skillStorage.title")}
      >
        <Segmented
          label={t("settings.skillStorage.title")}
          onChange={changeSkillStorage}
          options={[{ value: "ccp", label: t("settings.skillStorage.ccp") }, { value: "unified", label: t("settings.skillStorage.unified") }]}
          value={preferences.skillStorageLocation}
        />
      </SettingsField>

      <SettingsField footnote={t("settings.skillSync.symlinkHint")} hint={t("settings.skillSync.description")} title={t("settings.skillSync.title")}>
        <Segmented
          label={t("settings.skillSync.title")}
          onChange={(value) => void update({ skillSyncMethod: value })}
          options={[{ value: "symlink", label: t("settings.skillSync.symlink") }, { value: "copy", label: t("settings.skillSync.copy") }]}
          value={preferences.skillSyncMethod}
        />
      </SettingsField>

      <h3 className="st-group-title"><KeyRound aria-hidden="true" />{t("settings.codexAuth")}</h3>
      <div className="st-rows">
        <ToggleRow
          checked={preferences.preserveCodexOfficialAuthOnSwitch}
          description={t("settings.preserveCodexAuthDescription")}
          icon={KeyRound}
          onChange={(value) => void update({ preserveCodexOfficialAuthOnSwitch: value })}
          title={t("settings.preserveCodexAuth")}
          tone="var(--workspace-blue)"
        />
        <ToggleRow
          checked={preferences.unifyCodexSessionHistory}
          description={t("settings.unifyCodexHistoryDescription")}
          icon={History}
          onChange={(value) => void update({ unifyCodexSessionHistory: value })}
          title={t("settings.unifyCodexHistory")}
          tone="var(--workspace-blue)"
        />
      </div>

      <h3 className="st-group-title"><Monitor aria-hidden="true" />{t("settings.windowBehavior")}</h3>
      <div className="st-rows">
        <ToggleRow checked={preferences.launchOnStartup} description={t("settings.launchOnStartupDescription")} icon={Power} onChange={(value) => void update({ launchOnStartup: value, ...(value ? {} : { silentStartup: false }) })} title={t("settings.launchOnStartup")} tone="var(--workspace-warning)" />
        {preferences.launchOnStartup ? (
          <ToggleRow checked={preferences.silentStartup} description={t("settings.silentStartupDescription")} icon={EyeOff} onChange={(value) => void update({ silentStartup: value })} title={t("settings.silentStartup")} tone="var(--workspace-success)" />
        ) : null}
        <ToggleRow checked={preferences.applyToClaudeCodePlugin} description={t("settings.applyClaudePluginDescription")} icon={MonitorUp} onChange={(value) => void update({ applyToClaudeCodePlugin: value })} title={t("settings.applyClaudePlugin")} tone="oklch(0.68 0.16 300)" />
        <ToggleRow checked={preferences.skipClaudeOnboarding} description={t("settings.skipClaudeOnboardingDescription")} icon={MonitorUp} onChange={(value) => void update({ skipClaudeOnboarding: value })} title={t("settings.skipClaudeOnboarding")} tone="oklch(0.72 0.12 200)" />
        <ToggleRow checked={preferences.minimizeToTrayOnClose} description={t("settings.minimizeToTrayDescription")} icon={Monitor} onChange={(value) => void update({ minimizeToTrayOnClose: value })} title={t("settings.minimizeToTray")} tone="var(--workspace-blue)" />
      </div>

      <SettingsField footnote={t("settings.terminal.fallbackHint")} hint={t("settings.terminal.description")} title={t("settings.terminal.title")}>
        <select aria-label={t("settings.terminal.title")} className="st-select" onChange={(event) => void update({ preferredTerminal: event.target.value as AppPreferences["preferredTerminal"] })} value={preferences.preferredTerminal}>
          <option value="cmd">{t("settings.terminal.cmd")}</option>
          <option value="powershell">{t("settings.terminal.powershell")}</option>
          <option value="wt">{t("settings.terminal.wt")}</option>
        </select>
      </SettingsField>

      <SettingsField hint={t("settings.appPaths.description")} title={t("settings.appPaths.title")}>
        {appPaths}
      </SettingsField>
    </div>
  );
}
