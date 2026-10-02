import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { X } from "lucide-react";

import "./settings.css";
import type { AppActions } from "@/lib/actions";
import { useI18n } from "@/lib/i18n";
import type { MessageKey } from "@/lib/locales/zh";
import type { BackendSettings } from "@/types";

import { AdvancedTab } from "./AdvancedTab";
import { GeneralTab } from "./GeneralTab";
import { RoutingTab, type ProviderOption } from "./RoutingTab";
import { usePreferences, useRouting } from "./useSettingsData";

// Settings page laid out after CC Switch 3.20.4 (MIT): general/routing/advanced/about tabs, immediate-effect
// changes with rollback on failure; cloud sync is intentionally not included.

const TABS = ["general", "routing", "advanced", "about"] as const;
export type SettingsTabId = (typeof TABS)[number];

type Toast = { id: number; tone: "ok" | "error"; text: string };

function targetOf(targetApp: string | undefined): ProviderOption["appId"] {
  return targetApp === "claude" ? "claude" : targetApp === "claude-desktop" ? "claude-desktop" : "codex";
}

export function SettingsPage({
  actions,
  settings,
  appPaths,
  logs,
  about,
}: {
  actions: AppActions;
  settings: BackendSettings | null;
  appPaths: React.ReactNode;
  logs: React.ReactNode;
  about: React.ReactNode;
}) {
  const { t } = useI18n();
  const [tab, setTab] = useState<SettingsTabId>("general");
  const [toasts, setToasts] = useState<Toast[]>([]);
  const seq = useRef(0);
  const bodyRef = useRef<HTMLDivElement>(null);

  const notify = useCallback((tone: Toast["tone"], text: string) => {
    const id = ++seq.current;
    setToasts((current) => [...current.slice(-2), { id, tone, text }]);
    window.setTimeout(() => setToasts((current) => current.filter((item) => item.id !== id)), 5000);
  }, []);

  const failText = t("settings.saveFailed");
  const { preferences, resolvedDirs, update } = usePreferences(notify, failText);
  const routing = useRouting(notify, failText);

  useEffect(() => {
    bodyRef.current?.scrollTo({ top: 0 });
  }, [tab]);

  const providers = useMemo<ProviderOption[]>(
    () => (settings?.relayProfiles ?? []).map((profile) => ({ id: profile.id, name: profile.name || profile.id, appId: targetOf(profile.targetApp) })),
    [settings?.relayProfiles],
  );

  const saveExtraArgs = async (args: string[]) => {
    if (!settings) return;
    const result = await actions.saveSettings({ ...settings, codexExtraArgs: args });
    notify(result?.status === "ok" ? "ok" : "error", result?.status === "ok" ? t("settings.saved") : result?.message || failText);
  };

  return (
    <section aria-label={t("settings.title")} className="st-root">
      <div className="st-tabs" role="tablist">
        {TABS.map((id) => (
          <button aria-selected={tab === id} className={tab === id ? "on" : ""} key={id} onClick={() => setTab(id)} role="tab" type="button">
            {t(`settings.tab.${id}` as MessageKey)}
          </button>
        ))}
      </div>

      <div className="st-body" ref={bodyRef}>
        {tab === "general" ? <GeneralTab appPaths={appPaths} preferences={preferences} update={update} /> : null}
        {tab === "routing" ? <RoutingTab busy={routing.busy} notify={notify} providers={providers} routing={routing.data} run={routing.run} saveConfig={routing.saveConfig} /> : null}
        {tab === "advanced" ? (
          <AdvancedTab
            extraArgs={settings?.codexExtraArgs ?? []}
            logs={logs}
            notify={notify}
            onResetSettings={actions.resetSettings}
            onSaveExtraArgs={saveExtraArgs}
            preferences={preferences}
            resolvedDirs={resolvedDirs}
            update={update}
          />
        ) : null}
        {tab === "about" ? <div className="st-tab-body">{about}</div> : null}
      </div>

      {toasts.length ? (
        <div aria-live="polite" className="st-toasts">
          {toasts.map((toast) => (
            <div className={`st-toast ${toast.tone}`} key={toast.id} role="status">
              <span>{toast.text}</span>
              <button aria-label={t("common.cancel")} onClick={() => setToasts((current) => current.filter((item) => item.id !== toast.id))} type="button"><X aria-hidden="true" /></button>
            </div>
          ))}
        </div>
      ) : null}
    </section>
  );
}
