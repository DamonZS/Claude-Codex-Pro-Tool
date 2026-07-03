use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::{Value, json};

static TEST_LOG_PATH: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize)]
struct DiagnosticRecord {
    timestamp_ms: u64,
    pid: u32,
    event: String,
    detail: Value,
}

pub fn append_diagnostic_log(event: &str, detail: impl Serialize) -> std::io::Result<()> {
    let path = diagnostic_log_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let detail = serde_json::to_value(detail).unwrap_or_else(|error| {
        json!({
            "serialization_error": error.to_string()
        })
    });
    let record = DiagnosticRecord {
        timestamp_ms: now_ms(),
        pid: std::process::id(),
        event: event.to_string(),
        detail,
    };
    let line = serde_json::to_string(&record).unwrap_or_else(|error| {
        json!({
            "timestamp_ms": now_ms(),
            "pid": std::process::id(),
            "event": "diagnostic_log.serialization_failed",
            "detail": {
                "message": error.to_string()
            }
        })
        .to_string()
    });

    rotate_if_oversized(&path);

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

/// Roll the diagnostic log over once it crosses [`MAX_LOG_BYTES`].
///
/// The log is append-only and written on nearly every helper/launcher event, so
/// without rotation it grows without bound — bloating disk use and making every
/// tail read scan an ever-larger file. We keep exactly one previous generation
/// (`<log>.1`): the active file is renamed over any existing `.1`, then a fresh
/// active file starts empty. Rotation failures are swallowed on purpose — losing
/// the ability to rotate must never block writing the current event.
fn rotate_if_oversized(path: &std::path::Path) {
    const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;

    let Ok(metadata) = std::fs::metadata(path) else {
        // Missing file (first write) or an unreadable path: nothing to rotate.
        return;
    };
    if metadata.len() < MAX_LOG_BYTES {
        return;
    }

    let rotated = path.with_extension("log.1");
    // `rename` atomically replaces any existing `.1`, so we never accumulate more
    // than one historical generation.
    let _ = std::fs::rename(path, &rotated);
}

pub fn diagnostic_log_path() -> PathBuf {
    if let Some(lock) = TEST_LOG_PATH.get() {
        if let Ok(guard) = lock.lock() {
            if let Some(path) = &*guard {
                return path.clone();
            }
        }
    }
    if let Some(path) = std::env::var_os("CLAUDE_CODEX_PRO_DIAGNOSTIC_LOG").map(PathBuf::from) {
        if !path.as_os_str().is_empty() {
            return path;
        }
    }
    crate::paths::default_diagnostic_log_path()
}

#[doc(hidden)]
pub fn set_diagnostic_log_path_for_tests(path: Option<PathBuf>) {
    set_diagnostic_log_path_override(path);
}

pub fn set_diagnostic_log_path_override(path: Option<PathBuf>) {
    let lock = TEST_LOG_PATH.get_or_init(|| Mutex::new(None));
    *lock.lock().expect("test log path lock poisoned") = path;
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_log_path_honors_runtime_override() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("custom.log");
        set_diagnostic_log_path_override(Some(path.clone()));

        assert_eq!(diagnostic_log_path(), path);

        set_diagnostic_log_path_override(None);
    }

    #[test]
    fn oversized_log_rotates_into_single_previous_generation() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("diagnostic.log");
        let rotated = path.with_extension("log.1");

        // A file below the threshold must not rotate.
        std::fs::write(&path, b"small").unwrap();
        rotate_if_oversized(&path);
        assert!(path.exists(), "small log should stay in place");
        assert!(
            !rotated.exists(),
            "small log must not create a .1 generation"
        );

        // Cross the threshold: the active file is moved aside to `.1`, and the
        // caller then starts a fresh empty active file.
        let big = vec![b'x'; (5 * 1024 * 1024) as usize + 16];
        std::fs::write(&path, &big).unwrap();
        rotate_if_oversized(&path);
        assert!(
            !path.exists(),
            "active log should be renamed away on rotation"
        );
        assert_eq!(
            std::fs::metadata(&rotated).unwrap().len(),
            big.len() as u64,
            "previous generation should hold the old contents",
        );

        // A second rotation overwrites the previous generation rather than piling
        // up additional files.
        let second = vec![b'y'; (5 * 1024 * 1024) as usize + 16];
        std::fs::write(&path, &second).unwrap();
        rotate_if_oversized(&path);
        assert_eq!(
            std::fs::metadata(&rotated).unwrap().len(),
            second.len() as u64,
            "only one historical generation is kept",
        );
    }
}
