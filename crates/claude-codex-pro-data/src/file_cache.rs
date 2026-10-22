//! Per-file parse cache for the local scanners.
//!
//! A full scan re-reads multi-GB Codex/Claude/WorkBuddy logs on every refresh.
//! AITracker solves this with a per-file `(path, mtime, size)` watermark. This
//! cache does the same at the parser boundary: a file that has not changed is
//! served from memory/disk instead of being parsed again.
//!
//! The cache is a best-effort accelerator: every failure (missing/corrupt file,
//! stat error, serialize error) degrades to a normal parse.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, UNIX_EPOCH};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Bump whenever the parsers change how they derive an event: a cached payload
/// is keyed by `(path, mtime, size)` only, so a semantics change that keeps
/// `CACHE_VERSION` would keep serving events built by the previous rules.
const CACHE_VERSION: u32 = 2;
const CACHE_FILE: &str = "file-parse-cache-v1.json";
/// Minimum time between two persistence passes of the parse cache.
const SAVE_INTERVAL: Duration = Duration::from_secs(300);

#[derive(Debug, Default)]
struct CacheState {
    entries: HashMap<String, Entry>,
    dirty: bool,
    last_saved: Option<Instant>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Entry {
    size: u64,
    mtime_ms: u64,
    payload: Value,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CacheFile {
    version: u32,
    files: HashMap<String, Entry>,
}

static CACHE: OnceLock<Mutex<CacheState>> = OnceLock::new();
#[cfg(test)]
static CACHE_PATH_OVERRIDE: Mutex<Option<PathBuf>> = Mutex::new(None);

fn cache_path() -> PathBuf {
    #[cfg(test)]
    if let Some(path) = CACHE_PATH_OVERRIDE
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
    {
        return path;
    }
    claude_codex_pro_core::paths::default_app_state_dir()
        .join("aitracker")
        .join("cache")
        .join(CACHE_FILE)
}

fn state() -> &'static Mutex<CacheState> {
    CACHE.get_or_init(|| {
        let entries = fs::read(cache_path())
            .ok()
            .and_then(|bytes| serde_json::from_slice::<CacheFile>(&bytes).ok())
            .filter(|file| file.version == CACHE_VERSION)
            .map(|file| file.files)
            .unwrap_or_default();
        Mutex::new(CacheState {
            entries,
            dirty: false,
            last_saved: None,
        })
    })
}

fn lock_state() -> std::sync::MutexGuard<'static, CacheState> {
    match state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Process-wide TTL cache of scanned path lists.
///
/// Candidate discovery (directory walks, SQLite reads backed by slow disks) is
/// expensive compared to serving the list from memory. Keys must fully identify
/// the scan inputs so a different source never reuses another list.
static PATH_LISTS: OnceLock<Mutex<HashMap<String, (Instant, Vec<PathBuf>)>>> = OnceLock::new();

fn lock_path_lists() -> std::sync::MutexGuard<'static, HashMap<String, (Instant, Vec<PathBuf>)>> {
    match PATH_LISTS.get_or_init(|| Mutex::new(HashMap::new())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Return the path list cached under `key` while it is younger than `ttl`,
/// otherwise run `build` once and remember its result.
///
/// `build` deliberately runs without holding the list lock: a slow directory
/// scan must not block unrelated scanners that only need an already-cached
/// list. Two concurrent misses may therefore build twice; the last writer
/// wins, which is harmless because candidate lists are best-effort snapshots.
pub fn cached_path_list(
    key: &str,
    ttl: Duration,
    build: impl FnOnce() -> Vec<PathBuf>,
) -> Vec<PathBuf> {
    let hit = {
        let lists = lock_path_lists();
        lists
            .get(key)
            .filter(|(cached_at, _)| cached_at.elapsed() < ttl)
            .map(|(_, paths)| paths.clone())
    };
    if let Some(paths) = hit {
        return paths;
    }
    let paths = build();
    lock_path_lists().insert(key.to_string(), (Instant::now(), paths.clone()));
    paths
}

fn key(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

/// `(size, mtime_ms)` watermark for a file. Missing metadata means no cache.
fn stamp(path: &Path) -> Option<(u64, u64)> {
    let meta = fs::metadata(path).ok()?;
    let mtime_ms = meta
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0);
    Some((meta.len(), mtime_ms))
}

/// Cached parse output for `path`, or `None` when the file is new/changed.
pub fn cached<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let (size, mtime_ms) = stamp(path)?;
    let state = lock_state();
    let entry = state.entries.get(&key(path))?;
    if entry.size != size || entry.mtime_ms != mtime_ms {
        return None;
    }
    serde_json::from_value(entry.payload.clone()).ok()
}

/// Remember the parse output for `path` so the next scan can reuse it.
pub fn store<T: Serialize>(path: &Path, value: &T) {
    let Some((size, mtime_ms)) = stamp(path) else {
        return;
    };
    let Ok(payload) = serde_json::to_value(value) else {
        return;
    };
    let mut state = lock_state();
    state.entries.insert(
        key(path),
        Entry {
            size,
            mtime_ms,
            payload,
        },
    );
    state.dirty = true;
}

/// Persist the cache when it is dirty, at most once per [`SAVE_INTERVAL`].
///
/// A throttled call keeps `dirty` set so the next eligible call (or
/// [`save_forced`]) still writes the accumulated parse results.
pub fn save() {
    {
        let state = lock_state();
        if !state.dirty {
            return;
        }
        if state
            .last_saved
            .is_some_and(|last_saved| last_saved.elapsed() < SAVE_INTERVAL)
        {
            return;
        }
    }
    save_forced();
}

/// Persist the cache immediately, ignoring the interval throttle.
///
/// Tests and shutdown paths can call this to flush pending parse results.
pub fn save_forced() {
    let bytes = {
        let mut state = lock_state();
        if !state.dirty {
            return;
        }
        state.dirty = false;
        state.last_saved = Some(Instant::now());
        let file = CacheFile {
            version: CACHE_VERSION,
            files: state.entries.clone(),
        };
        let Ok(bytes) = serde_json::to_vec(&file) else {
            return;
        };
        bytes
    };
    let path = cache_path();
    let Some(directory) = path.parent() else {
        return;
    };
    if fs::create_dir_all(directory).is_err() {
        return;
    }
    let temporary = path.with_extension("json.tmp");
    if fs::write(&temporary, bytes).is_err() {
        return;
    }
    let _ = fs::rename(&temporary, &path);
}

#[cfg(test)]
pub(crate) fn reset_for_tests(path: PathBuf) {
    if let Ok(mut slot) = CACHE_PATH_OVERRIDE.lock() {
        *slot = Some(path);
    }
    let entries = fs::read(cache_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice::<CacheFile>(&bytes).ok())
        .filter(|file| file.version == CACHE_VERSION)
        .map(|file| file.files)
        .unwrap_or_default();
    let mut state = lock_state();
    state.entries = entries;
    state.dirty = false;
    state.last_saved = None;
}

#[cfg(test)]
pub(crate) fn reset_path_lists_for_tests() {
    lock_path_lists().clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn serial_test(f: impl FnOnce()) {
        static LOCK: Mutex<()> = Mutex::new(());
        let _guard = match LOCK.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        f();
    }

    #[test]
    fn cache_roundtrips_and_invalidates_on_change() {
        serial_test(|| {
            let directory = tempfile::tempdir().unwrap();
            let cache_file = directory.path().join("cache.json");
            reset_for_tests(cache_file.clone());
            let watched = directory.path().join("session.jsonl");
            fs::write(&watched, b"{\"a\":1}\n").unwrap();

            assert!(cached::<Vec<String>>(&watched).is_none());
            store(&watched, &vec!["value".to_string()]);
            assert_eq!(
                cached::<Vec<String>>(&watched),
                Some(vec!["value".to_string()])
            );
            save();
            assert!(cache_file.is_file());

            // Reload from disk: the entry survives a restart.
            reset_for_tests(cache_file.clone());
            assert_eq!(
                cached::<Vec<String>>(&watched),
                Some(vec!["value".to_string()])
            );

            // Appending changes the watermark, so the entry is stale.
            let mut handle = fs::OpenOptions::new().append(true).open(&watched).unwrap();
            handle.write_all(b"{\"b\":2}\n").unwrap();
            assert!(cached::<Vec<String>>(&watched).is_none());
        });
    }

    #[test]
    fn corrupt_cache_file_degrades_to_empty() {
        serial_test(|| {
            let directory = tempfile::tempdir().unwrap();
            let cache_file = directory.path().join("cache.json");
            fs::write(&cache_file, b"{not json").unwrap();
            reset_for_tests(cache_file);
            let watched = directory.path().join("session.jsonl");
            fs::write(&watched, b"x").unwrap();
            assert!(cached::<Vec<String>>(&watched).is_none());
        });
    }

    #[test]
    fn path_list_cache_hits_within_ttl_and_rebuilds_after_expiry() {
        reset_path_lists_for_tests();
        let key = "file-cache-test:path-list";
        let ttl = Duration::from_millis(500);
        let builds = std::cell::Cell::new(0u32);
        let build = |value: &str| {
            builds.set(builds.get() + 1);
            vec![PathBuf::from(value)]
        };

        assert_eq!(
            cached_path_list(key, ttl, || build("first")),
            vec![PathBuf::from("first")]
        );
        // The second call is inside the TTL, so its builder must not run.
        assert_eq!(
            cached_path_list(key, ttl, || build("second")),
            vec![PathBuf::from("first")]
        );
        assert_eq!(builds.get(), 1);

        std::thread::sleep(ttl + Duration::from_millis(100));
        assert_eq!(
            cached_path_list(key, ttl, || build("third")),
            vec![PathBuf::from("third")]
        );
        assert_eq!(builds.get(), 2);
        reset_path_lists_for_tests();
    }

    #[test]
    fn save_is_throttled_until_forced() {
        serial_test(|| {
            let directory = tempfile::tempdir().unwrap();
            let cache_file = directory.path().join("cache.json");
            reset_for_tests(cache_file.clone());
            let watched = directory.path().join("session.jsonl");
            fs::write(&watched, b"{\"a\":1}\n").unwrap();

            store(&watched, &vec!["first".to_string()]);
            save_forced();
            assert!(cache_file.is_file());
            let first_bytes = fs::read(&cache_file).unwrap();
            let first_modified = fs::metadata(&cache_file).unwrap().modified().unwrap();

            store(&watched, &vec!["second".to_string()]);
            save();
            // Within SAVE_INTERVAL the file is untouched and dirty stays set.
            assert_eq!(fs::read(&cache_file).unwrap(), first_bytes);
            assert_eq!(
                fs::metadata(&cache_file).unwrap().modified().unwrap(),
                first_modified
            );

            save_forced();
            assert_ne!(fs::read(&cache_file).unwrap(), first_bytes);
        });
    }
}
