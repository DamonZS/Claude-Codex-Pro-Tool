use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const APP_STATE_DIR: &str = ".claude-codex-pro";
const LEGACY_APP_STATE_DIR: &str = ".codex-session-delete";
const SETTINGS_FILE: &str = "settings.json";
const LATEST_STATUS_FILE: &str = "latest-status.json";
const DIAGNOSTIC_LOG_FILE: &str = "claude-codex-pro.log";

pub fn default_app_state_dir() -> PathBuf {
    let state_dir = if let Some(home_dir) =
        directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf())
    {
        home_dir.join(APP_STATE_DIR)
    } else {
        PathBuf::from(APP_STATE_DIR)
    };
    migrate_legacy_app_state_dir(&state_dir);
    state_dir
}

pub fn legacy_app_state_dir() -> PathBuf {
    if let Some(home_dir) = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()) {
        return home_dir.join(LEGACY_APP_STATE_DIR);
    }

    PathBuf::from(LEGACY_APP_STATE_DIR)
}

fn migrate_legacy_app_state_dir(state_dir: &PathBuf) {
    let legacy = legacy_app_state_dir();
    if state_dir.exists() || !legacy.exists() || legacy == *state_dir {
        return;
    }
    if std::fs::rename(&legacy, state_dir).is_ok() {
        return;
    }
    let _ = std::fs::create_dir_all(state_dir);
    for file in [SETTINGS_FILE, LATEST_STATUS_FILE, DIAGNOSTIC_LOG_FILE] {
        let from = legacy.join(file);
        let to = state_dir.join(file);
        if from.exists() && !to.exists() {
            let _ = std::fs::copy(from, to);
        }
    }
}

pub fn default_settings_path() -> PathBuf {
    if let Some(path) = settings_path_for_tests() {
        return path;
    }
    default_app_state_dir().join(SETTINGS_FILE)
}

pub fn default_latest_status_path() -> PathBuf {
    default_app_state_dir().join(LATEST_STATUS_FILE)
}

pub fn default_diagnostic_log_path() -> PathBuf {
    default_app_state_dir().join(DIAGNOSTIC_LOG_FILE)
}

fn settings_path_for_tests() -> Option<PathBuf> {
    SETTINGS_PATH_FOR_TESTS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|path| path.clone())
}

static SETTINGS_PATH_FOR_TESTS: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

pub fn set_settings_path_for_tests(path: Option<PathBuf>) -> Option<PathBuf> {
    SETTINGS_PATH_FOR_TESTS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|mut current| std::mem::replace(&mut *current, path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_path_uses_app_state_directory() {
        let path = default_settings_path();

        assert!(path.ends_with(".claude-codex-pro/settings.json"));
    }

    #[test]
    fn default_latest_status_path_uses_app_state_directory() {
        let path = default_latest_status_path();

        assert!(path.ends_with(".claude-codex-pro/latest-status.json"));
    }

    #[test]
    fn default_diagnostic_log_path_uses_app_state_directory() {
        let path = default_diagnostic_log_path();

        assert!(path.ends_with(".claude-codex-pro/claude-codex-pro.log"));
    }
}
