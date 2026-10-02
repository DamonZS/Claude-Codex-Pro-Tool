import { useCallback, useEffect, useRef, useState } from "react";

import { invokeCommand } from "@/tauriBridge";
import { setLanguage } from "@/lib/i18n";

import {
  DEFAULT_APP_PREFERENCES,
  SETTINGS_COMMANDS,
  type AppPreferences,
  type AppPreferencesResult,
  type RoutingConfig,
  type RoutingResult,
} from "./contract";

type Notify = (tone: "ok" | "error", text: string) => void;

/**
 * Device preferences with CC Switch's save model: each change is applied
 * optimistically and persisted at once; a failure rolls back and reports.
 */
export function usePreferences(notify: Notify, failText: string) {
  const [preferences, setPreferences] = useState<AppPreferences>(DEFAULT_APP_PREFERENCES);
  const [resolvedDirs, setResolvedDirs] = useState<AppPreferencesResult["resolvedDirs"]>({ app: "" });
  const [loaded, setLoaded] = useState(false);
  const latest = useRef(preferences);
  latest.current = preferences;

  const load = useCallback(async () => {
    try {
      const result = await invokeCommand<AppPreferencesResult>(SETTINGS_COMMANDS.loadPreferences);
      if (result?.status === "ok" && result.preferences) {
        const next = { ...DEFAULT_APP_PREFERENCES, ...result.preferences };
        setPreferences(next);
        setResolvedDirs(result.resolvedDirs ?? { app: "" });
        setLanguage(next.language);
      }
    } catch {
      // Keep defaults; the page still renders and saves will report errors.
    } finally {
      setLoaded(true);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  /** Apply a partial change immediately and persist it. */
  const update = useCallback(async (patch: Partial<AppPreferences>) => {
    const previous = latest.current;
    const next = { ...previous, ...patch };
    setPreferences(next);
    if (patch.language) setLanguage(patch.language);
    try {
      const result = await invokeCommand<AppPreferencesResult>(SETTINGS_COMMANDS.savePreferences, { request: { preferences: next } });
      if (result?.status !== "ok") throw new Error(result?.message || failText);
      if (result.preferences) setPreferences({ ...DEFAULT_APP_PREFERENCES, ...result.preferences });
      return true;
    } catch (error) {
      setPreferences(previous);
      if (patch.language) setLanguage(previous.language);
      notify("error", error instanceof Error && error.message ? error.message : failText);
      return false;
    }
  }, [failText, notify]);

  return { preferences, resolvedDirs, loaded, update, reload: load };
}

export function useRouting(notify: Notify, failText: string) {
  const [data, setData] = useState<RoutingResult | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      const result = await invokeCommand<RoutingResult>(SETTINGS_COMMANDS.loadRouting);
      if (result?.config) setData(result);
    } catch {
      // Routing backend not available yet; sections show their empty state.
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  // Refresh live stats while the service runs.
  useEffect(() => {
    if (!data?.runtime?.running) return;
    const timer = window.setInterval(() => void load(), 5000);
    return () => window.clearInterval(timer);
  }, [data?.runtime?.running, load]);

  const run = useCallback(async (command: string, args?: Record<string, unknown>) => {
    setBusy(true);
    try {
      const result = await invokeCommand<RoutingResult>(command, args);
      if (result?.status !== "ok") throw new Error(result?.message || failText);
      if (result.config) setData(result);
      else void load();
      return result;
    } catch (error) {
      notify("error", error instanceof Error && error.message ? error.message : failText);
      return null;
    } finally {
      setBusy(false);
    }
  }, [failText, load, notify]);

  const saveConfig = useCallback(
    (config: RoutingConfig) => run(SETTINGS_COMMANDS.saveRouting, { request: { config } }),
    [run],
  );

  return { data, busy, reload: load, run, saveConfig };
}
