/** Theme preference shared by the shell toggle and the settings page. */
export type ThemePreference = "light" | "dark" | "system";

export const THEME_STORAGE_KEY = "ccp-manager-theme";
export const THEME_CHANGE_EVENT = "ccp-theme-change";

export function readThemePreference(): ThemePreference {
  try {
    const value = window.localStorage.getItem(THEME_STORAGE_KEY);
    if (value === "light" || value === "dark" || value === "system") return value;
  } catch {
    // Local storage can be unavailable in hardened WebView contexts.
  }
  return "system";
}

/** Persist and broadcast so the shell re-applies the theme immediately. */
export function applyThemePreference(value: ThemePreference) {
  try {
    window.localStorage.setItem(THEME_STORAGE_KEY, value);
  } catch {
    // The broadcast below still applies the theme for this session.
  }
  window.dispatchEvent(new CustomEvent<ThemePreference>(THEME_CHANGE_EVENT, { detail: value }));
}
