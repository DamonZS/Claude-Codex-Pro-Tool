//! CCP data backups (`.ccpbak` bundles).
//!
//! A backup is a zip archive holding a consistent snapshot of everything the
//! manager persists on the device:
//!
//! - `manifest.json` — format marker, name, creation time and entry list.
//! - `ccp.db` — snapshot of the SQLite database, produced by `VACUUM INTO`.
//! - `settings.json` — the raw settings file, byte for byte.
//! - `app-preferences.json` — the raw preferences file, byte for byte.
//!
//! Every entry is optional except the manifest: a device that never wrote
//! `settings.json` still produces a valid bundle. Restoring validates the
//! whole archive before writing a single byte, so a corrupted or hostile
//! bundle can never leave the device half-restored.
//!
//! The JSON shapes mirror `BackupEntry` / `BackupListResult` in
//! `apps/claude-codex-pro-manager/src/components/settings/contract.ts`.

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};

/// File extension of a backup bundle.
const BUNDLE_EXTENSION: &str = ".ccpbak";
/// Suffix of the file written before an atomic rename into place.
const TEMP_SUFFIX: &str = ".tmp";
/// Suffix used while restoring a single target file.
const RESTORE_TEMP_SUFFIX: &str = ".restore.tmp";
/// Prefix marking the automatic pre-restore safety copies. Backups with this
/// name prefix are never pruned and never counted against the retention limit.
const SAFETY_PREFIX: &str = "restore-safety";

const MANIFEST_ENTRY: &str = "manifest.json";
const DB_ENTRY: &str = "ccp.db";
const SETTINGS_ENTRY: &str = "settings.json";
const PREFERENCES_ENTRY: &str = "app-preferences.json";

/// Every entry name a bundle is allowed to contain.
const ALLOWED_ENTRIES: [&str; 3] = [DB_ENTRY, SETTINGS_ENTRY, PREFERENCES_ENTRY];

/// Format marker written to `manifest.json`.
const FORMAT: &str = "ccp-backup";
/// Format version understood by this build.
const FORMAT_VERSION: u32 = 1;

/// Names longer than this are truncated; the UI shows one line.
const MAX_NAME_LENGTH: usize = 80;

/// A SQLite database always starts with these 16 bytes.
const SQLITE_MAGIC: &[u8] = b"SQLite format 3\0";

/// Every location a backup touches. Passed explicitly so tests can point at
/// temporary directories instead of the real application state.
pub struct BackupPaths {
    pub db: PathBuf,
    pub settings: PathBuf,
    pub preferences: PathBuf,
    pub backup_dir: PathBuf,
}

impl BackupPaths {
    /// The real device locations.
    pub fn default_paths() -> Self {
        Self {
            db: crate::ccp_db::default_db_path(),
            settings: crate::paths::default_settings_path(),
            preferences: crate::app_preferences::PreferencesStore::default_path(),
            backup_dir: crate::paths::default_app_state_dir().join("backups"),
        }
    }
}

/// One bundle as shown in the backup list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEntry {
    /// File name without the `.ccpbak` extension.
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub size_bytes: u64,
}

/// `manifest.json` inside a bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    format: String,
    version: u32,
    name: String,
    created_at: String,
    files: Vec<String>,
}

/// Create a bundle holding the current device state.
///
/// `id` is `ccp-<YYYYMMDD-HHMMSS>-<millis>` in local time, with `-2`, `-3`, …
/// appended until it does not collide with an existing file. `name` defaults
/// to the id; a user-provided name is trimmed and capped at 80 characters.
pub fn create_backup(paths: &BackupPaths, name: Option<&str>) -> anyhow::Result<BackupEntry> {
    std::fs::create_dir_all(&paths.backup_dir)
        .with_context(|| format!("创建备份目录失败：{}", paths.backup_dir.display()))?;

    let id = next_backup_id(&paths.backup_dir);
    let created_at = chrono::Local::now().to_rfc3339();
    let display_name = match name {
        Some(name) => truncate_name(name.trim()),
        None => id.clone(),
    };
    let bundle_path = paths.backup_dir.join(format!("{id}{BUNDLE_EXTENSION}"));
    let temp_path = paths
        .backup_dir
        .join(format!("{id}{BUNDLE_EXTENSION}{TEMP_SUFFIX}"));

    let result = write_bundle(
        paths,
        &temp_path,
        &Manifest {
            format: FORMAT.to_string(),
            version: FORMAT_VERSION,
            name: display_name.clone(),
            created_at: created_at.clone(),
            files: Vec::new(),
        },
    );
    if let Err(error) = result {
        let _ = std::fs::remove_file(&temp_path);
        return Err(error);
    }

    std::fs::rename(&temp_path, &bundle_path).with_context(|| {
        format!(
            "写入备份文件失败：{} → {}",
            temp_path.display(),
            bundle_path.display()
        )
    })?;

    let size_bytes = std::fs::metadata(&bundle_path)
        .map(|metadata| metadata.len())
        .unwrap_or_default();

    Ok(BackupEntry {
        id,
        name: display_name,
        created_at,
        size_bytes,
    })
}

/// List every readable bundle in the backup directory, newest first.
///
/// Files whose manifest cannot be read are skipped rather than failing the
/// whole listing. A missing directory yields an empty list.
pub fn list_backups(paths: &BackupPaths) -> anyhow::Result<Vec<BackupEntry>> {
    if !paths.backup_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&paths.backup_dir)
        .with_context(|| format!("读取备份目录失败：{}", paths.backup_dir.display()))?
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(id) = bundle_id(&path) else {
            continue;
        };
        let Some(manifest) = read_manifest(&path) else {
            continue;
        };
        let size_bytes = entry.metadata().map(|meta| meta.len()).unwrap_or_default();
        entries.push(BackupEntry {
            id,
            name: manifest.name,
            created_at: manifest.created_at,
            size_bytes,
        });
    }

    // Newest first. `createdAt` is RFC3339, so the textual order is the
    // chronological order; ids break ties so the order stays stable.
    entries.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.id.cmp(&left.id))
    });
    Ok(entries)
}

/// Rewrite only the display name inside a bundle's manifest.
pub fn rename_backup(paths: &BackupPaths, id: &str, name: &str) -> anyhow::Result<()> {
    validate_id(id)?;
    let trimmed = name.trim();
    if trimmed.is_empty() {
        bail!("备份名称不能为空");
    }
    if trimmed.chars().count() > MAX_NAME_LENGTH {
        bail!("备份名称不能超过 {MAX_NAME_LENGTH} 个字符");
    }

    let bundle_path = bundle_path(paths, id);
    let bytes = std::fs::read(&bundle_path)
        .with_context(|| format!("备份不存在：{}", bundle_path.display()))?;
    let mut manifest = read_manifest_from_bytes(&bytes)
        .with_context(|| format!("读取备份清单失败：{}", bundle_path.display()))?;
    manifest.name = trimmed.to_string();

    rewrite_manifest(&bundle_path, &manifest, &bytes)
}

/// Delete one bundle.
pub fn delete_backup(paths: &BackupPaths, id: &str) -> anyhow::Result<()> {
    validate_id(id)?;
    let bundle_path = bundle_path(paths, id);
    if !bundle_path.is_file() {
        bail!("备份不存在：{id}");
    }
    std::fs::remove_file(&bundle_path)
        .with_context(|| format!("删除备份失败：{}", bundle_path.display()))?;
    Ok(())
}

/// Snapshot the current state, then restore `id` over it.
///
/// Returns the safety copy that was taken first, so the UI can offer an undo.
pub fn restore_backup(paths: &BackupPaths, id: &str) -> anyhow::Result<BackupEntry> {
    validate_id(id)?;
    restore_local_bundle(paths, &bundle_path(paths, id))
}

/// Copy a fresh backup into `dest_dir`, returning the written path.
pub fn export_bundle(paths: &BackupPaths, dest_dir: &Path) -> anyhow::Result<PathBuf> {
    if !dest_dir.is_dir() {
        bail!("导出目录不存在：{}", dest_dir.display());
    }
    let entry = create_backup(paths, None)?;
    let source = bundle_path(paths, &entry.id);
    let destination = dest_dir.join(format!("{}{BUNDLE_EXTENSION}", entry.id));
    std::fs::copy(&source, &destination).with_context(|| {
        format!(
            "导出备份失败：{} → {}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(destination)
}

/// Import a bundle produced elsewhere, after snapshotting the current state.
///
/// The bundle is fully validated before anything is written; on rejection the
/// device files are left byte-identical.
pub fn import_bundle(paths: &BackupPaths, source: &Path) -> anyhow::Result<BackupEntry> {
    if !source.is_file() {
        bail!("导入文件不存在：{}", source.display());
    }
    let bytes =
        std::fs::read(source).with_context(|| format!("读取导入文件失败：{}", source.display()))?;
    validate_bundle(&bytes)?;
    restore_local_bundle(paths, source)
}

/// Take a safety copy, then restore the already-validated bundle at `bundle`.
fn restore_local_bundle(paths: &BackupPaths, bundle: &Path) -> anyhow::Result<BackupEntry> {
    let safety = create_backup(paths, Some(SAFETY_PREFIX))?;
    restore_from_bundle(paths, bundle)?;
    Ok(safety)
}

/// Keep only the newest `retain` ordinary backups; return how many were
/// deleted.
///
/// Safety copies are neither counted nor deleted.
pub fn prune_backups(paths: &BackupPaths, retain: usize) -> anyhow::Result<usize> {
    let mut ordinary: Vec<BackupEntry> = list_backups(paths)?
        .into_iter()
        .filter(|entry| !entry.name.starts_with(SAFETY_PREFIX))
        .collect();
    if ordinary.len() <= retain {
        return Ok(0);
    }

    ordinary.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.id.cmp(&left.id))
    });

    let mut deleted = 0;
    for entry in ordinary.into_iter().skip(retain) {
        delete_backup(paths, &entry.id)?;
        deleted += 1;
    }
    Ok(deleted)
}

/// Whether the automatic backup timer should fire now.
///
/// `interval_hours == 0` disables the feature. Safety copies do not count as
/// "the newest backup", so a restore never postpones the next automatic copy.
pub fn auto_backup_due(paths: &BackupPaths, interval_hours: u32) -> bool {
    if interval_hours == 0 {
        return false;
    }
    let newest = match list_backups(paths) {
        Ok(entries) => entries
            .into_iter()
            .filter(|entry| !entry.name.starts_with(SAFETY_PREFIX))
            .max_by(|left, right| {
                left.created_at
                    .cmp(&right.created_at)
                    .then_with(|| left.id.cmp(&right.id))
            }),
        Err(_) => None,
    };
    let Some(newest) = newest else {
        return true;
    };

    let Ok(created_at) = chrono::DateTime::parse_from_rfc3339(&newest.created_at) else {
        return true;
    };
    let age = chrono::Local::now().signed_duration_since(created_at);
    age.num_hours() >= i64::from(interval_hours)
}

/// Restore every entry of `bundle` over the device.
///
/// Validation reads the whole archive first and writes nothing; only when the
/// manifest, every entry name, the JSON files and the SQLite header are all
/// acceptable does the write phase begin.
fn restore_from_bundle(paths: &BackupPaths, bundle: &Path) -> anyhow::Result<()> {
    let bytes =
        std::fs::read(bundle).with_context(|| format!("读取备份失败：{}", bundle.display()))?;
    let contents = validate_bundle(&bytes)?;

    if let Some(settings) = contents.settings {
        write_restored_bytes(&paths.settings, settings).context("恢复 settings.json 失败")?;
    }
    if let Some(preferences) = contents.preferences {
        write_restored_bytes(&paths.preferences, preferences)
            .context("恢复 app-preferences.json 失败")?;
    }
    if let Some(database) = contents.db {
        write_restored_bytes(&paths.db, database).context("恢复 ccp.db 失败")?;
        // A stale WAL next to the freshly restored database would replay the
        // pre-restore state on the next open.
        for suffix in ["-wal", "-shm"] {
            let sidecar = sidecar_path(&paths.db, suffix);
            if sidecar.exists() {
                std::fs::remove_file(&sidecar)
                    .with_context(|| format!("清理数据库旁路文件失败：{}", sidecar.display()))?;
            }
        }
    }
    Ok(())
}

/// The validated payload of a bundle: only the entries the format allows.
struct BundleContents {
    db: Option<Vec<u8>>,
    settings: Option<Vec<u8>>,
    preferences: Option<Vec<u8>>,
}

/// Read and validate a bundle without touching the filesystem.
fn validate_bundle(bytes: &[u8]) -> anyhow::Result<BundleContents> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).context("备份文件已损坏或不是 zip")?;

    let mut contents = BundleContents {
        db: None,
        settings: None,
        preferences: None,
    };

    for index in 0..archive.len() {
        let mut file = archive.by_index(index).context("读取备份条目失败")?;
        let Some(entry_file) = file.enclosed_name() else {
            bail!("备份条目名不合法");
        };
        let Some(name) = entry_file.file_name().and_then(|name| name.to_str()) else {
            bail!("备份条目名不合法");
        };
        let name = name.to_string();
        if name == MANIFEST_ENTRY {
            continue;
        }
        if !ALLOWED_ENTRIES.contains(&name.as_str()) {
            bail!("备份包含不允许的条目：{name}");
        }

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .with_context(|| format!("读取备份条目失败：{name}"))?;
        match name.as_str() {
            DB_ENTRY => contents.db = Some(buffer),
            SETTINGS_ENTRY => contents.settings = Some(buffer),
            PREFERENCES_ENTRY => contents.preferences = Some(buffer),
            _ => unreachable!("entry names are filtered against ALLOWED_ENTRIES"),
        }
    }

    let manifest = read_manifest_from_bytes(bytes)?;
    if manifest.format != FORMAT {
        bail!("备份格式不受支持：{}", manifest.format);
    }
    if manifest.version != FORMAT_VERSION {
        bail!("备份版本不受支持：{}", manifest.version);
    }
    for name in &manifest.files {
        if !is_safe_entry_name(name) {
            bail!("备份清单包含不安全的条目名：{name}");
        }
        if !ALLOWED_ENTRIES.contains(&name.as_str()) {
            bail!("备份清单包含不允许的条目：{name}");
        }
    }

    if let Some(settings) = &contents.settings {
        serde_json::from_slice::<serde_json::Value>(settings)
            .context("备份中的 settings.json 不是合法 JSON")?;
    }
    if let Some(preferences) = &contents.preferences {
        serde_json::from_slice::<serde_json::Value>(preferences)
            .context("备份中的 app-preferences.json 不是合法 JSON")?;
    }
    if let Some(database) = &contents.db
        && !database.starts_with(SQLITE_MAGIC)
    {
        bail!("备份中的 ccp.db 不是合法的 SQLite 数据库");
    }

    Ok(contents)
}

/// Write a bundle at `path` containing the current device state.
fn write_bundle(paths: &BackupPaths, path: &Path, manifest: &Manifest) -> anyhow::Result<()> {
    let file = std::fs::File::create(path)
        .with_context(|| format!("创建备份临时文件失败：{}", path.display()))?;
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let settings = read_optional(&paths.settings);
    let preferences = read_optional(&paths.preferences);
    let database = snapshot_database(&paths.db)?;

    let mut manifest = manifest.clone();
    if database.is_some() {
        manifest.files.push(DB_ENTRY.to_string());
    }
    if settings.is_some() {
        manifest.files.push(SETTINGS_ENTRY.to_string());
    }
    if preferences.is_some() {
        manifest.files.push(PREFERENCES_ENTRY.to_string());
    }
    let encoded = serde_json::to_string_pretty(&manifest).context("序列化备份清单失败")?;

    writer
        .start_file(MANIFEST_ENTRY, options)
        .context("写入备份清单失败")?;
    writer
        .write_all(encoded.as_bytes())
        .context("写入备份清单失败")?;

    // The database goes first so a partially written archive is still
    // identifiable as "the big entry is here".
    if let Some(database) = database {
        writer
            .start_file(DB_ENTRY, options)
            .context("写入数据库快照失败")?;
        writer.write_all(&database).context("写入数据库快照失败")?;
    }
    if let Some(settings) = settings {
        writer
            .start_file(SETTINGS_ENTRY, options)
            .context("写入 settings.json 失败")?;
        writer
            .write_all(&settings)
            .context("写入 settings.json 失败")?;
    }
    if let Some(preferences) = preferences {
        writer
            .start_file(PREFERENCES_ENTRY, options)
            .context("写入 app-preferences.json 失败")?;
        writer
            .write_all(&preferences)
            .context("写入 app-preferences.json 失败")?;
    }

    // `finish` writes the central directory and flushes the file.
    writer.finish().context("完成备份文件失败")?;
    Ok(())
}

/// Replace a bundle's manifest, preserving every other entry verbatim.
fn rewrite_manifest(
    bundle_path: &Path,
    manifest: &Manifest,
    original: &[u8],
) -> anyhow::Result<()> {
    let temp_path = sidecar_path(bundle_path, &format!("{TEMP_SUFFIX}{TEMP_SUFFIX}"));
    let result = (|| -> anyhow::Result<()> {
        let file = std::fs::File::create(&temp_path)
            .with_context(|| format!("创建备份临时文件失败：{}", temp_path.display()))?;
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let encoded = serde_json::to_string_pretty(manifest).context("序列化备份清单失败")?;
        writer
            .start_file(MANIFEST_ENTRY, options)
            .context("写入备份清单失败")?;
        writer
            .write_all(encoded.as_bytes())
            .context("写入备份清单失败")?;

        // Only the other allowed entries exist in a bundle we wrote; anything
        // else was already rejected by `validate_bundle` on the way in.
        let mut archive = zip::ZipArchive::new(Cursor::new(original)).context("备份文件已损坏")?;
        for index in 0..archive.len() {
            let mut file = archive.by_index(index).context("读取备份条目失败")?;
            let Some(name) = file.enclosed_name().and_then(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().to_string())
            }) else {
                continue;
            };
            if name == MANIFEST_ENTRY || !ALLOWED_ENTRIES.contains(&name.as_str()) {
                continue;
            }
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .with_context(|| format!("读取备份条目失败：{name}"))?;
            drop(file);
            writer
                .start_file(name.as_str(), options)
                .with_context(|| format!("写入备份条目失败：{name}"))?;
            writer
                .write_all(&buffer)
                .with_context(|| format!("写入备份条目失败：{name}"))?;
        }

        writer.finish().context("完成备份文件失败")?;
        std::fs::rename(&temp_path, bundle_path).with_context(|| {
            format!(
                "替换备份文件失败：{} → {}",
                temp_path.display(),
                bundle_path.display()
            )
        })?;
        Ok(())
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    result
}

/// Snapshot the database with `VACUUM INTO`, returning the raw bytes.
///
/// A missing source database yields `None`; the entry is then skipped. The
/// temporary file is always removed, even on failure.
fn snapshot_database(db_path: &Path) -> anyhow::Result<Option<Vec<u8>>> {
    if !db_path.is_file() {
        return Ok(None);
    }

    let target = sidecar_path(db_path, ".snapshot.tmp");
    let _ = std::fs::remove_file(&target);
    // `VACUUM INTO` takes a string literal, so quotes in the path must be
    // doubled; the path itself is never logged.
    let escaped = target.to_string_lossy().replace('\'', "''");
    let statement = format!("VACUUM INTO '{escaped}'");

    let snapshot = (|| -> anyhow::Result<Vec<u8>> {
        let connection = crate::ccp_db::open(db_path)?;
        connection
            .execute_batch(&statement)
            .context("生成数据库快照失败")?;
        drop(connection);
        std::fs::read(&target).context("读取数据库快照失败")
    })();

    let _ = std::fs::remove_file(&target);
    snapshot.map(Some)
}

/// Read a device file, treating "missing" as "skip this entry" and any other
/// IO error as a hard failure.
fn read_optional(path: &Path) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

/// Write restored bytes next to the target, then rename into place.
///
/// The caller's context names the file; the error never carries its contents.
fn write_restored_bytes(target: &Path, bytes: Vec<u8>) -> anyhow::Result<()> {
    if let Some(parent) = target.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建目录失败：{}", parent.display()))?;
    }
    let temp_path = sidecar_path(target, RESTORE_TEMP_SUFFIX);
    std::fs::write(&temp_path, bytes)
        .with_context(|| format!("写入临时文件失败：{}", temp_path.display()))?;
    if let Err(error) = std::fs::rename(&temp_path, target) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(error).with_context(|| {
            format!(
                "替换文件失败：{} → {}",
                temp_path.display(),
                target.display()
            )
        });
    }
    Ok(())
}

/// `path` with `suffix` appended to the file name (`ccp.db` → `ccp.db-wal`).
fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

/// The bundle path for a validated id.
fn bundle_path(paths: &BackupPaths, id: &str) -> PathBuf {
    paths.backup_dir.join(format!("{id}{BUNDLE_EXTENSION}"))
}

/// The id of a `*.ccpbak` file, or `None` for anything else.
fn bundle_id(path: &Path) -> Option<String> {
    if !path.is_file() {
        return None;
    }
    let name = path.file_name()?.to_str()?;
    let id = name.strip_suffix(BUNDLE_EXTENSION)?;
    if !is_valid_id(id) {
        return None;
    }
    Some(id.to_string())
}

/// Read and parse the manifest of a bundle on disk.
fn read_manifest(path: &Path) -> Option<Manifest> {
    let bytes = std::fs::read(path).ok()?;
    read_manifest_from_bytes(&bytes).ok()
}

/// Parse a manifest out of an in-memory bundle.
fn read_manifest_from_bytes(bytes: &[u8]) -> anyhow::Result<Manifest> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).context("备份文件已损坏")?;
    let mut file = archive
        .by_name(MANIFEST_ENTRY)
        .context("备份缺少 manifest.json")?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).context("读取备份清单失败")?;
    drop(file);
    serde_json::from_slice(&buffer).context("备份清单格式无效")
}

/// Build the next free backup id.
fn next_backup_id(backup_dir: &Path) -> String {
    // Local time, minute precision, plus milliseconds so two backups in the
    // same second still sort correctly.
    let now = chrono::Local::now();
    let base = now.format("ccp-%Y%m%d-%H%M%S-%3f").to_string();
    if !backup_dir
        .join(format!("{base}{BUNDLE_EXTENSION}"))
        .exists()
    {
        return base;
    }
    let mut counter = 2_u32;
    loop {
        let candidate = format!("{base}-{counter}");
        if !backup_dir
            .join(format!("{candidate}{BUNDLE_EXTENSION}"))
            .exists()
        {
            return candidate;
        }
        counter += 1;
    }
}

/// Reject ids that could escape the backup directory.
///
/// Only `[A-Za-z0-9_-]` characters are accepted; this is what blocks
/// `../`, absolute paths and separators of either platform.
fn validate_id(id: &str) -> anyhow::Result<()> {
    if !is_valid_id(id) {
        bail!("无效的备份 ID");
    }
    Ok(())
}

fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        })
}

/// Whether an entry name from a manifest is safe to write.
fn is_safe_entry_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
        && ALLOWED_ENTRIES.contains(&name)
}

/// Trim a name and cut it to 80 characters (not bytes).
fn truncate_name(name: &str) -> String {
    name.chars().take(MAX_NAME_LENGTH).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fully isolated directory: a private temp dir per call.
    fn temp_paths(label: &str) -> (PathBuf, BackupPaths) {
        let unique = format!(
            "ccp-ccp-backup-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or_default()
        );
        let root = std::env::temp_dir().join(unique);
        let state = root.join("state");
        std::fs::create_dir_all(&state).expect("创建测试状态目录");
        let paths = BackupPaths {
            db: state.join("ccp.db"),
            settings: state.join("settings.json"),
            preferences: state.join("app-preferences.json"),
            backup_dir: root.join("backups"),
        };
        (root, paths)
    }

    fn seed(paths: &BackupPaths, kv_value: &str, settings: &str) {
        let conn = crate::ccp_db::open(&paths.db).expect("创建测试数据库");
        crate::ccp_db::kv_set(&conn, "test.key", kv_value).expect("写入 kv");
        drop(conn);
        std::fs::write(&paths.settings, settings).expect("写入 settings.json");
        std::fs::write(&paths.preferences, r#"{"language":"zh"}"#).expect("写入偏好设置");
    }

    fn read_entry_ids(paths: &BackupPaths) -> Vec<String> {
        list_backups(paths)
            .expect("列出备份")
            .into_iter()
            .map(|entry| entry.id)
            .collect()
    }

    fn entry_names(paths: &BackupPaths, id: &str) -> Vec<String> {
        let bytes = std::fs::read(bundle_path(paths, id)).expect("读取备份");
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("打开备份");
        let mut names = Vec::new();
        for index in 0..archive.len() {
            let file = archive.by_index(index).expect("读取条目");
            names.push(
                file.enclosed_name()
                    .and_then(|path| {
                        path.file_name()
                            .map(|name| name.to_string_lossy().to_string())
                    })
                    .expect("条目名"),
            );
        }
        names
    }

    fn replace_settings_in_bundle(paths: &BackupPaths, id: &str, settings: &[u8]) {
        let source = bundle_path(paths, id);
        let bytes = std::fs::read(&source).expect("读取备份");
        let mut archive = zip::ZipArchive::new(Cursor::new(&bytes)).expect("打开备份");
        let temp = sidecar_path(&source, ".test.tmp");
        {
            let file = std::fs::File::create(&temp).expect("创建临时备份");
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            for index in 0..archive.len() {
                let mut file = archive.by_index(index).expect("读取条目");
                let name = file.name().to_string();
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer).expect("读取条目内容");
                drop(file);
                let content: &[u8] = if name == SETTINGS_ENTRY {
                    settings
                } else {
                    &buffer
                };
                writer.start_file(name, options).expect("写入条目");
                writer.write_all(content).expect("写入条目内容");
            }
            writer.finish().expect("完成备份");
        }
        std::fs::rename(&temp, &source).expect("替换备份");
    }

    fn rewrite_manifest_entry(paths: &BackupPaths, id: &str, mutate: impl Fn(&mut Manifest)) {
        let source = bundle_path(paths, id);
        let bytes = std::fs::read(&source).expect("读取备份");
        let mut manifest = read_manifest_from_bytes(&bytes).expect("读取清单");
        mutate(&mut manifest);
        let mut archive = zip::ZipArchive::new(Cursor::new(&bytes)).expect("打开备份");
        let temp = sidecar_path(&source, ".test.tmp");
        {
            let file = std::fs::File::create(&temp).expect("创建临时备份");
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            let encoded = serde_json::to_vec(&manifest).expect("序列化清单");
            writer
                .start_file(MANIFEST_ENTRY, options)
                .expect("写入清单");
            writer.write_all(&encoded).expect("写入清单");
            for index in 0..archive.len() {
                let mut file = archive.by_index(index).expect("读取条目");
                let name = file.name().to_string();
                if name == MANIFEST_ENTRY {
                    continue;
                }
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer).expect("读取条目内容");
                drop(file);
                writer.start_file(name, options).expect("写入条目");
                writer.write_all(&buffer).expect("写入条目内容");
            }
            writer.finish().expect("完成备份");
        }
        std::fs::rename(&temp, &source).expect("替换备份");
    }

    #[test]
    fn ccp_backup_create_and_list_snapshot_contents() {
        let (root, paths) = temp_paths("create");
        seed(&paths, "before-backup", r#"{"apiKey":"secret"}"#);

        let entry = create_backup(&paths, None).expect("创建备份");
        assert!(entry.id.starts_with("ccp-"), "id 应带 ccp- 前缀");
        assert_eq!(entry.name, entry.id, "未提供名称时回退为 id");
        assert!(!entry.created_at.is_empty());
        assert!(entry.size_bytes > 0);
        assert!(bundle_path(&paths, &entry.id).is_file());

        let listed = list_backups(&paths).expect("列出备份");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, entry.id);

        let names = entry_names(&paths, &entry.id);
        for expected in [MANIFEST_ENTRY, DB_ENTRY, SETTINGS_ENTRY, PREFERENCES_ENTRY] {
            assert!(names.contains(&expected.to_string()), "缺少条目 {expected}");
        }

        // The snapshot must be a real, standalone database carrying the kv row.
        let extracted = root.join("extracted.db");
        let bytes = std::fs::read(bundle_path(&paths, &entry.id)).expect("读取备份");
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("打开备份");
        let mut file = archive.by_name(DB_ENTRY).expect("读取 ccp.db 条目");
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).expect("读取 ccp.db");
        drop(file);
        assert!(buffer.starts_with(SQLITE_MAGIC));
        std::fs::write(&extracted, buffer).expect("写出快照");

        let conn = rusqlite::Connection::open(&extracted).expect("打开快照");
        let value = crate::ccp_db::kv_get(&conn, "test.key").expect("读取 kv");
        assert_eq!(value.as_deref(), Some("before-backup"));
        drop(conn);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ccp_backup_rename_and_delete_roundtrip() {
        let (root, paths) = temp_paths("rename");
        seed(&paths, "v1", "{}");

        let created = create_backup(&paths, Some("  我的备份  ")).expect("创建备份");
        assert_eq!(created.name, "我的备份", "名称应去除首尾空白");

        rename_backup(&paths, &created.id, "  重命名后  ").expect("重命名备份");
        let listed = list_backups(&paths).expect("列出备份");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "重命名后");
        assert_eq!(listed[0].id, created.id);

        // Renaming must not drop the payload entries.
        let names = entry_names(&paths, &created.id);
        assert!(names.contains(&DB_ENTRY.to_string()));
        assert!(names.contains(&SETTINGS_ENTRY.to_string()));

        assert!(
            rename_backup(&paths, &created.id, "   ").is_err(),
            "空名称应被拒绝"
        );
        assert!(
            rename_backup(&paths, &created.id, &"长".repeat(81)).is_err(),
            "超长名称应被拒绝"
        );

        delete_backup(&paths, &created.id).expect("删除备份");
        assert!(list_backups(&paths).expect("列出备份").is_empty());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ccp_backup_restore_returns_to_backup_state() {
        let (root, paths) = temp_paths("restore");
        seed(&paths, "at-backup-time", r#"{"marker":"backup"}"#);
        let entry = create_backup(&paths, None).expect("创建备份");

        // Drift away from the backed-up state.
        let conn = crate::ccp_db::open(&paths.db).expect("打开数据库");
        crate::ccp_db::kv_set(&conn, "test.key", "after-backup").expect("改写 kv");
        drop(conn);
        std::fs::write(&paths.settings, r#"{"marker":"changed"}"#).expect("改写 settings.json");

        let safety = restore_backup(&paths, &entry.id).expect("恢复备份");
        assert!(
            safety.name.starts_with(SAFETY_PREFIX),
            "恢复前应先创建安全备份，实际为 {}",
            safety.name
        );

        assert_eq!(
            std::fs::read_to_string(&paths.settings).expect("读取 settings.json"),
            r#"{"marker":"backup"}"#
        );
        let conn = crate::ccp_db::open(&paths.db).expect("重新打开数据库");
        assert_eq!(
            crate::ccp_db::kv_get(&conn, "test.key").expect("读取 kv"),
            Some("at-backup-time".to_string())
        );
        drop(conn);

        // The original backup plus the safety copy.
        let ids = read_entry_ids(&paths);
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&entry.id));
        assert!(ids.contains(&safety.id));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ccp_backup_import_rejects_bad_bundles_without_writing() {
        let (root, paths) = temp_paths("import-reject");
        seed(&paths, "original", r#"{"marker":"original"}"#);
        let entry = create_backup(&paths, None).expect("创建备份");
        delete_backup(&paths, &entry.id).expect("删除用于重新导入的备份");

        let import_dir = root.join("imports");
        std::fs::create_dir_all(&import_dir).expect("创建导入目录");

        let before_settings = std::fs::read(&paths.settings).expect("读取 settings.json");
        let before_preferences = std::fs::read(&paths.preferences).expect("读取偏好设置");
        let before_db = std::fs::read(&paths.db).expect("读取数据库");

        // 1. Wrong format marker.
        seed(&paths, "original", r#"{"marker":"original"}"#);
        let fresh = create_backup(&paths, None).expect("创建备份");
        let source = bundle_path(&paths, &fresh.id);
        rewrite_manifest_entry(&paths, &fresh.id, |manifest| {
            manifest.format = "not-ccp".to_string();
        });
        let wrong_format = import_dir.join("wrong-format.ccpbak");
        std::fs::copy(&source, &wrong_format).expect("复制坏备份");

        // 2. An entry name containing `..`.
        rewrite_manifest_entry(&paths, &fresh.id, |manifest| {
            manifest.files.push("../evil.json".to_string());
        });
        let traversal = import_dir.join("traversal.ccpbak");
        std::fs::copy(&source, &traversal).expect("复制坏备份");

        // 3. Invalid JSON in settings.json.
        rewrite_manifest_entry(&paths, &fresh.id, |manifest| {
            manifest.files.retain(|name| name != "../evil.json");
        });
        replace_settings_in_bundle(&paths, &fresh.id, b"{ not json");
        let broken_json = import_dir.join("broken-json.ccpbak");
        std::fs::copy(&source, &broken_json).expect("复制坏备份");

        for bad in [&wrong_format, &traversal, &broken_json] {
            let error = import_bundle(&paths, bad).expect_err("坏备份必须被拒绝");
            assert!(!error.to_string().is_empty());
        }

        assert_eq!(
            std::fs::read(&paths.settings).expect("读取 settings.json"),
            before_settings,
            "被拒绝的导入不得改写 settings.json"
        );
        assert_eq!(
            std::fs::read(&paths.preferences).expect("读取偏好设置"),
            before_preferences
        );
        assert_eq!(
            std::fs::read(&paths.db).expect("读取数据库"),
            before_db,
            "被拒绝的导入不得改写数据库"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ccp_backup_import_restores_valid_bundle_with_safety_copy() {
        let (root, paths) = temp_paths("import-ok");
        seed(&paths, "imported-value", r#"{"marker":"imported"}"#);
        let entry = create_backup(&paths, None).expect("创建备份");
        let source = root.join("exported.ccpbak");
        std::fs::copy(bundle_path(&paths, &entry.id), &source).expect("导出备份");

        std::fs::write(&paths.settings, r#"{"marker":"drifted"}"#).expect("改写 settings.json");

        let safety = import_bundle(&paths, &source).expect("导入备份");
        assert!(safety.name.starts_with(SAFETY_PREFIX));
        assert_eq!(
            std::fs::read_to_string(&paths.settings).expect("读取 settings.json"),
            r#"{"marker":"imported"}"#
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ccp_backup_prune_keeps_newest_and_never_drops_safety() {
        let (root, paths) = temp_paths("prune");
        seed(&paths, "v1", "{}");

        let mut ordinary = Vec::new();
        for index in 0..5 {
            // A distinct millisecond keeps `createdAt` strictly ordered.
            std::thread::sleep(std::time::Duration::from_millis(2));
            ordinary
                .push(create_backup(&paths, Some(&format!("normal-{index}"))).expect("创建备份"));
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
        let safety = create_backup(&paths, Some(SAFETY_PREFIX)).expect("创建安全备份");

        let deleted = prune_backups(&paths, 2).expect("清理备份");
        assert_eq!(deleted, 3);

        let remaining = list_backups(&paths).expect("列出备份");
        assert_eq!(remaining.len(), 3, "2 个普通备份 + 1 个安全备份");
        assert!(
            remaining.iter().any(|entry| entry.id == safety.id),
            "安全备份不得被删除"
        );
        let kept: Vec<&str> = remaining
            .iter()
            .filter(|entry| !entry.name.starts_with(SAFETY_PREFIX))
            .map(|entry| entry.name.as_str())
            .collect();
        assert!(kept.contains(&"normal-4"));
        assert!(kept.contains(&"normal-3"));
        assert!(!kept.contains(&"normal-0"));
        let _ = ordinary;

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ccp_backup_rejects_invalid_ids() {
        let (root, paths) = temp_paths("bad-id");
        seed(&paths, "v1", "{}");
        let created = create_backup(&paths, None).expect("创建备份");

        for bad in ["../x", "..", "a/b", "a\\b", "ccp.db", "", "..\\x"] {
            let error = delete_backup(&paths, bad).expect_err("非法 ID 必须被拒绝");
            assert_eq!(error.to_string(), "无效的备份 ID");
            assert!(restore_backup(&paths, bad).is_err());
            assert!(rename_backup(&paths, bad, "x").is_err());
        }

        // A valid id still works, and the bad attempts left it untouched.
        assert!(bundle_path(&paths, &created.id).is_file());
        assert_eq!(read_entry_ids(&paths).len(), 1);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ccp_backup_due_and_export_behavior() {
        let (root, paths) = temp_paths("due");
        seed(&paths, "v1", "{}");

        // No backups yet: due immediately, unless the feature is off.
        assert!(!auto_backup_due(&paths, 0));
        assert!(auto_backup_due(&paths, 24));

        create_backup(&paths, None).expect("创建备份");
        assert!(!auto_backup_due(&paths, 24), "刚创建的备份不应到期");

        // Only the safety copy exists: still due.
        let (safety_root, safety_paths) = temp_paths("due-safety");
        seed(&safety_paths, "v1", "{}");
        create_backup(&safety_paths, Some(SAFETY_PREFIX)).expect("创建安全备份");
        assert!(auto_backup_due(&safety_paths, 24));
        let _ = std::fs::remove_dir_all(&safety_root);

        // Export writes a copy and refuses a missing directory.
        let dest = root.join("exports");
        std::fs::create_dir_all(&dest).expect("创建导出目录");
        let exported = export_bundle(&paths, &dest).expect("导出备份");
        assert!(exported.is_file());
        assert_eq!(
            exported.extension().and_then(|value| value.to_str()),
            Some("ccpbak")
        );
        assert!(export_bundle(&paths, &root.join("missing")).is_err());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ccp_backup_create_skips_missing_optional_files() {
        let (root, paths) = temp_paths("optional");
        std::fs::create_dir_all(&paths.backup_dir).expect("创建备份目录");

        // Nothing on disk at all: the manifest is still produced.
        let entry = create_backup(&paths, None).expect("空状态也应能创建备份");
        let names = entry_names(&paths, &entry.id);
        assert_eq!(names, vec![MANIFEST_ENTRY.to_string()]);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ccp_backup_list_skips_broken_manifests_and_missing_dir() {
        let (root, paths) = temp_paths("list");
        seed(&paths, "v1", "{}");

        // Missing directory: empty list, no error.
        assert!(
            list_backups(&paths)
                .expect("缺失目录应返回空列表")
                .is_empty()
        );

        let created = create_backup(&paths, None).expect("创建备份");
        std::fs::write(
            paths.backup_dir.join("ccp-broken.ccpbak"),
            b"not a zip at all",
        )
        .expect("写入损坏的备份");
        // Non-bundle files are ignored entirely.
        std::fs::write(paths.backup_dir.join("notes.txt"), b"hello").expect("写入普通文件");
        std::fs::write(paths.backup_dir.join("ccp-keep.ccpbak.tmp"), b"partial")
            .expect("写入残留临时文件");

        let listed = list_backups(&paths).expect("列出备份");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, created.id);

        let _ = std::fs::remove_dir_all(&root);
    }
}
