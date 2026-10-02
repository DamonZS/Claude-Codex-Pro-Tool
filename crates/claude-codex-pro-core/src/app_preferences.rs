//! Device-level application preferences.
//!
//! The JSON shape mirrors `AppPreferences` / `AppPreferencesResult` in
//! `apps/claude-codex-pro-manager/src/components/settings/contract.ts`
//! (camelCase keys, same defaults). The manager's `load_app_preferences` and
//! `save_app_preferences` commands are thin wrappers around the store below.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

/// Preference file name inside the CCP application state directory.
const PREFERENCES_FILE: &str = "app-preferences.json";
const PREFERENCES_TEMP_FILE: &str = "app-preferences.json.tmp";

const LANGUAGE_ZH: &str = "zh";
const LANGUAGE_EN: &str = "en";

const SKILL_STORAGE_CCP: &str = "ccp";
const SKILL_STORAGE_UNIFIED: &str = "unified";

const SKILL_SYNC_SYMLINK: &str = "symlink";
const SKILL_SYNC_COPY: &str = "copy";

const TERMINAL_CMD: &str = "cmd";
const TERMINAL_POWERSHELL: &str = "powershell";
const TERMINAL_WT: &str = "wt";

const LOG_LEVELS: [&str; 5] = ["error", "warn", "info", "debug", "trace"];
const BACKUP_INTERVAL_HOURS: [u32; 6] = [0, 6, 12, 24, 48, 168];
const BACKUP_RETAIN_COUNTS: [u32; 7] = [3, 5, 10, 15, 20, 30, 50];

/// Agent application ids, in the same order as `AGENT_APPS` in `contract.ts`.
pub const AGENT_IDS: [&str; 12] = [
    "claude",
    "claude-desktop",
    "codex",
    "gemini",
    "grok",
    "opencode",
    "openclaw",
    "hermes",
    "pi",
    "mcode",
    "workbuddy",
    "cursor",
];

/// Device-level preferences shared with the settings UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppPreferences {
    pub language: String,
    pub visible_apps: Vec<String>,
    pub show_project_switcher: bool,
    pub skill_storage_location: String,
    pub skill_sync_method: String,
    pub preserve_codex_official_auth_on_switch: bool,
    pub unify_codex_session_history: bool,
    pub launch_on_startup: bool,
    pub silent_startup: bool,
    pub apply_to_claude_code_plugin: bool,
    pub skip_claude_onboarding: bool,
    pub minimize_to_tray_on_close: bool,
    pub preferred_terminal: String,
    pub config_dirs: BTreeMap<String, String>,
    pub backup_interval_hours: u32,
    pub backup_retain_count: u32,
    pub log_enabled: bool,
    pub log_level: String,
}

impl Default for AppPreferences {
    /// Mirrors `DEFAULT_APP_PREFERENCES` in `contract.ts`.
    fn default() -> Self {
        Self {
            language: LANGUAGE_ZH.to_string(),
            visible_apps: default_visible_apps(),
            show_project_switcher: true,
            skill_storage_location: SKILL_STORAGE_CCP.to_string(),
            skill_sync_method: SKILL_SYNC_SYMLINK.to_string(),
            preserve_codex_official_auth_on_switch: false,
            unify_codex_session_history: false,
            launch_on_startup: false,
            silent_startup: false,
            apply_to_claude_code_plugin: false,
            skip_claude_onboarding: false,
            minimize_to_tray_on_close: true,
            preferred_terminal: TERMINAL_CMD.to_string(),
            config_dirs: BTreeMap::new(),
            backup_interval_hours: 24,
            backup_retain_count: 10,
            log_enabled: true,
            log_level: "info".to_string(),
        }
    }
}

/// The default visibility list: every agent, in contract order.
fn default_visible_apps() -> Vec<String> {
    AGENT_IDS.iter().map(|id| (*id).to_string()).collect()
}

fn is_agent_id(value: &str) -> bool {
    AGENT_IDS.contains(&value)
}

fn matches_choice(value: &str, choices: &[&str]) -> bool {
    choices.contains(&value)
}

/// Everything the manager persists, plus the directory the file lives in.
#[derive(Debug, Clone)]
pub struct PreferencesStore {
    path: PathBuf,
}

impl PreferencesStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Default location: `<app state dir>/app-preferences.json`.
    pub fn default_path() -> PathBuf {
        crate::paths::default_app_state_dir().join(PREFERENCES_FILE)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read the preferences file.
    ///
    /// A missing file, or an unreadable/unparsable one, yields the defaults.
    /// A corrupt file is preserved next to the original as
    /// `app-preferences.json.corrupt-<unix millis>.bak` so the user's data is
    /// never silently lost.
    pub fn load(&self) -> AppPreferences {
        let raw = match std::fs::read_to_string(&self.path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return AppPreferences::default();
            }
            Err(error) => {
                eprintln!(
                    "读取偏好设置失败，使用默认值：{}（{error}）",
                    self.path.display()
                );
                return AppPreferences::default();
            }
        };

        match serde_json::from_str::<AppPreferences>(&raw) {
            Ok(prefs) => normalize(prefs),
            Err(error) => {
                let backup = self.back_up_corrupt_file();
                match backup {
                    Some(backup) => eprintln!(
                        "偏好设置解析失败，已备份为 {} 并使用默认值：{error}",
                        backup.display()
                    ),
                    None => eprintln!(
                        "偏好设置解析失败，备份失败，使用默认值：{}（{error}）",
                        self.path.display()
                    ),
                }
                AppPreferences::default()
            }
        }
    }

    /// Normalize and persist the preferences atomically (temp file + rename).
    ///
    /// Returns the normalized value that was written, so callers can echo the
    /// effective preferences back to the UI.
    pub fn save(&self, prefs: AppPreferences) -> anyhow::Result<AppPreferences> {
        let prefs = normalize(prefs);
        let parent = self
            .path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        std::fs::create_dir_all(&parent)
            .with_context(|| format!("创建偏好设置目录失败：{}", parent.display()))?;

        let encoded = serde_json::to_string_pretty(&prefs).context("序列化偏好设置失败")?;
        let temp_path = parent.join(PREFERENCES_TEMP_FILE);
        std::fs::write(&temp_path, encoded)
            .with_context(|| format!("写入偏好设置临时文件失败：{}", temp_path.display()))?;
        std::fs::rename(&temp_path, &self.path).with_context(|| {
            format!(
                "替换偏好设置文件失败：{} → {}",
                temp_path.display(),
                self.path.display()
            )
        })?;

        Ok(prefs)
    }

    /// Copy a corrupt preferences file aside. Returns the backup path.
    fn back_up_corrupt_file(&self) -> Option<PathBuf> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis())
            .unwrap_or_default();
        let backup = self
            .path
            .with_file_name(format!("{PREFERENCES_FILE}.corrupt-{timestamp}.bak"));
        std::fs::copy(&self.path, &backup).ok()?;
        Some(backup)
    }
}

/// Clean up values coming from disk or from the UI.
///
/// Invalid enum values fall back to their defaults, unknown agent ids are
/// dropped, numeric choices are snapped back to their allowed sets, and
/// `silentStartup` cannot stay enabled while `launchOnStartup` is off.
pub fn normalize(prefs: AppPreferences) -> AppPreferences {
    let defaults = AppPreferences::default();

    let language = if matches_choice(&prefs.language, &[LANGUAGE_ZH, LANGUAGE_EN]) {
        prefs.language
    } else {
        defaults.language
    };

    let skill_storage_location = if matches_choice(
        &prefs.skill_storage_location,
        &[SKILL_STORAGE_CCP, SKILL_STORAGE_UNIFIED],
    ) {
        prefs.skill_storage_location
    } else {
        defaults.skill_storage_location
    };

    let skill_sync_method = if matches_choice(
        &prefs.skill_sync_method,
        &[SKILL_SYNC_SYMLINK, SKILL_SYNC_COPY],
    ) {
        prefs.skill_sync_method
    } else {
        defaults.skill_sync_method
    };

    let preferred_terminal = if matches_choice(
        &prefs.preferred_terminal,
        &[TERMINAL_CMD, TERMINAL_POWERSHELL, TERMINAL_WT],
    ) {
        prefs.preferred_terminal
    } else {
        defaults.preferred_terminal
    };

    let log_level = if matches_choice(&prefs.log_level, &LOG_LEVELS) {
        prefs.log_level
    } else {
        defaults.log_level
    };

    let mut visible_apps: Vec<String> = Vec::new();
    for id in prefs.visible_apps {
        if is_agent_id(&id) && !visible_apps.contains(&id) {
            visible_apps.push(id);
        }
    }
    if visible_apps.is_empty() {
        visible_apps = default_visible_apps();
    }

    let config_dirs: BTreeMap<String, String> = prefs
        .config_dirs
        .into_iter()
        .filter_map(|(key, value)| {
            let value = value.trim().to_string();
            if value.is_empty() || !is_agent_id(&key) {
                None
            } else {
                Some((key, value))
            }
        })
        .collect();

    let backup_interval_hours = if BACKUP_INTERVAL_HOURS.contains(&prefs.backup_interval_hours) {
        prefs.backup_interval_hours
    } else {
        defaults.backup_interval_hours
    };

    let backup_retain_count = if BACKUP_RETAIN_COUNTS.contains(&prefs.backup_retain_count) {
        prefs.backup_retain_count
    } else {
        defaults.backup_retain_count
    };

    let silent_startup = prefs.silent_startup && prefs.launch_on_startup;

    AppPreferences {
        language,
        visible_apps,
        show_project_switcher: prefs.show_project_switcher,
        skill_storage_location,
        skill_sync_method,
        preserve_codex_official_auth_on_switch: prefs.preserve_codex_official_auth_on_switch,
        unify_codex_session_history: prefs.unify_codex_session_history,
        launch_on_startup: prefs.launch_on_startup,
        silent_startup,
        apply_to_claude_code_plugin: prefs.apply_to_claude_code_plugin,
        skip_claude_onboarding: prefs.skip_claude_onboarding,
        minimize_to_tray_on_close: prefs.minimize_to_tray_on_close,
        preferred_terminal,
        config_dirs,
        backup_interval_hours,
        backup_retain_count,
        log_enabled: prefs.log_enabled,
        log_level,
    }
}

/// Where each agent keeps its configuration, including user overrides.
///
/// Keys are `"app"` (the CCP state directory) plus every agent id except
/// `"claude-desktop"`, which has no single directory to point at.
pub fn resolved_dirs(prefs: &AppPreferences) -> BTreeMap<String, String> {
    let mut dirs = BTreeMap::new();
    dirs.insert(
        "app".to_string(),
        crate::paths::default_app_state_dir()
            .to_string_lossy()
            .to_string(),
    );

    let home = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
    for id in AGENT_IDS {
        if id == "claude-desktop" {
            continue;
        }
        let resolved = prefs
            .config_dirs
            .get(id)
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
            .or_else(|| default_agent_dir(id, home.as_deref()));
        if let Some(resolved) = resolved {
            dirs.insert(id.to_string(), resolved);
        }
    }

    dirs
}

fn default_agent_dir(id: &str, home: Option<&Path>) -> Option<String> {
    let relative = match id {
        "claude" => ".claude",
        "codex" => ".codex",
        "gemini" => ".gemini",
        "grok" => ".grok",
        "opencode" => ".config/opencode",
        "openclaw" => ".openclaw",
        "hermes" => ".hermes",
        "pi" => ".pi/agent",
        "mcode" => ".mcode",
        "workbuddy" => ".workbuddy",
        "cursor" => ".cursor",
        _ => return None,
    };
    let home = home?;
    Some(home.join(relative).to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let unique = format!(
            "ccp-app-preferences-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or_default()
        );
        let dir = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&dir).expect("创建测试临时目录");
        dir
    }

    #[test]
    fn app_preferences_default_matches_contract() {
        let prefs = AppPreferences::default();

        assert_eq!(prefs.language, "zh");
        assert_eq!(prefs.visible_apps.len(), 12);
        assert_eq!(
            prefs.visible_apps.first().map(String::as_str),
            Some("claude")
        );
        assert_eq!(prefs.visible_apps, default_visible_apps());
        assert!(prefs.show_project_switcher);
        assert_eq!(prefs.skill_storage_location, "ccp");
        assert_eq!(prefs.skill_sync_method, "symlink");
        assert!(prefs.minimize_to_tray_on_close);
        assert_eq!(prefs.preferred_terminal, "cmd");
        assert!(prefs.config_dirs.is_empty());
        assert_eq!(prefs.backup_interval_hours, 24);
        assert_eq!(prefs.backup_retain_count, 10);
        assert!(prefs.log_enabled);
        assert_eq!(prefs.log_level, "info");
        assert!(!prefs.launch_on_startup);
        assert!(!prefs.silent_startup);
        assert!(!prefs.preserve_codex_official_auth_on_switch);
        assert!(!prefs.unify_codex_session_history);
        assert!(!prefs.apply_to_claude_code_plugin);
        assert!(!prefs.skip_claude_onboarding);
    }

    #[test]
    fn app_preferences_normalize_rejects_invalid_values() {
        let mut prefs = AppPreferences::default();
        prefs.language = "fr".to_string();
        prefs.preferred_terminal = "bash".to_string();
        prefs.skill_storage_location = "somewhere".to_string();
        prefs.skill_sync_method = "hardlink".to_string();
        prefs.log_level = "verbose".to_string();
        prefs.backup_interval_hours = 13;
        prefs.backup_retain_count = 4;
        prefs.visible_apps = vec![
            "codex".to_string(),
            "unknown-agent".to_string(),
            "codex".to_string(),
            "claude".to_string(),
        ];
        prefs.config_dirs = BTreeMap::from([
            ("claude".to_string(), "  D:/claude  ".to_string()),
            ("not-an-agent".to_string(), "D:/nope".to_string()),
            ("codex".to_string(), "   ".to_string()),
        ]);
        prefs.launch_on_startup = false;
        prefs.silent_startup = true;

        let normalized = normalize(prefs);
        let defaults = AppPreferences::default();

        assert_eq!(normalized.language, "zh");
        assert_eq!(normalized.preferred_terminal, "cmd");
        assert_eq!(normalized.skill_storage_location, "ccp");
        assert_eq!(normalized.skill_sync_method, "symlink");
        assert_eq!(normalized.log_level, "info");
        assert_eq!(normalized.backup_interval_hours, 24);
        assert_eq!(normalized.backup_retain_count, 10);
        assert_eq!(
            normalized.visible_apps,
            vec!["codex".to_string(), "claude".to_string()]
        );
        assert_eq!(
            normalized.config_dirs,
            BTreeMap::from([("claude".to_string(), "D:/claude".to_string())])
        );
        assert!(!normalized.silent_startup);
        assert_eq!(normalized.language, defaults.language);

        // An empty valid list falls back to showing every agent again.
        let mut empty_visible = AppPreferences::default();
        empty_visible.visible_apps = vec!["nope".to_string()];
        assert_eq!(
            normalize(empty_visible).visible_apps,
            default_visible_apps()
        );

        // silentStartup survives when launchOnStartup is enabled.
        let mut silent_on = AppPreferences::default();
        silent_on.launch_on_startup = true;
        silent_on.silent_startup = true;
        assert!(normalize(silent_on).silent_startup);
    }

    #[test]
    fn app_preferences_store_roundtrip_and_corrupt_backup() {
        let dir = temp_dir("roundtrip");
        let path = dir.join(PREFERENCES_FILE);
        let store = PreferencesStore::new(path.clone());

        // A missing file yields the contract defaults.
        assert_eq!(store.load(), AppPreferences::default());

        let mut prefs = AppPreferences::default();
        prefs.language = "en".to_string();
        prefs.visible_apps = vec!["codex".to_string(), "gemini".to_string()];
        prefs.config_dirs = BTreeMap::from([("codex".to_string(), "D:/codex-home".to_string())]);
        prefs.launch_on_startup = true;
        prefs.silent_startup = true;
        prefs.backup_interval_hours = 48;
        prefs.backup_retain_count = 5;
        prefs.log_level = "debug".to_string();

        let saved = store.save(prefs.clone()).expect("保存偏好设置");
        assert_eq!(saved, normalize(prefs.clone()));
        assert_eq!(store.load(), saved);

        let encoded = std::fs::read_to_string(&path).expect("读取偏好设置文件");
        assert!(encoded.contains("\"visibleApps\""));
        assert!(encoded.contains("\"configDirs\""));
        assert!(encoded.contains("\"minimizeToTrayOnClose\""));
        assert!(!encoded.contains("\"visible_apps\""));

        // A corrupt file is preserved verbatim, and load falls back to defaults.
        let corrupt = "{ not json at all";
        std::fs::write(&path, corrupt).expect("写入损坏的偏好设置文件");
        assert_eq!(store.load(), AppPreferences::default());

        let backups: Vec<PathBuf> = std::fs::read_dir(&dir)
            .expect("列出测试目录")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|entry| {
                entry
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("app-preferences.json.corrupt-"))
            })
            .collect();
        assert_eq!(backups.len(), 1, "应恰好生成一个损坏备份");
        assert_eq!(
            std::fs::read_to_string(&backups[0]).expect("读取损坏备份"),
            corrupt
        );

        drop(store);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
