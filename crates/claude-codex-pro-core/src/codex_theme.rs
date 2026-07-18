use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, bail};
use base64::Engine;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const STORE_DIR: &str = "codex-themes";
const STATE_FILE: &str = "state.json";
const LOCK_FILE: &str = "repository.lock";
const DEFAULT_THEME_ID: &str = "default";
const MAX_FILES: usize = 256;
const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 32 * 1024 * 1024;
const MAX_CSS_BYTES: u64 = 1024 * 1024;
const MAX_PREVIEW_BYTES: u64 = 4 * 1024 * 1024;
const MAX_CSS_VARIABLES: usize = 128;
const MAX_ROOT_CLASSES: usize = 16;
const MAX_ROOT_ATTRIBUTES: usize = 32;
const MAX_ASSET_VARIABLES: usize = 64;
const MAX_CSS_VARIABLE_VALUE_BYTES: usize = 1024;
const MAX_CSS_VARIABLE_VALUES_BYTES: usize = 64 * 1024;
const MAX_ROOT_ATTRIBUTE_VALUE_BYTES: usize = 256;
const MAX_RUNTIME_ASSET_DATA_URI_BYTES: usize = 48 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CodexThemeManifest {
    pub format_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    #[serde(default)]
    pub description: String,
    pub preview: String,
    pub entry_style: String,
    #[serde(default)]
    pub assets: Vec<String>,
    #[serde(default)]
    pub css_variables: BTreeMap<String, String>,
    #[serde(default)]
    pub root_attributes: CodexThemeRootAttributes,
    #[serde(default)]
    pub asset_variables: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CodexThemeRootAttributes {
    #[serde(default)]
    pub classes: Vec<String>,
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodexThemeSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub preview_data_uri: Option<String>,
    pub builtin: bool,
    pub current: bool,
    pub imported_at: u64,
    pub updated_at: u64,
    pub integrity_sha256: Option<String>,
    pub previous_version_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodexThemeList {
    pub themes: Vec<CodexThemeSummary>,
    pub current_theme_id: String,
    pub generation: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodexThemePayload {
    pub theme_id: String,
    pub generation: u64,
    pub css: String,
    pub css_variables: BTreeMap<String, String>,
    pub root_attributes: CodexThemeRootAttributes,
    pub asset_data_uris: BTreeMap<String, String>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodexThemeOperationResult {
    pub theme_id: String,
    pub persisted: bool,
    pub runtime_applied: bool,
    pub restart_required: bool,
    pub rolled_back: bool,
    pub generation: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct InstalledTheme {
    manifest: CodexThemeManifest,
    imported_at: u64,
    updated_at: u64,
    integrity_sha256: String,
    previous_version_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ThemeState {
    schema_version: u32,
    current_theme_id: String,
    previous_theme_id: Option<String>,
    generation: u64,
    themes: Vec<InstalledTheme>,
}

impl Default for ThemeState {
    fn default() -> Self {
        Self {
            schema_version: 1,
            current_theme_id: DEFAULT_THEME_ID.to_string(),
            previous_theme_id: None,
            generation: 0,
            themes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct MutationJournal {
    operation_id: String,
    operation_type: String,
    theme_id: String,
    phase: String,
    started_at: u64,
    state_before: ThemeState,
    staging_dir: Option<PathBuf>,
    target_dir: Option<PathBuf>,
    backup_dir: Option<PathBuf>,
    #[serde(default)]
    finished_at: Option<u64>,
    #[serde(default)]
    result: Option<String>,
}

pub struct CodexThemeStore {
    root: PathBuf,
}

impl CodexThemeStore {
    pub fn open_default() -> anyhow::Result<Self> {
        Self::open(crate::paths::default_app_state_dir().join(STORE_DIR))
    }

    pub fn open(root: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let store = Self { root: root.into() };
        store.ensure_layout()?;
        let _lock = store.acquire_lock()?;
        store.recover_pending_locked()?;
        if !store.state_path().exists() {
            store.write_state(&ThemeState::default())?;
        }
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn list_themes(&self) -> anyhow::Result<CodexThemeList> {
        let _lock = self.acquire_lock()?;
        self.recover_pending_locked()?;
        let state = self.read_state()?;
        let mut themes = Vec::with_capacity(state.themes.len() + 1);
        themes.push(CodexThemeSummary {
            id: DEFAULT_THEME_ID.to_string(),
            name: "Codex 默认主题".to_string(),
            version: "builtin".to_string(),
            author: "Codex".to_string(),
            description: "移除 CCP 主题覆盖，恢复 Codex 原始外观。".to_string(),
            preview_data_uri: None,
            builtin: true,
            current: state.current_theme_id == DEFAULT_THEME_ID,
            imported_at: 0,
            updated_at: 0,
            integrity_sha256: None,
            previous_version_available: false,
        });

        let mut installed = state.themes.clone();
        installed.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.manifest.name.cmp(&right.manifest.name))
        });
        for item in installed {
            themes.push(self.summary_for(&item, &state.current_theme_id)?);
        }
        Ok(CodexThemeList {
            themes,
            current_theme_id: state.current_theme_id,
            generation: state.generation,
        })
    }

    pub fn import_theme(&self, source: impl AsRef<Path>) -> anyhow::Result<CodexThemeSummary> {
        self.import_theme_with_options(source, false)
    }

    pub fn import_theme_with_options(
        &self,
        source: impl AsRef<Path>,
        replace_existing: bool,
    ) -> anyhow::Result<CodexThemeSummary> {
        let source = source.as_ref();
        if !source.exists() {
            bail!("主题来源不存在");
        }
        let _lock = self.acquire_lock()?;
        self.recover_pending_locked()?;
        let mut state = self.read_state()?;
        let operation_id = operation_id();
        let staging_dir = self.staging_dir().join(&operation_id);
        let package_dir = staging_dir.join("package");
        fs::create_dir_all(&package_dir).context("无法创建主题暂存目录")?;

        let prepare_result = if source.is_dir() {
            copy_directory_checked(source, &package_dir)
        } else {
            extract_archive_checked(source, &package_dir)
        };
        if let Err(error) = prepare_result {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(error);
        }

        let prepared = locate_package_root(&package_dir).and_then(|package_root| {
            validate_package(&package_root).map(|validated| (package_root, validated))
        });
        let (package_root, (manifest, css, integrity_sha256)) = match prepared {
            Ok(value) => value,
            Err(error) => {
                let _ = fs::remove_dir_all(&staging_dir);
                return Err(error);
            }
        };
        if manifest.id == DEFAULT_THEME_ID {
            let _ = fs::remove_dir_all(&staging_dir);
            bail!("default 是保留主题标识");
        }
        if !replace_existing
            && state
                .themes
                .iter()
                .any(|item| item.manifest.id == manifest.id)
        {
            let _ = fs::remove_dir_all(&staging_dir);
            bail!("主题 ID 已存在，需要确认后才能替换上一版本");
        }

        let target_dir = self.library_dir().join(&manifest.id);
        let backup_dir = self.backups_dir().join(&manifest.id).join(&operation_id);
        let replacing_active_theme = state.current_theme_id == manifest.id;
        let state_before = state.clone();
        let mut journal = MutationJournal {
            operation_id: operation_id.clone(),
            operation_type: "import".to_string(),
            theme_id: manifest.id.clone(),
            phase: "prepared".to_string(),
            started_at: now_secs(),
            state_before,
            staging_dir: Some(PathBuf::from("staging").join(&operation_id)),
            target_dir: Some(PathBuf::from("library").join(&manifest.id)),
            backup_dir: Some(
                PathBuf::from("backups")
                    .join(&manifest.id)
                    .join(&operation_id),
            ),
            finished_at: None,
            result: None,
        };
        self.write_journal(&journal)?;

        let transaction_result = (|| -> anyhow::Result<()> {
            if target_dir.exists() {
                let parent = backup_dir.parent().context("主题备份目录无效")?;
                fs::create_dir_all(parent)?;
                fs::rename(&target_dir, &backup_dir).context("主题正在被占用，无法保留上一版本")?;
                journal.phase = "backup-created".to_string();
                self.write_journal(&journal)?;
            }

            fs::rename(&package_root, &target_dir).context("主题资源被占用，原子替换失败")?;
            journal.phase = "files-swapped".to_string();
            self.write_journal(&journal)?;

            let (_, committed_css, committed_integrity) =
                validate_package(&target_dir).context("主题原子替换后的完整性复核失败")?;
            if committed_css != css || committed_integrity != integrity_sha256 {
                bail!("主题原子替换后的内容与暂存版本不一致");
            }

            let now = now_secs();
            if let Some(existing) = state
                .themes
                .iter_mut()
                .find(|item| item.manifest.id == manifest.id)
            {
                existing.manifest = manifest.clone();
                existing.updated_at = now;
                existing.integrity_sha256 = integrity_sha256.clone();
                existing.previous_version_available = backup_dir.exists();
            } else {
                state.themes.push(InstalledTheme {
                    manifest: manifest.clone(),
                    imported_at: now,
                    updated_at: now,
                    integrity_sha256: integrity_sha256.clone(),
                    previous_version_available: false,
                });
            }
            if replacing_active_theme {
                state.generation = state.generation.saturating_add(1);
            }
            self.write_state(&state)?;
            journal.phase = "state-committed".to_string();
            self.write_journal(&journal)?;
            Ok(())
        })();

        if let Err(error) = transaction_result {
            let rollback = self.rollback_journal(&journal);
            if let Err(rollback_error) = rollback {
                return Err(error.context(format!("主题导入失败，回滚也失败: {rollback_error:#}")));
            }
            return Err(error);
        }

        let _ = fs::remove_dir_all(&staging_dir);
        self.archive_journal(&journal, "committed")?;
        let committed = self.read_state()?;
        let installed = committed
            .themes
            .iter()
            .find(|item| item.manifest.id == manifest.id)
            .context("主题提交后状态复核失败")?;
        if css.is_empty() {
            bail!("主题提交后样式复核失败");
        }
        self.summary_for(installed, &committed.current_theme_id)
    }

    pub fn apply_theme(&self, theme_id: &str) -> anyhow::Result<CodexThemeOperationResult> {
        if theme_id == DEFAULT_THEME_ID {
            return self.restore_default_theme();
        }
        validate_theme_id(theme_id, false)?;
        let _lock = self.acquire_lock()?;
        self.recover_pending_locked()?;
        let mut state = self.read_state()?;
        if !state.themes.iter().any(|item| item.manifest.id == theme_id) {
            bail!("主题不存在或尚未通过校验");
        }
        if state.current_theme_id == theme_id {
            return Ok(operation_result(
                theme_id,
                state.generation,
                "该主题已在使用中，重启 Codex 可重新加载。",
            ));
        }
        self.commit_active_theme(&mut state, theme_id)?;
        Ok(operation_result(
            theme_id,
            state.generation,
            "主题已保存，重启 Codex 后生效。",
        ))
    }

    pub fn restore_default_theme(&self) -> anyhow::Result<CodexThemeOperationResult> {
        let _lock = self.acquire_lock()?;
        self.recover_pending_locked()?;
        let mut state = self.read_state()?;
        if state.current_theme_id != DEFAULT_THEME_ID {
            self.commit_active_theme(&mut state, DEFAULT_THEME_ID)?;
        }
        let payload = self.active_theme_payload_for_state(&state)?;
        if !payload.is_default || !payload.css.is_empty() {
            bail!("默认主题清理复核失败");
        }
        Ok(operation_result(
            DEFAULT_THEME_ID,
            state.generation,
            "CCP 主题覆盖已清理，重启 Codex 后恢复默认外观。",
        ))
    }

    pub fn active_theme_payload(&self) -> anyhow::Result<CodexThemePayload> {
        let _lock = self.acquire_lock()?;
        self.recover_pending_locked()?;
        let state = self.read_state()?;
        self.active_theme_payload_for_state(&state)
    }

    fn commit_active_theme(&self, state: &mut ThemeState, theme_id: &str) -> anyhow::Result<()> {
        let operation_id = operation_id();
        let mut journal = MutationJournal {
            operation_id: operation_id.clone(),
            operation_type: if theme_id == DEFAULT_THEME_ID {
                "restore-default".to_string()
            } else {
                "apply".to_string()
            },
            theme_id: theme_id.to_string(),
            phase: "prepared".to_string(),
            started_at: now_secs(),
            state_before: state.clone(),
            staging_dir: None,
            target_dir: None,
            backup_dir: None,
            finished_at: None,
            result: None,
        };
        self.write_journal(&journal)?;
        state.previous_theme_id = Some(state.current_theme_id.clone());
        state.current_theme_id = theme_id.to_string();
        state.generation = state.generation.saturating_add(1);
        if let Err(error) = self.write_state(state) {
            let _ = self.rollback_journal(&journal);
            return Err(error);
        }
        journal.phase = "state-committed".to_string();
        if let Err(error) = self.write_journal(&journal) {
            let _ = self.write_state(&journal.state_before);
            return Err(error.context("主题状态已回滚"));
        }
        let verified = self.read_state()?;
        if verified.current_theme_id != theme_id || verified.generation != state.generation {
            self.rollback_journal(&journal)?;
            bail!("主题状态提交后复核失败，已恢复上一状态");
        }
        self.archive_journal(&journal, "committed")?;
        Ok(())
    }

    fn active_theme_payload_for_state(
        &self,
        state: &ThemeState,
    ) -> anyhow::Result<CodexThemePayload> {
        if state.current_theme_id == DEFAULT_THEME_ID {
            return Ok(CodexThemePayload {
                theme_id: DEFAULT_THEME_ID.to_string(),
                generation: state.generation,
                css: String::new(),
                css_variables: BTreeMap::new(),
                root_attributes: CodexThemeRootAttributes::default(),
                asset_data_uris: BTreeMap::new(),
                is_default: true,
            });
        }
        let installed = state
            .themes
            .iter()
            .find(|item| item.manifest.id == state.current_theme_id)
            .context("当前主题记录已损坏")?;
        let style_path = checked_join(
            &self.library_dir().join(&installed.manifest.id),
            &installed.manifest.entry_style,
        )?;
        let css = fs::read_to_string(&style_path).context("当前主题样式不可读取")?;
        validate_css(&css)?;
        let runtime = compile_runtime_resources(
            &self.library_dir().join(&installed.manifest.id),
            &installed.manifest,
        )?;
        Ok(CodexThemePayload {
            theme_id: installed.manifest.id.clone(),
            generation: state.generation,
            css,
            css_variables: runtime.css_variables,
            root_attributes: runtime.root_attributes,
            asset_data_uris: runtime.asset_data_uris,
            is_default: false,
        })
    }

    fn summary_for(
        &self,
        item: &InstalledTheme,
        current_theme_id: &str,
    ) -> anyhow::Result<CodexThemeSummary> {
        let package_root = self.library_dir().join(&item.manifest.id);
        let preview_path = checked_join(&package_root, &item.manifest.preview)?;
        let preview = fs::read(&preview_path).context("主题预览图不可读取")?;
        let mime = image_mime(&preview).context("主题预览图格式不受支持")?;
        Ok(CodexThemeSummary {
            id: item.manifest.id.clone(),
            name: item.manifest.name.clone(),
            version: item.manifest.version.clone(),
            author: item.manifest.author.clone(),
            description: item.manifest.description.clone(),
            preview_data_uri: Some(format!(
                "data:{mime};base64,{}",
                base64::engine::general_purpose::STANDARD.encode(preview)
            )),
            builtin: false,
            current: current_theme_id == item.manifest.id,
            imported_at: item.imported_at,
            updated_at: item.updated_at,
            integrity_sha256: Some(item.integrity_sha256.clone()),
            previous_version_available: item.previous_version_available,
        })
    }

    fn recover_pending_locked(&self) -> anyhow::Result<()> {
        for entry in fs::read_dir(self.journal_dir()).context("无法读取主题事务日志")? {
            let path = entry?.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let bytes = fs::read(&path)?;
            let journal: MutationJournal = serde_json::from_slice(&bytes)
                .with_context(|| format!("主题事务日志损坏: {}", path.display()))?;
            if journal.phase == "state-committed" {
                if self.journal_commit_is_valid(&journal)? {
                    if let Some(staging) =
                        self.resolve_journal_path(journal.staging_dir.as_ref())?
                    {
                        let _ = fs::remove_dir_all(staging);
                    }
                    self.archive_journal(&journal, "recovered-commit")?;
                } else {
                    self.rollback_journal(&journal)?;
                }
                continue;
            }
            self.rollback_journal(&journal)?;
        }
        Ok(())
    }

    fn rollback_journal(&self, journal: &MutationJournal) -> anyhow::Result<()> {
        let previous_theme_existed = journal
            .state_before
            .themes
            .iter()
            .any(|item| item.manifest.id == journal.theme_id);
        if let Some(target) = self.resolve_journal_path(journal.target_dir.as_ref())? {
            let backup = self.resolve_journal_path(journal.backup_dir.as_ref())?;
            let backup_exists = backup.as_ref().is_some_and(|path| path.exists());
            if target.exists() && (backup_exists || !previous_theme_existed) {
                fs::remove_dir_all(&target).context("无法清理未提交主题")?;
            }
            if let Some(backup) = backup {
                if backup.exists() && !target.exists() {
                    fs::rename(backup, target).context("无法恢复上一主题版本")?;
                }
            }
        }
        self.write_state(&journal.state_before)?;
        if let Some(staging) = self.resolve_journal_path(journal.staging_dir.as_ref())? {
            let _ = fs::remove_dir_all(staging);
        }
        self.archive_journal(journal, "rolled-back")
    }

    fn journal_commit_is_valid(&self, journal: &MutationJournal) -> anyhow::Result<bool> {
        let state = self.read_state()?;
        if journal.operation_type == "import" {
            let Some(installed) = state
                .themes
                .iter()
                .find(|item| item.manifest.id == journal.theme_id)
            else {
                return Ok(false);
            };
            let Some(target) = self.resolve_journal_path(journal.target_dir.as_ref())? else {
                return Ok(false);
            };
            let Ok((manifest, _, integrity)) = validate_package(&target) else {
                return Ok(false);
            };
            return Ok(manifest == installed.manifest && integrity == installed.integrity_sha256);
        }
        Ok(state.current_theme_id == journal.theme_id)
    }

    fn ensure_layout(&self) -> anyhow::Result<()> {
        for directory in [
            self.root.clone(),
            self.library_dir(),
            self.staging_dir(),
            self.journal_dir(),
            self.history_dir(),
            self.backups_dir(),
        ] {
            fs::create_dir_all(&directory)
                .with_context(|| format!("无法创建主题目录: {}", directory.display()))?;
        }
        Ok(())
    }

    fn acquire_lock(&self) -> anyhow::Result<File> {
        let path = self.root.join(LOCK_FILE);
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&path)
            .context("无法打开主题仓库锁")?;
        file.try_lock_exclusive()
            .context("主题仓库正被其他操作占用，请稍后重试")?;
        Ok(file)
    }

    fn read_state(&self) -> anyhow::Result<ThemeState> {
        if !self.state_path().exists() {
            return Ok(ThemeState::default());
        }
        let bytes = fs::read(self.state_path()).context("无法读取主题状态")?;
        let state: ThemeState = serde_json::from_slice(&bytes).context("主题状态文件损坏")?;
        if state.schema_version != 1 {
            bail!("主题状态版本不受支持");
        }
        Ok(state)
    }

    fn write_state(&self, state: &ThemeState) -> anyhow::Result<()> {
        let bytes = serde_json::to_vec_pretty(state)?;
        crate::settings::atomic_write(&self.state_path(), &bytes)
    }

    fn write_journal(&self, journal: &MutationJournal) -> anyhow::Result<()> {
        let bytes = serde_json::to_vec_pretty(journal)?;
        crate::settings::atomic_write(
            &self
                .journal_dir()
                .join(format!("{}.json", journal.operation_id)),
            &bytes,
        )
    }

    fn remove_journal(&self, operation_id: &str) -> anyhow::Result<()> {
        let path = self.journal_dir().join(format!("{operation_id}.json"));
        if path.exists() {
            fs::remove_file(path).context("无法清理主题事务日志")?;
        }
        Ok(())
    }

    fn archive_journal(&self, journal: &MutationJournal, result: &str) -> anyhow::Result<()> {
        let mut completed = journal.clone();
        completed.finished_at = Some(now_secs());
        completed.result = Some(result.to_string());
        let bytes = serde_json::to_vec_pretty(&completed)?;
        crate::settings::atomic_write(
            &self
                .history_dir()
                .join(format!("{}.json", completed.operation_id)),
            &bytes,
        )?;
        self.remove_journal(&journal.operation_id)
    }

    fn resolve_journal_path(&self, path: Option<&PathBuf>) -> anyhow::Result<Option<PathBuf>> {
        let Some(path) = path else {
            return Ok(None);
        };
        if path.is_absolute()
            || path
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            bail!("主题事务日志包含越界路径");
        }
        Ok(Some(self.root.join(path)))
    }

    fn state_path(&self) -> PathBuf {
        self.root.join(STATE_FILE)
    }

    fn library_dir(&self) -> PathBuf {
        self.root.join("library")
    }

    fn staging_dir(&self) -> PathBuf {
        self.root.join("staging")
    }

    fn journal_dir(&self) -> PathBuf {
        self.root.join("journal")
    }

    fn history_dir(&self) -> PathBuf {
        self.root.join("history")
    }

    fn backups_dir(&self) -> PathBuf {
        self.root.join("backups")
    }
}

fn operation_result(theme_id: &str, generation: u64, message: &str) -> CodexThemeOperationResult {
    CodexThemeOperationResult {
        theme_id: theme_id.to_string(),
        persisted: true,
        runtime_applied: false,
        restart_required: true,
        rolled_back: false,
        generation,
        message: message.to_string(),
    }
}

fn operation_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{}-{nanos}", std::process::id())
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn locate_package_root(staging: &Path) -> anyhow::Result<PathBuf> {
    if find_manifest_path(staging).is_some() {
        return Ok(staging.to_path_buf());
    }
    let entries = fs::read_dir(staging)?.collect::<Result<Vec<_>, _>>()?;
    if entries.len() == 1 && entries[0].file_type()?.is_dir() {
        let nested = entries[0].path();
        if find_manifest_path(&nested).is_some() {
            return Ok(nested);
        }
    }
    bail!("主题包缺少 theme.json 或 theme.manifest.json")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompiledRuntimeResources {
    css_variables: BTreeMap<String, String>,
    root_attributes: CodexThemeRootAttributes,
    asset_data_uris: BTreeMap<String, String>,
}

fn compile_runtime_resources(
    root: &Path,
    manifest: &CodexThemeManifest,
) -> anyhow::Result<CompiledRuntimeResources> {
    if manifest.css_variables.len() > MAX_CSS_VARIABLES {
        bail!("主题 CSS 变量数量超过限制");
    }
    if manifest.root_attributes.classes.len() > MAX_ROOT_CLASSES {
        bail!("主题根类数量超过限制");
    }
    if manifest.root_attributes.attributes.len() > MAX_ROOT_ATTRIBUTES {
        bail!("主题根属性数量超过限制");
    }
    if manifest.asset_variables.len() > MAX_ASSET_VARIABLES {
        bail!("主题图片变量数量超过限制");
    }

    let mut css_values_bytes = 0_usize;
    for (name, value) in &manifest.css_variables {
        validate_theme_variable_name(name)?;
        validate_css_variable_value(value)?;
        css_values_bytes = css_values_bytes.saturating_add(value.len());
        if css_values_bytes > MAX_CSS_VARIABLE_VALUES_BYTES {
            bail!("主题 CSS 变量值总大小超过限制");
        }
    }

    let mut root_classes = BTreeSet::new();
    for class_name in &manifest.root_attributes.classes {
        if !is_namespaced_identifier(class_name, "ccp-theme-") {
            bail!("主题根类必须使用 ccp-theme-* 命名空间");
        }
        if !root_classes.insert(class_name) {
            bail!("主题根类存在重复项");
        }
    }
    for (name, value) in &manifest.root_attributes.attributes {
        if !is_namespaced_identifier(name, "data-ccp-theme-") || is_reserved_root_attribute(name) {
            bail!("主题根属性无效或使用了保留名称");
        }
        if value.len() > MAX_ROOT_ATTRIBUTE_VALUE_BYTES || value.chars().any(char::is_control) {
            bail!("主题根属性值无效或超过大小限制");
        }
    }

    let declared_assets = manifest.assets.iter().collect::<BTreeSet<_>>();
    if declared_assets.len() != manifest.assets.len() {
        bail!("主题资源清单存在重复路径");
    }
    let mut asset_data_uris = BTreeMap::new();
    let mut data_uri_bytes = 0_usize;
    for (name, relative_path) in &manifest.asset_variables {
        validate_theme_variable_name(name)?;
        if manifest.css_variables.contains_key(name) {
            bail!("主题变量不能同时由 CSS 值和图片资源拥有");
        }
        if !declared_assets.contains(relative_path) {
            bail!("主题图片变量引用了未声明资源");
        }
        let path = checked_join(root, relative_path)?;
        let bytes = fs::read(&path).context("主题图片变量资源不可读取")?;
        let detected_mime = image_mime(&bytes).context("主题图片变量只允许 PNG、JPEG 或 WebP")?;
        if expected_image_mime(&path) != Some(detected_mime) {
            bail!("主题图片变量的扩展名与实际格式不一致");
        }
        let data_uri = format!(
            "data:{detected_mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        );
        data_uri_bytes = data_uri_bytes.saturating_add(data_uri.len());
        if data_uri_bytes > MAX_RUNTIME_ASSET_DATA_URI_BYTES {
            bail!("主题运行时图片载荷超过大小限制");
        }
        asset_data_uris.insert(name.clone(), data_uri);
    }

    Ok(CompiledRuntimeResources {
        css_variables: manifest.css_variables.clone(),
        root_attributes: manifest.root_attributes.clone(),
        asset_data_uris,
    })
}

fn validate_theme_variable_name(value: &str) -> anyhow::Result<()> {
    if !is_namespaced_identifier(value, "--ccp-theme-") || is_reserved_theme_variable(value) {
        bail!("主题变量无效或使用了保留名称");
    }
    Ok(())
}

fn validate_css_variable_value(value: &str) -> anyhow::Result<()> {
    if value.trim().is_empty()
        || value.len() > MAX_CSS_VARIABLE_VALUE_BYTES
        || value.chars().any(char::is_control)
    {
        bail!("主题 CSS 变量值无效或超过大小限制");
    }
    let lowered = value.to_ascii_lowercase();
    if value.contains(';')
        || value.contains('{')
        || value.contains('}')
        || lowered.contains("url(")
        || lowered.contains("@import")
        || lowered.contains("expression(")
        || lowered.contains("javascript:")
        || lowered.contains("!important")
    {
        bail!("主题 CSS 变量值包含不受支持的内容");
    }
    Ok(())
}

fn is_namespaced_identifier(value: &str, prefix: &str) -> bool {
    let Some(suffix) = value.strip_prefix(prefix) else {
        return false;
    };
    if suffix.is_empty() || suffix.len() > 63 {
        return false;
    }
    let mut bytes = suffix.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_lowercase() || first.is_ascii_digit())
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn is_reserved_theme_variable(value: &str) -> bool {
    matches!(
        value,
        "--ccp-theme-id"
            | "--ccp-theme-generation"
            | "--ccp-theme-active"
            | "--ccp-theme-payload-sha256"
    )
}

fn is_reserved_root_attribute(value: &str) -> bool {
    matches!(
        value,
        "data-ccp-theme-id"
            | "data-ccp-theme-generation"
            | "data-ccp-theme-active"
            | "data-ccp-theme-payload-sha256"
    )
}

fn expected_image_mime(path: &Path) -> Option<&'static str> {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

fn validate_package(root: &Path) -> anyhow::Result<(CodexThemeManifest, String, String)> {
    let manifest_path = find_manifest_path(root).context("主题包缺少 manifest")?;
    let bytes = fs::read(&manifest_path).context("主题 manifest 不可读取")?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        bail!("主题 manifest 超过大小限制");
    }
    let manifest: CodexThemeManifest =
        serde_json::from_slice(&bytes).context("主题 manifest 格式错误")?;
    if manifest.format_version != 1 {
        bail!("主题格式版本不受支持");
    }
    validate_theme_id(&manifest.id, false)?;
    validate_text_field("主题名称", &manifest.name, 1, 80)?;
    validate_text_field("主题版本", &manifest.version, 1, 40)?;
    validate_text_field("主题作者", &manifest.author, 1, 80)?;
    if manifest.description.len() > 400 {
        bail!("主题描述过长");
    }

    let preview_path = checked_join(root, &manifest.preview)?;
    let preview_meta = fs::metadata(&preview_path).context("主题预览图不存在")?;
    if !preview_meta.is_file() || preview_meta.len() > MAX_PREVIEW_BYTES {
        bail!("主题预览图无效或超过大小限制");
    }
    let preview = fs::read(&preview_path)?;
    image_mime(&preview).context("主题预览图必须是 PNG、JPEG 或 WebP")?;

    let style_path = checked_join(root, &manifest.entry_style)?;
    let style_meta = fs::metadata(&style_path).context("主题样式不存在")?;
    if !style_meta.is_file() || style_meta.len() > MAX_CSS_BYTES {
        bail!("主题样式无效或超过大小限制");
    }
    if style_path.extension().and_then(|value| value.to_str()) != Some("css") {
        bail!("主题样式入口必须是 CSS 文件");
    }
    let css = fs::read_to_string(&style_path).context("主题 CSS 必须是 UTF-8 文本")?;
    validate_css(&css)?;

    for asset in &manifest.assets {
        let path = checked_join(root, asset)?;
        let metadata = fs::metadata(&path).context("主题声明的资源不存在")?;
        if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
            bail!("主题资源无效或超过大小限制");
        }
        validate_asset_extension(&path)?;
    }

    compile_runtime_resources(root, &manifest)?;

    let integrity = hash_directory(root)?;
    Ok((manifest, css, integrity))
}

fn validate_theme_id(value: &str, allow_default: bool) -> anyhow::Result<()> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-_".contains(&byte))
    {
        bail!("主题 ID 仅允许小写字母、数字、短横线和下划线");
    }
    if !allow_default && value == DEFAULT_THEME_ID {
        bail!("default 是保留主题标识");
    }
    Ok(())
}

fn validate_text_field(label: &str, value: &str, min: usize, max: usize) -> anyhow::Result<()> {
    let length = value.trim().chars().count();
    if length < min || length > max || value.chars().any(char::is_control) {
        bail!("{label}无效");
    }
    Ok(())
}

fn validate_css(css: &str) -> anyhow::Result<()> {
    if css.trim().is_empty() {
        bail!("主题 CSS 为空");
    }
    let lowered = css.to_ascii_lowercase();
    for forbidden in [
        "@import",
        "javascript:",
        "expression(",
        "-moz-binding",
        "url(http:",
        "url(https:",
        "url(file:",
        "url(\"http:",
        "url(\"https:",
        "url('http:",
        "url('https:",
    ] {
        if lowered.contains(forbidden) {
            bail!("主题 CSS 包含远程加载或可执行内容");
        }
    }
    Ok(())
}

fn validate_asset_extension(path: &Path) -> anyhow::Result<()> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(
        extension.as_str(),
        "css" | "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" | "woff" | "woff2" | "ttf"
    ) {
        bail!("主题包含不受支持的资源类型");
    }
    Ok(())
}

fn find_manifest_path(root: &Path) -> Option<PathBuf> {
    ["theme.json", "theme.manifest.json"]
        .into_iter()
        .map(|name| root.join(name))
        .find(|path| path.is_file())
}

fn checked_join(root: &Path, relative: &str) -> anyhow::Result<PathBuf> {
    let path = Path::new(relative);
    if path.is_absolute() || relative.contains(':') || relative.contains('\0') {
        bail!("主题资源路径必须是安全的相对路径");
    }
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("主题资源路径包含越界片段");
    }
    let joined = root.join(path);
    let metadata = fs::symlink_metadata(&joined).context("主题资源路径不存在")?;
    if metadata.file_type().is_symlink() {
        bail!("主题资源不允许使用符号链接");
    }
    Ok(joined)
}

fn copy_directory_checked(source: &Path, target: &Path) -> anyhow::Result<()> {
    let mut counters = CopyCounters::default();
    copy_directory_inner(source, target, &mut counters)
}

#[derive(Default)]
struct CopyCounters {
    files: usize,
    bytes: u64,
}

fn copy_directory_inner(
    source: &Path,
    target: &Path,
    counters: &mut CopyCounters,
) -> anyhow::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source).context("无法读取主题来源目录")? {
        let entry = entry?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata.file_type().is_symlink() {
            bail!("主题包不允许符号链接或重解析点");
        }
        let destination = target.join(entry.file_name());
        if metadata.is_dir() {
            copy_directory_inner(&entry.path(), &destination, counters)?;
            continue;
        }
        if !metadata.is_file() {
            bail!("主题包包含不受支持的文件类型");
        }
        counters.files += 1;
        counters.bytes = counters.bytes.saturating_add(metadata.len());
        if counters.files > MAX_FILES
            || metadata.len() > MAX_FILE_BYTES
            || counters.bytes > MAX_TOTAL_BYTES
        {
            bail!("主题包超过文件数量或大小限制");
        }
        fs::copy(entry.path(), destination)?;
    }
    Ok(())
}

fn extract_archive_checked(source: &Path, target: &Path) -> anyhow::Result<()> {
    if source.extension().and_then(|value| value.to_str()) != Some("zip") {
        bail!("请选择主题目录或 ZIP 主题包");
    }
    let bytes = fs::read(source).context("无法读取主题压缩包")?;
    if bytes.len() as u64 > MAX_TOTAL_BYTES {
        bail!("主题压缩包超过大小限制");
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).context("主题压缩包已损坏")?;
    if archive.len() > MAX_FILES {
        bail!("主题压缩包文件过多");
    }
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let Some(enclosed) = entry.enclosed_name() else {
            bail!("主题压缩包包含越界路径");
        };
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            bail!("主题压缩包不允许符号链接");
        }
        if entry.size() > MAX_FILE_BYTES {
            bail!("主题压缩包包含超大文件");
        }
        total = total.saturating_add(entry.size());
        if total > MAX_TOTAL_BYTES {
            bail!("主题解压后超过大小限制");
        }
        let output = target.join(enclosed);
        if entry.is_dir() {
            fs::create_dir_all(&output)?;
            continue;
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(&output)?;
        std::io::copy(&mut entry, &mut file)?;
    }
    Ok(())
}

fn image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        return Some("image/jpeg");
    }
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    None
}

fn hash_directory(root: &Path) -> anyhow::Result<String> {
    let mut paths = Vec::new();
    collect_files(root, root, &mut paths)?;
    paths.sort();
    let mut hasher = Sha256::new();
    for relative in paths {
        hasher.update(relative.to_string_lossy().as_bytes());
        let mut file = File::open(root.join(&relative))?;
        let mut buffer = [0_u8; 8192];
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn collect_files(root: &Path, current: &Path, output: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata.file_type().is_symlink() {
            bail!("主题资源不允许符号链接");
        }
        if metadata.is_dir() {
            collect_files(root, &entry.path(), output)?;
        } else if metadata.is_file() {
            output.push(entry.path().strip_prefix(root)?.to_path_buf());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG_1X1: &[u8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 0];

    fn write_theme(root: &Path, id: &str, css: &str) {
        write_theme_version(root, id, "1.0.0", css);
    }

    fn write_theme_version(root: &Path, id: &str, version: &str, css: &str) {
        fs::create_dir_all(root.join("assets")).unwrap();
        fs::write(root.join("preview.png"), PNG_1X1).unwrap();
        fs::write(root.join("assets/theme.css"), css).unwrap();
        let manifest = serde_json::json!({
            "format_version": 1,
            "id": id,
            "name": format!("Theme {id}"),
            "version": version,
            "author": "CCP Test",
            "description": "test theme",
            "preview": "preview.png",
            "entry_style": "assets/theme.css",
            "assets": ["assets/theme.css"]
        });
        fs::write(
            root.join("theme.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
    }

    fn write_theme_with_runtime_resources(
        root: &Path,
        id: &str,
        css_variables: serde_json::Value,
        root_attributes: serde_json::Value,
        asset_variables: serde_json::Value,
        declared_assets: &[&str],
    ) {
        fs::create_dir_all(root.join("assets")).unwrap();
        fs::write(root.join("preview.png"), PNG_1X1).unwrap();
        fs::write(
            root.join("assets/theme.css"),
            ":root.ccp-theme-runtime { background-image: var(--ccp-theme-art); }",
        )
        .unwrap();
        fs::write(root.join("assets/hero.png"), PNG_1X1).unwrap();
        fs::write(root.join("assets/not-image.css"), ":root {}").unwrap();
        let manifest = serde_json::json!({
            "format_version": 1,
            "id": id,
            "name": format!("Theme {id}"),
            "version": "1.0.0",
            "author": "CCP Test",
            "description": "runtime resource test theme",
            "preview": "preview.png",
            "entry_style": "assets/theme.css",
            "assets": declared_assets,
            "css_variables": css_variables,
            "root_attributes": root_attributes,
            "asset_variables": asset_variables
        });
        fs::write(
            root.join("theme.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn default_theme_is_always_first() {
        let temp = tempfile::tempdir().unwrap();
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();
        let list = store.list_themes().unwrap();
        assert_eq!(list.themes[0].id, DEFAULT_THEME_ID);
        assert!(list.themes[0].current);
    }

    #[test]
    fn import_apply_and_restore_are_persistent_and_isolated() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        write_theme(&source, "quiet-dark", ":root { --ccp-test: #111; }");
        let root = temp.path().join("store");
        let store = CodexThemeStore::open(&root).unwrap();
        let imported = store.import_theme(&source).unwrap();
        assert_eq!(imported.id, "quiet-dark");
        assert!(
            imported
                .preview_data_uri
                .as_deref()
                .unwrap()
                .starts_with("data:image/png;base64,")
        );

        let applied = store.apply_theme("quiet-dark").unwrap();
        assert!(applied.persisted);
        assert!(applied.restart_required);
        assert!(!applied.runtime_applied);
        let payload = CodexThemeStore::open(&root)
            .unwrap()
            .active_theme_payload()
            .unwrap();
        assert_eq!(payload.theme_id, "quiet-dark");
        assert!(payload.css.contains("--ccp-test"));

        store.restore_default_theme().unwrap();
        let restored = store.active_theme_payload().unwrap();
        assert!(restored.is_default);
        assert!(restored.css.is_empty());
        assert!(restored.css_variables.is_empty());
        assert!(restored.root_attributes.classes.is_empty());
        assert!(restored.root_attributes.attributes.is_empty());
        assert!(restored.asset_data_uris.is_empty());
        assert_eq!(store.list_themes().unwrap().themes.len(), 2);
    }

    #[test]
    fn replacing_active_theme_refreshes_generation_and_runtime_payload() {
        let temp = tempfile::tempdir().unwrap();
        let active_v1 = temp.path().join("active-v1");
        let active_v2 = temp.path().join("active-v2");
        let inactive_v1 = temp.path().join("inactive-v1");
        let inactive_v2 = temp.path().join("inactive-v2");
        write_theme_version(
            &active_v1,
            "active-theme",
            "1.0.0",
            ":root { --ccp-active-version: v1; }",
        );
        write_theme_version(
            &active_v2,
            "active-theme",
            "1.1.0",
            ":root { --ccp-active-version: v2; }",
        );
        write_theme_version(
            &inactive_v1,
            "inactive-theme",
            "1.0.0",
            ":root { --ccp-inactive-version: v1; }",
        );
        write_theme_version(
            &inactive_v2,
            "inactive-theme",
            "1.1.0",
            ":root { --ccp-inactive-version: v2; }",
        );

        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();
        store.import_theme(&active_v1).unwrap();
        store.import_theme(&inactive_v1).unwrap();
        let applied = store.apply_theme("active-theme").unwrap();

        let replaced = store.import_theme_with_options(&active_v2, true).unwrap();
        assert_eq!(replaced.version, "1.1.0");
        assert!(replaced.current);
        assert!(replaced.previous_version_available);
        let refreshed = store.active_theme_payload().unwrap();
        assert_eq!(refreshed.theme_id, "active-theme");
        assert_eq!(refreshed.generation, applied.generation + 1);
        assert!(refreshed.css.contains("--ccp-active-version: v2"));
        assert!(!refreshed.css.contains("--ccp-active-version: v1"));

        let generation_before_inactive_update = refreshed.generation;
        store.import_theme_with_options(&inactive_v2, true).unwrap();
        let after_inactive_update = store.active_theme_payload().unwrap();
        assert_eq!(
            after_inactive_update.generation,
            generation_before_inactive_update
        );
        assert!(
            after_inactive_update
                .css
                .contains("--ccp-active-version: v2")
        );
    }

    #[test]
    fn runtime_resources_are_validated_and_embedded_in_payload() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        write_theme_with_runtime_resources(
            &source,
            "runtime-theme",
            serde_json::json!({"--ccp-theme-accent": "#10b981"}),
            serde_json::json!({
                "classes": ["ccp-theme-runtime"],
                "attributes": {"data-ccp-theme-tone": "dark"}
            }),
            serde_json::json!({"--ccp-theme-art": "assets/hero.png"}),
            &["assets/theme.css", "assets/hero.png"],
        );

        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();
        store.import_theme(&source).unwrap();
        store.apply_theme("runtime-theme").unwrap();
        let payload = store.active_theme_payload().unwrap();

        assert_eq!(payload.css_variables["--ccp-theme-accent"], "#10b981");
        assert_eq!(
            payload.root_attributes.classes,
            vec!["ccp-theme-runtime".to_string()]
        );
        assert_eq!(
            payload.root_attributes.attributes["data-ccp-theme-tone"],
            "dark"
        );
        assert!(payload.asset_data_uris["--ccp-theme-art"].starts_with("data:image/png;base64,"));
    }

    #[test]
    fn runtime_resources_reject_unsafe_or_undeclared_values() {
        let temp = tempfile::tempdir().unwrap();
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();

        let cases = [
            (
                "bad-variable",
                serde_json::json!({"runtime-accent": "#10b981"}),
                serde_json::json!({
                    "classes": ["ccp-theme-runtime"],
                    "attributes": {"data-ccp-theme-tone": "dark"}
                }),
                serde_json::json!({}),
                vec!["assets/theme.css"],
            ),
            (
                "bad-attribute",
                serde_json::json!({}),
                serde_json::json!({"classes": [], "attributes": {"class": "dark"}}),
                serde_json::json!({}),
                vec!["assets/theme.css"],
            ),
            (
                "reserved-attribute",
                serde_json::json!({}),
                serde_json::json!({
                    "classes": [],
                    "attributes": {"data-ccp-theme-id": "spoofed"}
                }),
                serde_json::json!({}),
                vec!["assets/theme.css"],
            ),
            (
                "undeclared-image",
                serde_json::json!({}),
                serde_json::json!({"classes": [], "attributes": {}}),
                serde_json::json!({"--ccp-theme-art": "assets/hero.png"}),
                vec!["assets/theme.css"],
            ),
            (
                "non-image",
                serde_json::json!({}),
                serde_json::json!({"classes": [], "attributes": {}}),
                serde_json::json!({"--ccp-theme-art": "assets/not-image.css"}),
                vec!["assets/theme.css", "assets/not-image.css"],
            ),
            (
                "remote-variable",
                serde_json::json!({"--ccp-theme-art": "url(https://example.invalid/art.png)"}),
                serde_json::json!({"classes": [], "attributes": {}}),
                serde_json::json!({}),
                vec!["assets/theme.css"],
            ),
        ];

        for (id, css_variables, root_attributes, asset_variables, declared_assets) in cases {
            let source = temp.path().join(id);
            write_theme_with_runtime_resources(
                &source,
                id,
                css_variables,
                root_attributes,
                asset_variables,
                &declared_assets,
            );
            assert!(
                store.import_theme(&source).is_err(),
                "{id} must be rejected"
            );
        }

        let outside = temp.path().join("outside.png");
        fs::write(&outside, PNG_1X1).unwrap();
        let traversal = temp.path().join("path-traversal");
        write_theme_with_runtime_resources(
            &traversal,
            "path-traversal",
            serde_json::json!({}),
            serde_json::json!({"classes": [], "attributes": {}}),
            serde_json::json!({"--ccp-theme-art": "../outside.png"}),
            &["assets/theme.css", "../outside.png"],
        );
        assert!(store.import_theme(&traversal).is_err());
    }

    #[test]
    fn runtime_resources_reject_duplicate_variable_ownership() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("duplicate-variable");
        write_theme_with_runtime_resources(
            &source,
            "duplicate-variable",
            serde_json::json!({"--ccp-theme-art": "none"}),
            serde_json::json!({"classes": [], "attributes": {}}),
            serde_json::json!({"--ccp-theme-art": "assets/hero.png"}),
            &["assets/theme.css", "assets/hero.png"],
        );
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();
        assert!(store.import_theme(&source).is_err());
    }

    #[test]
    fn repository_theme_directories_and_archives_compile_to_the_same_payload() {
        let repository_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        for theme_id in ["codex-dream-skin-macos", "codex-dream-skin-windows"] {
            let directory = repository_root.join("Theme").join(theme_id);
            let archive = repository_root
                .join("Theme")
                .join(format!("{theme_id}.zip"));
            let expected_class = format!(
                "ccp-theme-{}",
                theme_id.strip_prefix("codex-").unwrap_or(theme_id)
            );
            let mut payloads = Vec::new();

            for source in [directory, archive] {
                assert!(
                    source.exists(),
                    "missing repository theme source: {source:?}"
                );
                let temp = tempfile::tempdir().unwrap();
                let store = CodexThemeStore::open(temp.path().join("store")).unwrap();
                let imported = store.import_theme(&source).unwrap();
                assert_eq!(imported.id, theme_id);
                store.apply_theme(theme_id).unwrap();
                let payload = store.active_theme_payload().unwrap();

                assert!(payload.css.contains("var(--ccp-theme-art)"));
                assert!(
                    payload.css.contains(r#"[data-feature="game-source"]"#)
                        || payload.css.contains(r#"[data-testid="home-icon"]"#),
                    "{theme_id} compiled from {source:?} must target a Codex native home fingerprint"
                );
                for legacy_selector in [
                    ".dream-home-shell",
                    ".dream-skin-home-shell",
                    ".dream-home",
                    ".dream-skin-home",
                ] {
                    assert!(
                        !payload.css.contains(legacy_selector),
                        "{theme_id} compiled from {source:?} must not depend on {legacy_selector}"
                    );
                }
                assert_eq!(
                    payload.root_attributes.classes,
                    vec![expected_class.clone()]
                );
                assert_eq!(
                    payload.root_attributes.attributes["data-ccp-theme-shell"],
                    "light"
                );
                assert!(
                    payload.asset_data_uris["--ccp-theme-art"]
                        .starts_with("data:image/png;base64,")
                );
                payloads.push(payload);
            }

            assert_eq!(payloads[0], payloads[1]);
        }
    }

    #[test]
    fn import_rejects_reserved_id_and_remote_css() {
        let temp = tempfile::tempdir().unwrap();
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();
        let reserved = temp.path().join("reserved");
        write_theme(&reserved, "default", ":root { color: red; }");
        assert!(store.import_theme(&reserved).is_err());

        let remote = temp.path().join("remote");
        write_theme(
            &remote,
            "remote-theme",
            "body { background: url(https://example.invalid/a.png); }",
        );
        assert!(store.import_theme(&remote).is_err());
    }

    #[test]
    fn repository_lock_reports_busy() {
        let temp = tempfile::tempdir().unwrap();
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();
        let held = store.acquire_lock().unwrap();
        let error = store.list_themes().unwrap_err();
        drop(held);
        assert!(format!("{error:#}").contains("占用"));
    }

    #[test]
    fn unfinished_apply_journal_restores_previous_state() {
        let temp = tempfile::tempdir().unwrap();
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();
        let before = store.read_state().unwrap();
        let mut changed = before.clone();
        changed.current_theme_id = "missing-theme".to_string();
        changed.generation = 8;
        store.write_state(&changed).unwrap();
        let journal = MutationJournal {
            operation_id: "interrupted".to_string(),
            operation_type: "apply".to_string(),
            theme_id: "missing-theme".to_string(),
            phase: "prepared".to_string(),
            started_at: now_secs(),
            state_before: before,
            staging_dir: None,
            target_dir: None,
            backup_dir: None,
            finished_at: None,
            result: None,
        };
        store.write_journal(&journal).unwrap();

        let reopened = CodexThemeStore::open(temp.path().join("store")).unwrap();
        let state = reopened.read_state().unwrap();
        assert_eq!(state.current_theme_id, DEFAULT_THEME_ID);
        assert!(!reopened.journal_dir().join("interrupted.json").exists());
    }
}
