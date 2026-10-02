/**
 * Minimal zh/en i18n. The language preference is persisted by the backend
 * (`AppPreferences.language`) and mirrored here so every screen can read it
 * synchronously. Untranslated keys fall back to Chinese, then to the key.
 */
import { useSyncExternalStore } from "react";

import type { UiLanguage } from "@/components/settings/contract";
import { en } from "@/lib/locales/en";
import { zh, type MessageKey } from "@/lib/locales/zh";

const STORAGE_KEY = "ccp-manager-language";
const listeners = new Set<() => void>();

function readStored(): UiLanguage {
  try {
    return window.localStorage.getItem(STORAGE_KEY) === "en" ? "en" : "zh";
  } catch {
    return "zh";
  }
}

let current: UiLanguage = typeof window === "undefined" ? "zh" : readStored();

export function getLanguage(): UiLanguage {
  return current;
}

export function setLanguage(language: UiLanguage) {
  if (language === current) return;
  current = language;
  try {
    window.localStorage.setItem(STORAGE_KEY, language);
  } catch {
    // Persistence is best-effort; the backend preference stays authoritative.
  }
  document.documentElement.lang = language === "en" ? "en" : "zh-CN";
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function translate(key: MessageKey, params?: Record<string, string | number>, language = current) {
  const table = language === "en" ? en : zh;
  let text: string = table[key] ?? zh[key] ?? key;
  if (params) {
    for (const [name, value] of Object.entries(params)) text = text.replaceAll(`{${name}}`, String(value));
  }
  return text;
}

export function useI18n() {
  const language = useSyncExternalStore(subscribe, getLanguage, () => "zh" as UiLanguage);
  return {
    language,
    t: (key: MessageKey, params?: Record<string, string | number>) => translate(key, params, language),
  };
}
