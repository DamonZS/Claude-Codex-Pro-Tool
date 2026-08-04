use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, bail};
use base64::Engine;
use fs2::FileExt;
use futures_util::StreamExt;
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
const MAX_MANAGER_BACKGROUND_BYTES: u64 = 16 * 1024 * 1024;
const MAX_MANAGER_BACKGROUND_PIXELS: u64 = 100_000_000;
const MIN_MANAGER_BACKGROUND_WIDTH: u32 = 1280;
const MIN_MANAGER_BACKGROUND_HEIGHT: u32 = 720;
const USER_MANAGER_BACKGROUND_SOURCE: &str = "user-selected";
const MANAGER_BACKGROUND_FILE: &str = "current.bin";
const MANAGER_BACKGROUND_LIBRARY_DIR: &str = "library";
const DIY_THEME_ID_PREFIX: &str = "ccp-diy-";
const DIY_BUILD_PREFIX: &str = "diy-build-";
const DIY_IMAGE_LAYOUT_FULLSCREEN: &str = "fullscreen";
const DIY_IMAGE_LAYOUT_BANNER: &str = "banner";
const DIY_IMAGE_LAYOUT_CARD: &str = "card";
const DIY_TEXT_COLOR_LIGHT: &str = "#F3F5F7";
const DIY_TEXT_COLOR_DARK: &str = "#17191C";
const DIY_BACKGROUND_VARIABLE: &str = "--ccp-theme-art";
const DIY_BACKGROUND_MAX_BYTES: u64 = 8 * 1024 * 1024;
const DIY_BACKGROUND_MAX_PIXELS: u64 = 100_000_000;
const DIY_BACKGROUND_PREVIEW_MAX_WIDTH: u32 = 1280;
const DIY_BACKGROUND_PREVIEW_MAX_HEIGHT: u32 = 720;
const DIY_PREVIEW_WIDTH: u32 = 960;
const DIY_PREVIEW_HEIGHT: u32 = 600;
const DIY_PREVIEW_SIDEBAR_WIDTH: u32 = 176;
const DIY_PREVIEW_HERO_X: u32 = 328;
const DIY_PREVIEW_HERO_Y: u32 = 85;
const DIY_PREVIEW_HERO_WIDTH: u32 = 480;
const DIY_PREVIEW_HERO_HEIGHT: u32 = 184;
const DIY_PREVIEW_BANNER_X: u32 = 255;
const DIY_PREVIEW_BANNER_Y: u32 = 85;
const DIY_PREVIEW_BANNER_WIDTH: u32 = 627;
const DIY_PREVIEW_BANNER_HEIGHT: u32 = 144;
const DIY_PREVIEW_COMPOSER_X: u32 = 231;
const DIY_PREVIEW_COMPOSER_Y: u32 = 444;
const DIY_PREVIEW_COMPOSER_WIDTH: u32 = 674;
const DIY_PREVIEW_COMPOSER_HEIGHT: u32 = 136;
const DIY_PREVIEW_VISUAL_RADIUS: u32 = 8;
const DIY_PREVIEW_SHADOW_OFFSET_Y: u32 = 13;
const DIY_PREVIEW_SHADOW_SIGMA: f32 = 16.0;
const DIY_PREVIEW_SHADOW_PADDING: u32 = 48;
const DIY_PREVIEW_SHADOW_ALPHA: u8 = 46;
const OFFICIAL_THEME_RAW_BASE_URL: &str = "https://raw.githubusercontent.com/DamonZS/Claude-Codex-Pro-Tool/63ef4da6fbc22832553bab126c93e56aea2a91a6/Theme";
const DREAM_SKIN_HOME_LAYOUT_COMPAT_MARKER: &str =
    "/* CCP current Codex home-layout compatibility. */";
const LEGACY_DREAM_SKIN_HOME_LAYOUT_ANCHOR: &str = r#":is([data-feature="game-source"], [data-testid="home-icon"])
) > div:first-child"#;
const CURRENT_DREAM_SKIN_HOME_LAYOUT_ANCHOR: &str = r#":is([data-feature="game-source"], [data-testid="home-icon"])
) > div:has([data-feature="game-source"])"#;
const LIGHT_THEME_RUNTIME_COMPAT_CSS: &str = r#"
/* CCP light-theme runtime compatibility for current Codex color tokens. */
:root[data-ccp-theme-shell="light"] {
  --color-text-foreground: var(--ccp-theme-text, #241d1f) !important;
  --color-text-foreground-secondary: var(--ccp-theme-muted, #716367) !important;
  --color-text-accent: var(--ccp-theme-accent, #d94d5c) !important;
  --color-background-surface: var(--ccp-theme-background, #f8f4f5) !important;
  --color-background-panel: var(--ccp-theme-panel, #ffffff) !important;
  --color-background-control: var(--ccp-theme-panel, #ffffff) !important;
  --color-background-elevated-primary: var(--ccp-theme-panel-alt, #fff7f8) !important;
  --color-border: var(--ccp-theme-border, rgba(190, 112, 121, .28)) !important;
  --color-border-focus: var(--ccp-theme-accent, #d94d5c) !important;
  --color-token-foreground: var(--ccp-theme-text, #241d1f) !important;
  --color-token-text-primary: var(--ccp-theme-text, #241d1f) !important;
  --color-token-text-secondary: var(--ccp-theme-muted, #716367) !important;
  --color-token-bg-primary: var(--ccp-theme-panel, #ffffff) !important;
  --color-token-bg-secondary: var(--ccp-theme-panel-alt, #fff7f8) !important;
  --color-token-input-background: var(--ccp-theme-panel, #ffffff) !important;
  --color-token-input-foreground: var(--ccp-theme-text, #241d1f) !important;
  --color-token-input-placeholder-foreground: var(--ccp-theme-muted, #716367) !important;
  --color-token-menu-background: var(--ccp-theme-panel, #ffffff) !important;
  --color-token-menu-foreground: var(--ccp-theme-text, #241d1f) !important;
  --color-token-list-hover-background: color-mix(in srgb, var(--ccp-theme-accent, #d94d5c) 12%, transparent) !important;
  --vscode-foreground: var(--ccp-theme-text, #241d1f) !important;
  --vscode-descriptionForeground: var(--ccp-theme-muted, #716367) !important;
  --vscode-menu-background: var(--ccp-theme-panel, #ffffff) !important;
  --vscode-menu-foreground: var(--ccp-theme-text, #241d1f) !important;
  --vscode-list-hoverBackground: color-mix(in srgb, var(--ccp-theme-accent, #d94d5c) 12%, transparent) !important;
  --vscode-input-background: var(--ccp-theme-panel, #ffffff) !important;
  --vscode-input-foreground: var(--ccp-theme-text, #241d1f) !important;
}

:root[data-ccp-theme-shell="light"] body,
:root[data-ccp-theme-shell="light"] #root {
  color: var(--ccp-theme-text, #241d1f) !important;
}

:root[data-ccp-theme-shell="light"] main:has(
  [role="main"] :is([data-feature="game-source"], [data-testid="home-icon"])
) {
  background:
    linear-gradient(90deg, color-mix(in srgb, var(--ccp-theme-background, #f8f4f5) 96%, transparent), color-mix(in srgb, var(--ccp-theme-panel-alt, #fff7f8) 84%, transparent)),
    var(--ccp-theme-art) right center / cover no-repeat !important;
  color: var(--ccp-theme-text, #241d1f) !important;
}
"#;

#[derive(Debug, Clone, Copy)]
struct OfficialThemeDefinition {
    id: &'static str,
    name: &'static str,
    archive_sha256: &'static str,
}

const OFFICIAL_THEMES: &[OfficialThemeDefinition] = &[
    OfficialThemeDefinition {
        id: "aurora-glass",
        name: "极光穹顶",
        archive_sha256: "728b688c960c1c816cc93600dd72200467048f50128cecf16a0afa0a14fa250c",
    },
    OfficialThemeDefinition {
        id: "clockwork-fox-spirit",
        name: "机关狐灵",
        archive_sha256: "8511cc157ac693dbfb1428b125da7edf70ff325541e0d68877c12d3672f52a42",
    },
    OfficialThemeDefinition {
        id: "codex-dream-skin-macos",
        name: "Codex Dream Skin - macOS",
        archive_sha256: "0199f4fff9073b2b8a3c40ffcb230b9a0e24d4bfd71dab7080169686bc8aaeb2",
    },
    OfficialThemeDefinition {
        id: "codex-dream-skin-windows",
        name: "Codex Dream Skin - Windows",
        archive_sha256: "424fdd72c08f57b9c06941aebebce89dfe5489347b75657e8558cf92ba307ac7",
    },
    OfficialThemeDefinition {
        id: "cyber-changan",
        name: "赛博长安",
        archive_sha256: "c79d90da6ce0da293035325a26efb9e431e0539c3a145f4ae6fa292d56ff6889",
    },
    OfficialThemeDefinition {
        id: "lotus-fire-nezha",
        name: "莲火哪吒",
        archive_sha256: "192af19b9b09c7d8deedbcaa37b38ec73d5c6a56eb5ce0c86e35c5a65a865589",
    },
    OfficialThemeDefinition {
        id: "obsidian-gold",
        name: "黑金环域",
        archive_sha256: "37715125572452fffcc6e9907931cb8366a425c363310392c5c22fbe643b6c38",
    },
    OfficialThemeDefinition {
        id: "verdant-sanctuary",
        name: "森光秘境",
        archive_sha256: "578f487b0e44b812f70ef841f587a94135a1df64003db3a9191370a1c9c377f3",
    },
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CodexThemeDiySettings {
    pub mode: String,
    pub accent_color: String,
    pub background_color: String,
    pub surface_color: String,
    pub text_color: String,
    pub glass_opacity: u8,
    pub blur_px: u8,
    pub radius_px: u8,
    pub font_scale_percent: u8,
    pub density: String,
    #[serde(default = "default_diy_image_layout")]
    pub image_layout: String,
    #[serde(default)]
    pub background_file_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CodexThemeDiyInput {
    #[serde(default)]
    pub theme_id: Option<String>,
    #[serde(default)]
    pub expected_integrity_sha256: Option<String>,
    pub name: String,
    #[serde(default = "default_diy_author")]
    pub author: String,
    #[serde(default)]
    pub description: String,
    pub settings: CodexThemeDiySettings,
    #[serde(default)]
    pub background_path: Option<String>,
    #[serde(default)]
    pub remove_background: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CodexThemeDiyAutomaticPalette {
    pub mode: String,
    pub accent_color: String,
    pub background_color: String,
    pub surface_color: String,
    pub text_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CodexThemeDiyBackgroundPreview {
    pub file_name: String,
    pub data_uri: String,
    pub automatic_palette: CodexThemeDiyAutomaticPalette,
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diy: Option<CodexThemeDiySettings>,
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
    pub diy: Option<CodexThemeDiySettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodexThemeList {
    pub themes: Vec<CodexThemeSummary>,
    pub official_themes: Vec<CodexOfficialTheme>,
    pub current_theme_id: String,
    pub generation: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodexOfficialTheme {
    pub id: String,
    pub name: String,
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
pub struct CodexThemeManagerBackground {
    pub theme_id: String,
    pub generation: u64,
    pub data_uri: Option<String>,
    pub source_variable: Option<String>,
    pub is_default: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub mime_type: Option<String>,
    pub user_override: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodexManagerBackgroundItem {
    pub id: String,
    pub file_name: String,
    pub preview_data_uri: String,
    pub width: u32,
    pub height: u32,
    pub mime_type: String,
    pub updated_at: u64,
    pub current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodexManagerBackgroundLibrary {
    pub items: Vec<CodexManagerBackgroundItem>,
    pub current_background_id: Option<String>,
    pub generation: u64,
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
struct StoredManagerBackground {
    #[serde(default)]
    id: String,
    #[serde(default)]
    file_name: String,
    mime_type: String,
    width: u32,
    height: u32,
    sha256: String,
    updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ThemeState {
    schema_version: u32,
    current_theme_id: String,
    previous_theme_id: Option<String>,
    generation: u64,
    themes: Vec<InstalledTheme>,
    #[serde(default)]
    manager_background: Option<StoredManagerBackground>,
    #[serde(default)]
    manager_backgrounds: Vec<StoredManagerBackground>,
    #[serde(default)]
    current_manager_background_id: Option<String>,
}

impl Default for ThemeState {
    fn default() -> Self {
        Self {
            schema_version: 1,
            current_theme_id: DEFAULT_THEME_ID.to_string(),
            previous_theme_id: None,
            generation: 0,
            themes: Vec::new(),
            manager_background: None,
            manager_backgrounds: Vec::new(),
            current_manager_background_id: None,
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
    version_backup_dir: Option<PathBuf>,
    #[serde(default)]
    staged_version_backup_dir: Option<PathBuf>,
    #[serde(default)]
    finished_at: Option<u64>,
    #[serde(default)]
    result: Option<String>,
}

#[derive(Debug, Clone)]
enum ImportConstraint {
    Standard,
    ExpectedId(String),
    DiyCreate {
        theme_id: String,
    },
    DiyEdit {
        theme_id: String,
        expected_integrity_sha256: String,
    },
}

struct DiyBackground {
    bytes: Vec<u8>,
    image: image::DynamicImage,
    extension: &'static str,
    file_name: String,
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
            diy: None,
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
            official_themes: OFFICIAL_THEMES
                .iter()
                .map(|theme| CodexOfficialTheme {
                    id: theme.id.to_string(),
                    name: theme.name.to_string(),
                })
                .collect(),
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
        self.import_theme_checked(
            source.as_ref(),
            replace_existing,
            ImportConstraint::Standard,
        )
    }

    fn import_theme_checked(
        &self,
        source: &Path,
        replace_existing: bool,
        constraint: ImportConstraint,
    ) -> anyhow::Result<CodexThemeSummary> {
        if !source.exists() {
            bail!("主题来源不存在");
        }
        let _lock = self.acquire_lock()?;
        self.recover_pending_locked()?;
        self.import_theme_checked_locked(source, replace_existing, constraint)
    }

    fn import_theme_checked_locked(
        &self,
        source: &Path,
        replace_existing: bool,
        constraint: ImportConstraint,
    ) -> anyhow::Result<CodexThemeSummary> {
        if !source.exists() {
            bail!("主题来源不存在");
        }
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
        if let Err(error) = self.validate_import_constraint(&constraint, &manifest, &state) {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(error);
        }
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
            version_backup_dir: None,
            staged_version_backup_dir: None,
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

    fn validate_import_constraint(
        &self,
        constraint: &ImportConstraint,
        manifest: &CodexThemeManifest,
        state: &ThemeState,
    ) -> anyhow::Result<()> {
        match constraint {
            ImportConstraint::Standard => Ok(()),
            ImportConstraint::ExpectedId(expected) => {
                if manifest.id != *expected {
                    bail!("下载的主题 ID 与请求不一致，已拒绝安装");
                }
                Ok(())
            }
            ImportConstraint::DiyCreate { theme_id } => {
                if manifest.id != *theme_id || manifest.diy.is_none() {
                    bail!("DIY 主题暂存包与创建请求不一致");
                }
                if state
                    .themes
                    .iter()
                    .any(|theme| theme.manifest.id == *theme_id)
                    || self.library_dir().join(theme_id).exists()
                {
                    bail!("DIY 主题 ID 已存在，请重新创建");
                }
                Ok(())
            }
            ImportConstraint::DiyEdit {
                theme_id,
                expected_integrity_sha256,
            } => {
                if manifest.id != *theme_id || manifest.diy.is_none() {
                    bail!("DIY 主题暂存包与编辑请求不一致");
                }
                let existing = state
                    .themes
                    .iter()
                    .find(|theme| theme.manifest.id == *theme_id)
                    .context("要编辑的 DIY 主题不存在")?;
                if existing.manifest.diy.is_none() {
                    bail!("仅能编辑由 DIY 工作台创建的主题");
                }
                if existing.integrity_sha256 != *expected_integrity_sha256 {
                    bail!("DIY 主题在编辑期间已发生变化，请重新打开后再保存");
                }
                Ok(())
            }
        }
    }

    pub fn save_diy_theme(&self, input: CodexThemeDiyInput) -> anyhow::Result<CodexThemeSummary> {
        let CodexThemeDiyInput {
            theme_id,
            expected_integrity_sha256,
            name,
            author,
            description,
            mut settings,
            background_path,
            remove_background,
        } = input;
        let name = name.trim().to_string();
        let author = if author.trim().is_empty() {
            default_diy_author()
        } else {
            author.trim().to_string()
        };
        let description = description.trim().to_string();
        validate_text_field("主题名称", &name, 1, 80)?;
        validate_text_field("主题作者", &author, 1, 80)?;
        if description.chars().count() > 400 || description.chars().any(char::is_control) {
            bail!("主题描述无效或过长");
        }
        validate_diy_effect_settings(&settings)?;
        settings.background_file_name = None;

        let background_path = match background_path {
            Some(value) if value.trim().is_empty() => bail!("DIY 背景路径不能为空"),
            Some(value) => Some(value.trim().to_string()),
            None => None,
        };
        if remove_background && background_path.is_some() {
            bail!("不能同时选择新背景并移除背景");
        }
        let selected_background = background_path
            .as_deref()
            .map(Path::new)
            .map(validate_diy_background_source)
            .transpose()?;

        let _lock = self.acquire_lock()?;
        self.recover_pending_locked()?;
        let state = self.read_state()?;
        let normalized_theme_id = match theme_id {
            Some(value) if value.trim().is_empty() => bail!("DIY 主题 ID 不能为空"),
            Some(value) => Some(value.trim().to_string()),
            None => None,
        };
        let expected_integrity_sha256 = match expected_integrity_sha256 {
            Some(value) if value.trim().is_empty() => {
                bail!("DIY 主题完整性标识不能为空")
            }
            Some(value) => Some(value.trim().to_string()),
            None => None,
        };

        let (theme_id, version, replace_existing, constraint, retained_background) =
            if let Some(theme_id) = normalized_theme_id {
                validate_theme_id(&theme_id, false)?;
                if !theme_id.starts_with(DIY_THEME_ID_PREFIX) {
                    bail!("仅能编辑由 DIY 工作台创建的主题");
                }
                let existing = state
                    .themes
                    .iter()
                    .find(|theme| theme.manifest.id == theme_id)
                    .context("要编辑的 DIY 主题不存在")?;
                let existing_diy = existing
                    .manifest
                    .diy
                    .as_ref()
                    .context("仅能编辑由 DIY 工作台创建的主题")?;
                let expected_integrity_sha256 = expected_integrity_sha256
                    .as_deref()
                    .context("DIY 主题缺少打开编辑器时的完整性标识，请重新打开后再保存")?;
                if existing.integrity_sha256 != expected_integrity_sha256 {
                    bail!("DIY 主题在编辑期间已发生变化，请重新打开后再保存");
                }
                let retained_background = if selected_background.is_none() && !remove_background {
                    let mut retained = self.load_existing_diy_background(existing)?;
                    if let Some(background) = retained.as_mut()
                        && let Some(file_name) = existing_diy.background_file_name.as_ref()
                    {
                        background.file_name = file_name.clone();
                    }
                    retained
                } else {
                    None
                };
                (
                    theme_id.clone(),
                    next_diy_version(&existing.manifest.version)?,
                    true,
                    ImportConstraint::DiyEdit {
                        theme_id,
                        expected_integrity_sha256: expected_integrity_sha256.to_string(),
                    },
                    retained_background,
                )
            } else {
                if expected_integrity_sha256.is_some() {
                    bail!("新建 DIY 主题不能携带已有主题完整性标识");
                }
                let theme_id = self.unique_diy_theme_id(&state, &name)?;
                (
                    theme_id.clone(),
                    "1.0.0".to_string(),
                    false,
                    ImportConstraint::DiyCreate { theme_id },
                    None,
                )
            };

        let background = if remove_background {
            None
        } else {
            selected_background.or(retained_background)
        };
        let automatic_palette = automatic_diy_palette(background.as_ref());
        apply_automatic_diy_palette(&mut settings, &automatic_palette);
        settings.density = "comfortable".to_string();
        settings.font_scale_percent = 100;
        settings.background_file_name = background
            .as_ref()
            .map(|background| background.file_name.clone());
        normalize_diy_settings(&mut settings)?;

        let build_id = format!("{DIY_BUILD_PREFIX}{}", operation_id());
        let build_dir = self.staging_dir().join(&build_id);
        let package_dir = build_dir.join("package");
        let save_result = (|| -> anyhow::Result<CodexThemeSummary> {
            write_diy_package(
                &package_dir,
                &theme_id,
                &name,
                &version,
                &author,
                &description,
                &settings,
                background.as_ref(),
            )?;
            validate_package(&package_dir).context("生成的 DIY 主题未通过完整性校验")?;
            self.import_theme_checked_locked(&package_dir, replace_existing, constraint)
        })();
        let cleanup_result = remove_dir_all_with_retry(&build_dir);
        match (save_result, cleanup_result) {
            (Ok(summary), _) => Ok(summary),
            (Err(error), Ok(())) => Err(error),
            (Err(error), Err(cleanup_error)) => Err(error.context(format!(
                "DIY 主题保存失败，暂存目录清理也失败: {cleanup_error}"
            ))),
        }
    }

    pub fn preview_diy_background(
        &self,
        source: impl AsRef<Path>,
    ) -> anyhow::Result<CodexThemeDiyBackgroundPreview> {
        let background = validate_diy_background_source(source.as_ref())?;
        diy_background_preview(&background)
    }

    pub fn diy_theme_background_preview(
        &self,
        theme_id: &str,
    ) -> anyhow::Result<CodexThemeDiyBackgroundPreview> {
        let theme_id = theme_id.trim();
        validate_theme_id(theme_id, false)?;
        let _lock = self.acquire_lock()?;
        self.recover_pending_locked()?;
        let state = self.read_state()?;
        let theme = state
            .themes
            .iter()
            .find(|theme| theme.manifest.id == theme_id)
            .context("要预览的 DIY 主题不存在")?;
        let diy = theme
            .manifest
            .diy
            .as_ref()
            .context("仅能读取由 DIY 工作台创建的主题背景")?;
        let file_name = diy
            .background_file_name
            .as_ref()
            .context("该 DIY 主题没有可预览的背景")?;
        validate_background_file_name(file_name)?;
        let mut background = self
            .load_existing_diy_background(theme)?
            .context("该 DIY 主题没有可预览的背景")?;
        background.file_name = file_name.clone();
        diy_background_preview(&background)
    }

    fn unique_diy_theme_id(&self, state: &ThemeState, name: &str) -> anyhow::Result<String> {
        let slug = diy_id_slug(name);
        let seed = operation_id();
        for nonce in 0..1024_u16 {
            let digest = sha256_bytes(format!("{seed}:{name}:{nonce}").as_bytes());
            let suffix = digest
                .strip_prefix("sha256:")
                .unwrap_or(&digest)
                .chars()
                .take(12)
                .collect::<String>();
            let theme_id = format!("{DIY_THEME_ID_PREFIX}{slug}-{suffix}");
            if !state
                .themes
                .iter()
                .any(|theme| theme.manifest.id == theme_id)
                && !self.library_dir().join(&theme_id).exists()
            {
                return Ok(theme_id);
            }
        }
        bail!("无法生成唯一的 DIY 主题 ID，请重试")
    }

    fn load_existing_diy_background(
        &self,
        theme: &InstalledTheme,
    ) -> anyhow::Result<Option<DiyBackground>> {
        let Some(relative_path) = theme.manifest.asset_variables.get(DIY_BACKGROUND_VARIABLE)
        else {
            return Ok(None);
        };
        let package_root = self.library_dir().join(&theme.manifest.id);
        let path = checked_join(&package_root, relative_path)?;
        validate_diy_background_source(&path).map(Some)
    }

    pub async fn download_official_theme(
        &self,
        theme_id: &str,
    ) -> anyhow::Result<CodexThemeSummary> {
        let definition = OFFICIAL_THEMES
            .iter()
            .find(|theme| theme.id == theme_id)
            .context("该主题不在 CCP 官方下载目录中")?;

        {
            let _lock = self.acquire_lock()?;
            self.recover_pending_locked()?;
            let state = self.read_state()?;
            if state
                .themes
                .iter()
                .any(|theme| theme.manifest.id == theme_id)
            {
                bail!("该主题已安装，无需重复下载");
            }
        }

        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .user_agent(format!("CCP/{}", crate::version::VERSION))
            .build()
            .context("无法初始化主题下载客户端")?;
        let url = format!("{OFFICIAL_THEME_RAW_BASE_URL}/{}.zip", definition.id);
        let response = client
            .get(&url)
            .send()
            .await
            .context("无法连接 CCP 官方主题仓库")?;
        let status = response.status();
        if !status.is_success() {
            bail!("CCP 官方主题下载失败，GitHub 返回 HTTP {status}");
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_TOTAL_BYTES)
        {
            bail!("CCP 官方主题压缩包超过 32 MiB 限制");
        }

        let mut archive_bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("CCP 官方主题下载中断")?;
            if archive_bytes.len().saturating_add(chunk.len()) > MAX_TOTAL_BYTES as usize {
                bail!("CCP 官方主题压缩包超过 32 MiB 限制");
            }
            archive_bytes.extend_from_slice(&chunk);
        }
        if archive_bytes.is_empty() {
            bail!("CCP 官方主题下载结果为空");
        }
        let downloaded_sha256 = sha256_bytes(&archive_bytes);
        if downloaded_sha256 != format!("sha256:{}", definition.archive_sha256) {
            bail!("CCP 官方主题压缩包完整性校验失败");
        }

        let download_dir = self
            .staging_dir()
            .join(format!("download-{}", operation_id()));
        let archive_path = download_dir.join(format!("{}.zip", definition.id));
        let install_result = (|| -> anyhow::Result<CodexThemeSummary> {
            fs::create_dir_all(&download_dir).context("无法创建主题下载暂存目录")?;
            fs::write(&archive_path, archive_bytes).context("无法暂存下载的主题压缩包")?;
            self.import_theme_checked(
                &archive_path,
                false,
                ImportConstraint::ExpectedId(definition.id.to_string()),
            )
        })();
        let _ = fs::remove_dir_all(&download_dir);
        install_result.with_context(|| format!("无法安装官方主题“{}”", definition.name))
    }

    pub fn delete_theme(&self, theme_id: &str) -> anyhow::Result<CodexThemeOperationResult> {
        if theme_id == DEFAULT_THEME_ID {
            bail!("Codex 默认主题不能删除");
        }
        validate_theme_id(theme_id, false)?;

        let _lock = self.acquire_lock()?;
        self.recover_pending_locked()?;
        let mut state = self.read_state()?;
        if state.current_theme_id == theme_id {
            bail!("当前主题正在使用，请先恢复默认主题或切换到其他主题");
        }
        if !state
            .themes
            .iter()
            .any(|theme| theme.manifest.id == theme_id)
        {
            bail!("主题不存在或已经删除");
        }

        let target_dir = self.library_dir().join(theme_id);
        if !target_dir.is_dir() {
            bail!("主题文件缺失，已保留状态记录以便诊断");
        }
        let operation_id = operation_id();
        let staging_dir = self.staging_dir().join(&operation_id);
        let staged_theme_dir = staging_dir.join("theme");
        let version_backup_dir = self.backups_dir().join(theme_id);
        let staged_version_backup_dir = staging_dir.join("versions");
        fs::create_dir_all(&staging_dir).context("无法创建主题删除暂存目录")?;

        let mut journal = MutationJournal {
            operation_id: operation_id.clone(),
            operation_type: "delete".to_string(),
            theme_id: theme_id.to_string(),
            phase: "prepared".to_string(),
            started_at: now_secs(),
            state_before: state.clone(),
            staging_dir: Some(PathBuf::from("staging").join(&operation_id)),
            target_dir: Some(PathBuf::from("library").join(theme_id)),
            backup_dir: Some(PathBuf::from("staging").join(&operation_id).join("theme")),
            version_backup_dir: Some(PathBuf::from("backups").join(theme_id)),
            staged_version_backup_dir: Some(
                PathBuf::from("staging")
                    .join(&operation_id)
                    .join("versions"),
            ),
            finished_at: None,
            result: None,
        };
        if let Err(error) = self.write_journal(&journal) {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(error);
        }

        let transaction_result = (|| -> anyhow::Result<()> {
            fs::rename(&target_dir, &staged_theme_dir)
                .context("主题正在被占用，无法移入删除暂存区")?;
            journal.phase = "files-staged".to_string();
            self.write_journal(&journal)?;

            if version_backup_dir.exists() {
                fs::rename(&version_backup_dir, &staged_version_backup_dir)
                    .context("主题历史版本正在被占用，无法删除")?;
                journal.phase = "backups-staged".to_string();
                self.write_journal(&journal)?;
            }

            state.themes.retain(|theme| theme.manifest.id != theme_id);
            self.write_state(&state)?;
            journal.phase = "state-committed".to_string();
            self.write_journal(&journal)?;
            Ok(())
        })();

        if let Err(error) = transaction_result {
            if let Err(rollback_error) = self.rollback_journal(&journal) {
                return Err(error.context(format!("主题删除失败，回滚也失败: {rollback_error:#}")));
            }
            return Err(error);
        }

        if !self.journal_commit_is_valid(&journal)? {
            self.rollback_journal(&journal)?;
            bail!("主题删除提交后复核失败，已恢复原主题");
        }
        remove_dir_all_with_retry(&staging_dir).context("主题已从列表删除，但暂存文件清理失败")?;
        self.archive_journal(&journal, "committed")?;

        Ok(CodexThemeOperationResult {
            theme_id: theme_id.to_string(),
            persisted: true,
            runtime_applied: false,
            restart_required: false,
            rolled_back: false,
            generation: state.generation,
            message: "主题已删除。".to_string(),
        })
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

    pub fn active_manager_background(&self) -> anyhow::Result<CodexThemeManagerBackground> {
        let _lock = self.acquire_lock()?;
        self.recover_pending_locked()?;
        let mut state = self.read_state()?;
        self.migrate_manager_background_library_locked(&mut state)?;
        self.active_manager_background_for_state(&state)
    }

    pub fn manager_background_library(&self) -> anyhow::Result<CodexManagerBackgroundLibrary> {
        let _lock = self.acquire_lock()?;
        self.recover_pending_locked()?;
        let mut state = self.read_state()?;
        self.migrate_manager_background_library_locked(&mut state)?;
        self.manager_background_library_for_state(&state)
    }

    pub fn set_manager_background(
        &self,
        source: impl AsRef<Path>,
    ) -> anyhow::Result<CodexThemeManagerBackground> {
        let source = source.as_ref();
        let (bytes, mut background) = validate_manager_background_source(source)?;
        background.id = manager_background_id(&background.sha256);
        background.file_name = source
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("CCP 背景")
            .to_string();
        let _lock = self.acquire_lock()?;
        self.recover_pending_locked()?;
        let mut state = self.read_state()?;
        self.migrate_manager_background_library_locked(&mut state)?;
        let state_before = state.clone();
        if let Some(existing) = state
            .manager_backgrounds
            .iter()
            .find(|item| item.sha256 == background.sha256)
        {
            state.current_manager_background_id = Some(existing.id.clone());
            state.manager_background = None;
            state.generation = state.generation.saturating_add(1);
            self.write_state(&state)
                .context("无法切换到已保存的 CCP 背景")?;
            return self.active_manager_background_for_state(&state);
        }

        let target_path = self.manager_background_item_path(&background.id)?;
        fs::create_dir_all(self.manager_background_library_dir())
            .context("无法创建 CCP 背景图库目录")?;
        crate::settings::atomic_write(&target_path, &bytes).context("无法保存 CCP 背景到图库")?;
        let saved = fs::read(&target_path).context("无法复核保存的 CCP 背景")?;
        if saved != bytes {
            let _ = fs::remove_file(&target_path);
            bail!("CCP 背景保存复核失败");
        }
        state.manager_backgrounds.push(background.clone());
        state.current_manager_background_id = Some(background.id.clone());
        state.manager_background = None;
        state.generation = state.generation.saturating_add(1);
        if let Err(error) = self.write_state(&state) {
            let _ = fs::remove_file(&target_path);
            let _ = self.write_state(&state_before);
            return Err(error.context("保存 CCP 背景失败，已恢复上一状态"));
        }
        self.active_manager_background_for_state(&state)
    }

    pub fn clear_manager_background(&self) -> anyhow::Result<CodexThemeManagerBackground> {
        let _lock = self.acquire_lock()?;
        self.recover_pending_locked()?;
        let mut state = self.read_state()?;
        self.migrate_manager_background_library_locked(&mut state)?;
        if state.current_manager_background_id.is_some() || state.manager_background.is_some() {
            state.current_manager_background_id = None;
            state.manager_background = None;
            state.generation = state.generation.saturating_add(1);
            self.write_state(&state)
                .context("无法清除管理工具背景状态")?;
        }
        self.active_manager_background_for_state(&state)
    }

    pub fn apply_manager_background(
        &self,
        background_id: &str,
    ) -> anyhow::Result<CodexThemeManagerBackground> {
        validate_manager_background_id(background_id)?;
        let _lock = self.acquire_lock()?;
        self.recover_pending_locked()?;
        let mut state = self.read_state()?;
        self.migrate_manager_background_library_locked(&mut state)?;
        let background = state
            .manager_backgrounds
            .iter()
            .find(|item| item.id == background_id)
            .context("要应用的 CCP 背景不存在")?;
        let bytes = self.read_manager_background_item(background)?;
        if bytes.is_empty() {
            bail!("要应用的 CCP 背景文件为空");
        }
        if state.current_manager_background_id.as_deref() != Some(background_id) {
            state.current_manager_background_id = Some(background_id.to_string());
            state.manager_background = None;
            state.generation = state.generation.saturating_add(1);
            self.write_state(&state).context("无法应用 CCP 背景")?;
        }
        self.active_manager_background_for_state(&state)
    }

    pub fn delete_manager_background(
        &self,
        background_id: &str,
    ) -> anyhow::Result<CodexManagerBackgroundLibrary> {
        validate_manager_background_id(background_id)?;
        let _lock = self.acquire_lock()?;
        self.recover_pending_locked()?;
        let mut state = self.read_state()?;
        self.migrate_manager_background_library_locked(&mut state)?;
        if state.current_manager_background_id.as_deref() == Some(background_id) {
            bail!("正在使用的 CCP 背景不能删除，请先切换或恢复默认");
        }
        let index = state
            .manager_backgrounds
            .iter()
            .position(|item| item.id == background_id)
            .context("要删除的 CCP 背景不存在")?;
        let state_before = state.clone();
        let path = self.manager_background_item_path(background_id)?;
        let old_bytes = fs::read(&path).context("无法读取要删除的 CCP 背景")?;
        state.manager_backgrounds.remove(index);
        state.generation = state.generation.saturating_add(1);
        self.write_state(&state)
            .context("无法提交 CCP 背景删除状态")?;
        if let Err(error) = fs::remove_file(&path) {
            let _ = self.write_state(&state_before);
            let _ = crate::settings::atomic_write(&path, &old_bytes);
            return Err(error.into());
        }
        self.manager_background_library_for_state(&state)
    }

    fn active_manager_background_for_state(
        &self,
        state: &ThemeState,
    ) -> anyhow::Result<CodexThemeManagerBackground> {
        if let Some(background_id) = state.current_manager_background_id.as_deref()
            && let Some(background) = state
                .manager_backgrounds
                .iter()
                .find(|item| item.id == background_id)
        {
            let bytes = self.read_manager_background_item(background)?;
            return Ok(CodexThemeManagerBackground {
                theme_id: state.current_theme_id.clone(),
                generation: state.generation,
                data_uri: Some(data_uri(&background.mime_type, &bytes)),
                source_variable: Some(USER_MANAGER_BACKGROUND_SOURCE.to_string()),
                is_default: false,
                width: Some(background.width),
                height: Some(background.height),
                mime_type: Some(background.mime_type.clone()),
                user_override: true,
            });
        }

        Ok(CodexThemeManagerBackground {
            theme_id: state.current_theme_id.clone(),
            generation: state.generation,
            data_uri: None,
            source_variable: None,
            is_default: true,
            width: None,
            height: None,
            mime_type: None,
            user_override: false,
        })
    }

    fn manager_background_library_for_state(
        &self,
        state: &ThemeState,
    ) -> anyhow::Result<CodexManagerBackgroundLibrary> {
        let mut items = Vec::with_capacity(state.manager_backgrounds.len());
        for background in &state.manager_backgrounds {
            let bytes = self.read_manager_background_item(background)?;
            items.push(CodexManagerBackgroundItem {
                id: background.id.clone(),
                file_name: background.file_name.clone(),
                preview_data_uri: manager_background_preview_data_uri(&bytes)?,
                width: background.width,
                height: background.height,
                mime_type: background.mime_type.clone(),
                updated_at: background.updated_at,
                current: state.current_manager_background_id.as_deref()
                    == Some(background.id.as_str()),
            });
        }
        items.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        Ok(CodexManagerBackgroundLibrary {
            items,
            current_background_id: state.current_manager_background_id.clone(),
            generation: state.generation,
        })
    }

    fn migrate_manager_background_library_locked(
        &self,
        state: &mut ThemeState,
    ) -> anyhow::Result<()> {
        if !state.manager_backgrounds.is_empty() || state.manager_background.is_none() {
            return Ok(());
        }
        let mut background = state.manager_background.clone().unwrap();
        let legacy_path = self.manager_background_dir().join(MANAGER_BACKGROUND_FILE);
        let bytes = fs::read(&legacy_path).context("无法迁移现有 CCP 背景")?;
        if image_mime(&bytes) != Some(background.mime_type.as_str())
            || sha256_bytes(&bytes) != background.sha256
        {
            bail!("现有 CCP 背景完整性校验失败，未执行迁移");
        }
        background.id = manager_background_id(&background.sha256);
        background.file_name = "已迁移的 CCP 背景".to_string();
        let target = self.manager_background_item_path(&background.id)?;
        let state_before = state.clone();
        fs::create_dir_all(self.manager_background_library_dir())
            .context("无法创建 CCP 背景图库目录")?;
        crate::settings::atomic_write(&target, &bytes).context("无法迁移现有 CCP 背景到图库")?;
        state.manager_backgrounds.push(background.clone());
        state.current_manager_background_id = Some(background.id.clone());
        state.manager_background = None;
        if let Err(error) = self.write_state(state) {
            let _ = fs::remove_file(&target);
            *state = state_before;
            return Err(error.context("无法提交 CCP 背景图库迁移状态"));
        }
        let _ = fs::remove_file(legacy_path);
        Ok(())
    }

    fn read_manager_background_item(
        &self,
        background: &StoredManagerBackground,
    ) -> anyhow::Result<Vec<u8>> {
        validate_manager_background_id(&background.id)?;
        let bytes = fs::read(self.manager_background_item_path(&background.id)?)
            .context("CCP 背景图库文件不存在")?;
        if image_mime(&bytes) != Some(background.mime_type.as_str())
            || sha256_bytes(&bytes) != background.sha256
        {
            bail!("CCP 背景图库文件完整性校验失败");
        }
        Ok(bytes)
    }

    fn manager_background_item_path(&self, background_id: &str) -> anyhow::Result<PathBuf> {
        validate_manager_background_id(background_id)?;
        Ok(self
            .manager_background_dir()
            .join(MANAGER_BACKGROUND_LIBRARY_DIR)
            .join(format!("{background_id}.bin")))
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
            version_backup_dir: None,
            staged_version_backup_dir: None,
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
        let package_root = self.library_dir().join(&installed.manifest.id);
        let runtime = compile_runtime_resources(&package_root, &installed.manifest)?;
        let is_diy = installed.manifest.diy.is_some();
        let mut css = if let Some(settings) = installed.manifest.diy.as_ref() {
            render_diy_css(
                &installed.manifest.id,
                settings,
                runtime.asset_data_uris.contains_key("--ccp-theme-art"),
            )?
        } else {
            let style_path = checked_join(&package_root, &installed.manifest.entry_style)?;
            let css = normalize_css_line_endings(
                fs::read_to_string(&style_path).context("当前主题样式不可读取")?,
            );
            validate_css(&css)?;
            css
        };
        css = apply_official_theme_runtime_compat(&installed.manifest.id, css);
        if !is_diy
            && installed
                .manifest
                .root_attributes
                .attributes
                .get("data-ccp-theme-shell")
                .is_some_and(|shell| shell == "light")
        {
            css.push_str(LIGHT_THEME_RUNTIME_COMPAT_CSS);
        }
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
            diy: item.manifest.diy.clone(),
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
                        if journal.operation_type == "delete" {
                            remove_dir_all_with_retry(&staging)
                                .context("无法完成已提交主题的删除清理")?;
                        } else {
                            let _ = fs::remove_dir_all(staging);
                        }
                    }
                    self.archive_journal(&journal, "recovered-commit")?;
                } else {
                    self.rollback_journal(&journal)?;
                }
                continue;
            }
            self.rollback_journal(&journal)?;
        }
        self.cleanup_stale_diy_builds_locked()?;
        Ok(())
    }

    fn cleanup_stale_diy_builds_locked(&self) -> anyhow::Result<()> {
        for entry in fs::read_dir(self.staging_dir()).context("无法读取主题暂存目录")? {
            let entry = entry?;
            let name = entry.file_name();
            if name.to_string_lossy().starts_with(DIY_BUILD_PREFIX) && entry.file_type()?.is_dir() {
                remove_dir_all_with_retry(&entry.path())
                    .context("无法清理未完成的 DIY 暂存目录")?;
            }
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
        if let Some(staged_versions) =
            self.resolve_journal_path(journal.staged_version_backup_dir.as_ref())?
        {
            if staged_versions.exists() {
                let original_versions = self
                    .resolve_journal_path(journal.version_backup_dir.as_ref())?
                    .context("主题版本备份恢复路径缺失")?;
                if original_versions.exists() {
                    bail!("主题版本备份恢复目标已存在");
                }
                if let Some(parent) = original_versions.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::rename(staged_versions, original_versions).context("无法恢复主题历史版本")?;
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
        if journal.operation_type == "delete" {
            let target_absent = self
                .resolve_journal_path(journal.target_dir.as_ref())?
                .is_none_or(|target| !target.exists());
            return Ok(target_absent
                && state.current_theme_id != journal.theme_id
                && !state
                    .themes
                    .iter()
                    .any(|theme| theme.manifest.id == journal.theme_id));
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
            self.manager_background_dir(),
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

    fn manager_background_dir(&self) -> PathBuf {
        self.root.join("manager-background")
    }

    fn manager_background_library_dir(&self) -> PathBuf {
        self.manager_background_dir()
            .join(MANAGER_BACKGROUND_LIBRARY_DIR)
    }
}

fn apply_official_theme_runtime_compat(theme_id: &str, css: String) -> String {
    if !matches!(
        theme_id,
        "codex-dream-skin-macos" | "codex-dream-skin-windows"
    ) {
        return css;
    }

    let mut upgraded = css.replace(
        LEGACY_DREAM_SKIN_HOME_LAYOUT_ANCHOR,
        CURRENT_DREAM_SKIN_HOME_LAYOUT_ANCHOR,
    );
    upgraded.push('\n');
    upgraded.push_str(DREAM_SKIN_HOME_LAYOUT_COMPAT_MARKER);
    upgraded.push('\n');
    upgraded
}

fn default_diy_author() -> String {
    "CCP 用户".to_string()
}

fn default_diy_image_layout() -> String {
    DIY_IMAGE_LAYOUT_CARD.to_string()
}

fn default_diy_automatic_palette() -> CodexThemeDiyAutomaticPalette {
    CodexThemeDiyAutomaticPalette {
        mode: "dark".to_string(),
        accent_color: "#0A84FF".to_string(),
        background_color: "#111418".to_string(),
        surface_color: "#20252B".to_string(),
        text_color: "#F3F5F7".to_string(),
    }
}

fn automatic_diy_palette(background: Option<&DiyBackground>) -> CodexThemeDiyAutomaticPalette {
    let Some(background) = background else {
        return default_diy_automatic_palette();
    };
    let sample = background.image.thumbnail(64, 64).to_rgba8();
    let mut color_weight = 0_u64;
    let mut red_sum = 0_u64;
    let mut green_sum = 0_u64;
    let mut blue_sum = 0_u64;
    let mut accent_weight = 0_u64;
    let mut accent_red_sum = 0_u64;
    let mut accent_green_sum = 0_u64;
    let mut accent_blue_sum = 0_u64;

    for pixel in sample.pixels() {
        let [red, green, blue, alpha] = pixel.0;
        if alpha < 32 {
            continue;
        }
        let alpha = u64::from(alpha);
        color_weight = color_weight.saturating_add(alpha);
        red_sum = red_sum.saturating_add(u64::from(red) * alpha);
        green_sum = green_sum.saturating_add(u64::from(green) * alpha);
        blue_sum = blue_sum.saturating_add(u64::from(blue) * alpha);

        let maximum = red.max(green).max(blue);
        let minimum = red.min(green).min(blue);
        let chroma = maximum.saturating_sub(minimum);
        let luma = rgb_luma([red, green, blue]);
        if chroma < 32 || !(30..=225).contains(&luma) {
            continue;
        }
        let balance = 255_u64.saturating_sub(u64::from(luma.abs_diff(128)));
        let weight = alpha
            .saturating_mul(u64::from(chroma).pow(2))
            .saturating_mul(128 + balance);
        accent_weight = accent_weight.saturating_add(weight);
        accent_red_sum = accent_red_sum.saturating_add(u64::from(red) * weight);
        accent_green_sum = accent_green_sum.saturating_add(u64::from(green) * weight);
        accent_blue_sum = accent_blue_sum.saturating_add(u64::from(blue) * weight);
    }

    if color_weight == 0 {
        return default_diy_automatic_palette();
    }
    let average = [
        (red_sum / color_weight) as u8,
        (green_sum / color_weight) as u8,
        (blue_sum / color_weight) as u8,
    ];
    let mode = if rgb_luma(average) >= 154 {
        "light"
    } else {
        "dark"
    };
    let mut accent = if accent_weight == 0 {
        [10, 132, 255]
    } else {
        [
            (accent_red_sum / accent_weight) as u8,
            (accent_green_sum / accent_weight) as u8,
            (accent_blue_sum / accent_weight) as u8,
        ]
    };
    let (background_color, surface_color, text_color, accent_target) = if mode == "light" {
        (
            mix_rgb(average, [247, 248, 250], 55),
            mix_rgb(average, [255, 255, 255], 68),
            [23, 25, 28],
            [12, 78, 158],
        )
    } else {
        (
            mix_rgb(average, [16, 19, 23], 55),
            mix_rgb(average, [31, 36, 42], 62),
            [243, 245, 247],
            [102, 184, 255],
        )
    };
    for _ in 0..8 {
        if contrast_ratio(accent, surface_color) >= 3.0 {
            break;
        }
        accent = mix_rgb(accent, accent_target, 22);
    }

    CodexThemeDiyAutomaticPalette {
        mode: mode.to_string(),
        accent_color: rgb_hex(accent),
        background_color: rgb_hex(background_color),
        surface_color: rgb_hex(surface_color),
        text_color: rgb_hex(text_color),
    }
}

fn apply_automatic_diy_palette(
    settings: &mut CodexThemeDiySettings,
    palette: &CodexThemeDiyAutomaticPalette,
) {
    settings.mode.clone_from(&palette.mode);
    settings.accent_color.clone_from(&palette.accent_color);
    settings
        .background_color
        .clone_from(&palette.background_color);
    settings.surface_color.clone_from(&palette.surface_color);
    settings.text_color.clone_from(&palette.text_color);
}

fn rgb_luma([red, green, blue]: [u8; 3]) -> u8 {
    ((u32::from(red) * 2_126 + u32::from(green) * 7_152 + u32::from(blue) * 722) / 10_000) as u8
}

fn mix_rgb(source: [u8; 3], target: [u8; 3], target_percent: u8) -> [u8; 3] {
    let target_percent = u16::from(target_percent.min(100));
    let source_percent = 100_u16.saturating_sub(target_percent);
    [
        ((u16::from(source[0]) * source_percent + u16::from(target[0]) * target_percent) / 100)
            as u8,
        ((u16::from(source[1]) * source_percent + u16::from(target[1]) * target_percent) / 100)
            as u8,
        ((u16::from(source[2]) * source_percent + u16::from(target[2]) * target_percent) / 100)
            as u8,
    ]
}

fn rgb_hex([red, green, blue]: [u8; 3]) -> String {
    format!("#{red:02X}{green:02X}{blue:02X}")
}

fn contrast_ratio(left: [u8; 3], right: [u8; 3]) -> f32 {
    let left = relative_luminance(left);
    let right = relative_luminance(right);
    (left.max(right) + 0.05) / (left.min(right) + 0.05)
}

fn relative_luminance(color: [u8; 3]) -> f32 {
    let channel = |value: u8| {
        let value = f32::from(value) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color[0]) + 0.7152 * channel(color[1]) + 0.0722 * channel(color[2])
}

fn validate_diy_effect_settings(settings: &CodexThemeDiySettings) -> anyhow::Result<()> {
    if !(8..=90).contains(&settings.glass_opacity) {
        bail!("玻璃透光度必须在 10% 至 92% 之间");
    }
    if settings.blur_px > 48 {
        bail!("模糊强度必须在 0 至 48px 之间");
    }
    if settings.radius_px > 16 {
        bail!("圆角必须在 0 至 16px 之间");
    }
    if !(90..=110).contains(&settings.font_scale_percent) {
        bail!("文字大小必须在 90% 至 110% 之间");
    }
    Ok(())
}

fn normalize_diy_settings(settings: &mut CodexThemeDiySettings) -> anyhow::Result<()> {
    settings.mode = settings.mode.trim().to_ascii_lowercase();
    settings.density = settings.density.trim().to_ascii_lowercase();
    settings.image_layout = settings.image_layout.trim().to_ascii_lowercase();
    settings.accent_color = normalize_hex_color("主色", &settings.accent_color)?;
    settings.background_color = normalize_hex_color("背景色", &settings.background_color)?;
    settings.surface_color = normalize_hex_color("表面色", &settings.surface_color)?;
    settings.text_color = normalize_hex_color("文字色", &settings.text_color)?;
    if !matches!(
        settings.text_color.as_str(),
        DIY_TEXT_COLOR_LIGHT | DIY_TEXT_COLOR_DARK
    ) {
        bail!("DIY 主题文字颜色仅支持白字或黑字");
    }
    validate_diy_settings(settings)
}

fn validate_diy_settings(settings: &CodexThemeDiySettings) -> anyhow::Result<()> {
    if !matches!(settings.mode.as_str(), "dark" | "light") {
        bail!("DIY 主题外观模式仅支持 dark 或 light");
    }
    if !matches!(settings.density.as_str(), "compact" | "comfortable") {
        bail!("DIY 主题密度仅支持 compact 或 comfortable");
    }
    if !matches!(
        settings.image_layout.as_str(),
        DIY_IMAGE_LAYOUT_FULLSCREEN | DIY_IMAGE_LAYOUT_BANNER | DIY_IMAGE_LAYOUT_CARD
    ) {
        bail!("DIY 主题图片布局仅支持 fullscreen、banner 或 card");
    }
    for (label, color) in [
        ("主色", &settings.accent_color),
        ("背景色", &settings.background_color),
        ("表面色", &settings.surface_color),
        ("文字色", &settings.text_color),
    ] {
        validate_hex_color(label, color)?;
    }
    validate_diy_effect_settings(settings)?;
    if let Some(file_name) = settings.background_file_name.as_deref() {
        validate_background_file_name(file_name)?;
    }
    Ok(())
}

fn normalize_hex_color(label: &str, value: &str) -> anyhow::Result<String> {
    let value = value.trim();
    validate_hex_color(label, value)?;
    Ok(value.to_ascii_uppercase())
}

fn validate_hex_color(label: &str, value: &str) -> anyhow::Result<()> {
    if value.len() != 7
        || !value.starts_with('#')
        || !value.as_bytes()[1..]
            .iter()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("{label}必须使用完整的 #RRGGBB 格式");
    }
    Ok(())
}

fn validate_background_file_name(value: &str) -> anyhow::Result<()> {
    if value.is_empty()
        || value.chars().count() > 255
        || value.chars().any(char::is_control)
        || value.contains(['/', '\\', ':'])
        || matches!(value, "." | "..")
    {
        bail!("DIY 背景文件名无效");
    }
    Ok(())
}

fn next_diy_version(current: &str) -> anyhow::Result<String> {
    let parts = current.split('.').collect::<Vec<_>>();
    if parts.len() != 3 {
        bail!("DIY 主题版本记录损坏");
    }
    let major = parts[0].parse::<u32>().context("DIY 主题主版本号无效")?;
    let minor = parts[1].parse::<u32>().context("DIY 主题次版本号无效")?;
    let patch = parts[2]
        .parse::<u32>()
        .context("DIY 主题修订版本号无效")?
        .checked_add(1)
        .context("DIY 主题修订版本号已达到上限")?;
    Ok(format!("{major}.{minor}.{patch}"))
}

fn diy_id_slug(name: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for byte in name.bytes() {
        let normalized = if byte.is_ascii_alphanumeric() {
            Some(byte.to_ascii_lowercase() as char)
        } else if byte.is_ascii_whitespace() || matches!(byte, b'-' | b'_') {
            Some('-')
        } else {
            None
        };
        let Some(character) = normalized else {
            continue;
        };
        if character == '-' {
            if slug.is_empty() || previous_dash {
                continue;
            }
            previous_dash = true;
        } else {
            previous_dash = false;
        }
        slug.push(character);
        if slug.len() >= 20 {
            break;
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "theme".to_string()
    } else {
        slug.to_string()
    }
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

fn data_uri(mime_type: &str, bytes: &[u8]) -> String {
    format!(
        "data:{mime_type};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
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

fn remove_dir_all_with_retry(path: &Path) -> std::io::Result<()> {
    let mut last_error = None;
    for attempt in 0..3 {
        match fs::remove_dir_all(path) {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => last_error = Some(error),
        }
        if attempt < 2 {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    Err(last_error.expect("remove_dir_all retry must retain an error"))
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

fn validate_diy_background_source(path: &Path) -> anyhow::Result<DiyBackground> {
    let source = path.to_string_lossy().to_ascii_lowercase();
    if ["http://", "https://", "data:", "file://"]
        .iter()
        .any(|prefix| source.starts_with(prefix))
    {
        bail!("DIY 背景只允许用户选择的本地图片文件");
    }
    let metadata = fs::symlink_metadata(path).context("DIY 背景文件不存在")?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!("DIY 背景必须是本地普通图片文件");
    }
    if metadata.len() == 0 || metadata.len() > DIY_BACKGROUND_MAX_BYTES {
        bail!("DIY 背景必须小于 8 MiB");
    }
    let expected_mime = expected_image_mime(path).context("DIY 背景只支持 PNG、JPEG 或 WebP")?;
    let bytes = fs::read(path).context("无法读取 DIY 背景")?;
    let reader = image::ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .context("无法识别 DIY 背景格式")?;
    let format = reader.format().context("无法识别 DIY 背景格式")?;
    let (actual_mime, extension) = match format {
        image::ImageFormat::Png => ("image/png", "png"),
        image::ImageFormat::Jpeg => ("image/jpeg", "jpg"),
        image::ImageFormat::WebP => ("image/webp", "webp"),
        _ => bail!("DIY 背景只支持 PNG、JPEG 或 WebP"),
    };
    if actual_mime != expected_mime {
        bail!("DIY 背景扩展名与真实图片格式不一致");
    }
    let (width, height) = reader.into_dimensions().context("无法读取 DIY 背景尺寸")?;
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if width == 0 || height == 0 || pixels > DIY_BACKGROUND_MAX_PIXELS {
        bail!("DIY 背景像素总数不能超过 100,000,000");
    }
    let image = image::load_from_memory_with_format(&bytes, format)
        .context("DIY 背景文件已损坏或无法完整解码")?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .context("DIY 背景文件名不是有效文本")?
        .to_string();
    validate_background_file_name(&file_name)?;

    Ok(DiyBackground {
        bytes,
        image,
        extension,
        file_name,
    })
}

fn diy_background_preview(
    background: &DiyBackground,
) -> anyhow::Result<CodexThemeDiyBackgroundPreview> {
    let preview = background.image.thumbnail(
        DIY_BACKGROUND_PREVIEW_MAX_WIDTH,
        DIY_BACKGROUND_PREVIEW_MAX_HEIGHT,
    );
    let mut encoded = Cursor::new(Vec::new());
    preview
        .write_to(&mut encoded, image::ImageFormat::Png)
        .context("无法生成 DIY 背景预览")?;
    Ok(CodexThemeDiyBackgroundPreview {
        file_name: background.file_name.clone(),
        data_uri: data_uri("image/png", encoded.get_ref()),
        automatic_palette: automatic_diy_palette(Some(background)),
    })
}

fn validate_manager_background_source(
    path: &Path,
) -> anyhow::Result<(Vec<u8>, StoredManagerBackground)> {
    let metadata = fs::symlink_metadata(path).context("管理工具背景文件不存在")?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!("管理工具背景必须是本地图片文件");
    }
    if metadata.len() == 0 || metadata.len() > MAX_MANAGER_BACKGROUND_BYTES {
        bail!("管理工具背景必须小于 16 MB");
    }
    let expected_mime =
        expected_image_mime(path).context("管理工具背景只支持 PNG、JPEG 或 WebP")?;
    let bytes = fs::read(path).context("无法读取管理工具背景")?;
    let reader = image::ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .context("无法识别管理工具背景格式")?;
    let format = reader.format().context("无法识别管理工具背景格式")?;
    let actual_mime = match format {
        image::ImageFormat::Png => "image/png",
        image::ImageFormat::Jpeg => "image/jpeg",
        image::ImageFormat::WebP => "image/webp",
        _ => bail!("管理工具背景只支持 PNG、JPEG 或 WebP"),
    };
    if actual_mime != expected_mime {
        bail!("管理工具背景扩展名与真实图片格式不一致");
    }
    let (width, height) = reader
        .into_dimensions()
        .context("无法读取管理工具背景尺寸")?;
    let long_edge = width.max(height);
    let short_edge = width.min(height);
    if long_edge < MIN_MANAGER_BACKGROUND_WIDTH || short_edge < MIN_MANAGER_BACKGROUND_HEIGHT {
        bail!(
            "管理工具背景长边至少需要 {} 像素，短边至少需要 {} 像素",
            MIN_MANAGER_BACKGROUND_WIDTH,
            MIN_MANAGER_BACKGROUND_HEIGHT
        );
    }
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if pixels > MAX_MANAGER_BACKGROUND_PIXELS {
        bail!("管理工具背景像素尺寸过大");
    }
    image::load_from_memory_with_format(&bytes, format)
        .context("管理工具背景文件已损坏或无法完整解码")?;

    Ok((
        bytes.clone(),
        StoredManagerBackground {
            id: String::new(),
            file_name: String::new(),
            mime_type: actual_mime.to_string(),
            width,
            height,
            sha256: sha256_bytes(&bytes),
            updated_at: now_secs(),
        },
    ))
}

fn manager_background_id(sha256: &str) -> String {
    let digest = sha256.strip_prefix("sha256:").unwrap_or(sha256);
    format!("ccp-bg-{}", digest.chars().take(20).collect::<String>())
}

fn validate_manager_background_id(background_id: &str) -> anyhow::Result<()> {
    if !is_namespaced_identifier(background_id, "ccp-bg-") {
        bail!("CCP 背景 ID 无效");
    }
    Ok(())
}

fn manager_background_preview_data_uri(bytes: &[u8]) -> anyhow::Result<String> {
    let image = image::load_from_memory(bytes).context("无法解码 CCP 背景预览")?;
    let preview = image.thumbnail(720, 405);
    let mut encoded = Cursor::new(Vec::new());
    preview
        .write_to(&mut encoded, image::ImageFormat::Png)
        .context("无法生成 CCP 背景预览")?;
    Ok(data_uri("image/png", encoded.get_ref()))
}

#[allow(clippy::too_many_arguments)]
fn write_diy_package(
    root: &Path,
    theme_id: &str,
    name: &str,
    version: &str,
    author: &str,
    description: &str,
    settings: &CodexThemeDiySettings,
    background: Option<&DiyBackground>,
) -> anyhow::Result<()> {
    let assets_dir = root.join("assets");
    fs::create_dir_all(&assets_dir).context("无法创建 DIY 主题资源目录")?;

    let css = render_diy_css(theme_id, settings, background.is_some())?;
    let preview = render_diy_preview(settings, background)?;
    let style_path = "assets/theme.css".to_string();
    let preview_path = "assets/preview.png".to_string();
    crate::settings::atomic_write(&root.join(&style_path), css.as_bytes())
        .context("无法写入 DIY 主题样式")?;
    crate::settings::atomic_write(&root.join(&preview_path), &preview)
        .context("无法写入 DIY 主题预览")?;

    let mut assets = vec![style_path.clone(), preview_path.clone()];
    let mut asset_variables = BTreeMap::new();
    if let Some(background) = background {
        let relative_path = format!("assets/background.{}", background.extension);
        crate::settings::atomic_write(&root.join(&relative_path), &background.bytes)
            .context("无法写入 DIY 主题背景")?;
        assets.push(relative_path.clone());
        asset_variables.insert(DIY_BACKGROUND_VARIABLE.to_string(), relative_path);
    }

    let scope_class = diy_scope_class(theme_id)?;
    let manifest = CodexThemeManifest {
        format_version: 1,
        id: theme_id.to_string(),
        name: name.to_string(),
        version: version.to_string(),
        author: author.to_string(),
        description: description.to_string(),
        preview: preview_path,
        entry_style: style_path,
        assets,
        css_variables: BTreeMap::new(),
        root_attributes: CodexThemeRootAttributes {
            classes: vec![scope_class],
            attributes: BTreeMap::from([
                ("data-ccp-theme-shell".to_string(), settings.mode.clone()),
                ("data-ccp-theme-origin".to_string(), "ccp-diy".to_string()),
            ]),
        },
        asset_variables,
        diy: Some(settings.clone()),
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    crate::settings::atomic_write(&root.join("theme.json"), &manifest_bytes)
        .context("无法写入 DIY 主题 manifest")?;
    Ok(())
}

fn diy_scope_class(theme_id: &str) -> anyhow::Result<String> {
    if !theme_id.starts_with(DIY_THEME_ID_PREFIX) {
        bail!("DIY 主题 ID 命名空间无效");
    }
    let scope = format!("ccp-theme-{theme_id}");
    if !is_namespaced_identifier(&scope, "ccp-theme-") {
        bail!("DIY 主题作用域无效");
    }
    Ok(scope)
}

fn render_diy_css(
    theme_id: &str,
    settings: &CodexThemeDiySettings,
    has_background: bool,
) -> anyhow::Result<String> {
    validate_diy_settings(settings)?;
    let scope = diy_scope_class(theme_id)?;
    let border = rgba_css(&settings.text_color, 24)?;
    let muted_text = rgba_css(&settings.text_color, 74)?;
    let text_is_light = rgb_luma(parse_hex_color(&settings.text_color)?) >= 128;
    let text_shadow = if text_is_light {
        "rgb(0 0 0 / 0.68)"
    } else {
        "rgb(255 255 255 / 0.72)"
    };
    let strong_text_shadow = if text_is_light {
        "rgb(0 0 0 / 0.72)"
    } else {
        "rgb(255 255 255 / 0.78)"
    };
    let background_image = if has_background {
        "var(--ccp-theme-art, none)"
    } else {
        "none"
    };
    let fullscreen_background =
        has_background && settings.image_layout == DIY_IMAGE_LAYOUT_FULLSCREEN;
    let shell_surface = if fullscreen_background {
        rgba_css(&settings.background_color, 78)?
    } else {
        settings.background_color.clone()
    };
    let panel_surface = if fullscreen_background {
        rgba_css(&settings.background_color, 78)?
    } else {
        settings.surface_color.clone()
    };
    let content_surface = if fullscreen_background {
        rgba_css(&settings.surface_color, 96)?
    } else {
        settings.surface_color.clone()
    };
    let sidebar_surface = settings.surface_color.clone();
    let canvas_background = if fullscreen_background {
        background_image.to_string()
    } else {
        "none".to_string()
    };
    let has_hero_image = has_background
        && matches!(
            settings.image_layout.as_str(),
            DIY_IMAGE_LAYOUT_BANNER | DIY_IMAGE_LAYOUT_CARD
        );
    let (
        hero_visual_background_color,
        hero_visual_background_image,
        hero_visual_background_size,
        hero_visual_width,
        hero_visual_height,
        hero_visual_border,
        hero_visual_color,
        hero_content_opacity,
        hero_visual_radius,
        hero_visual_shadow,
    ) = if !has_hero_image {
        (
            "var(--ccp-theme-diy-content-surface)".to_string(),
            "none".to_string(),
            "auto",
            "58px",
            "58px",
            "var(--ccp-theme-diy-border)",
            "var(--ccp-theme-diy-accent)",
            "1",
            "50%",
            "none",
        )
    } else if settings.image_layout == DIY_IMAGE_LAYOUT_BANNER {
        (
            "var(--ccp-theme-diy-content-surface)".to_string(),
            background_image.to_string(),
            "cover",
            "min(88%, 470px)",
            "108px",
            "var(--ccp-theme-diy-border)",
            "transparent",
            "0",
            "8px",
            "0 10px 24px rgb(0 0 0 / 0.18)",
        )
    } else {
        (
            "var(--ccp-theme-diy-content-surface)".to_string(),
            background_image.to_string(),
            "contain",
            "min(76%, 360px)",
            "138px",
            "var(--ccp-theme-diy-border)",
            "transparent",
            "0",
            "8px",
            "0 10px 24px rgb(0 0 0 / 0.18)",
        )
    };
    Ok(format!(
        r#"/* Generated by CCP DIY Theme Builder. User CSS is never inserted. */
.{scope} {{
  color-scheme: {mode};
  --ccp-theme-diy-accent: {accent};
  --ccp-theme-diy-background: {background};
  --ccp-theme-diy-shell-surface: {shell_surface};
  --ccp-theme-diy-panel-surface: {panel_surface};
  --ccp-theme-diy-content-surface: {content_surface};
  --ccp-theme-diy-sidebar-surface: {sidebar_surface};
  --ccp-theme-diy-text: {text};
  --ccp-theme-diy-muted-text: {muted_text};
  --ccp-theme-diy-border: {border};
  --ccp-theme-diy-text-shadow: {text_shadow};
  --ccp-theme-diy-text-shadow-strong: {strong_text_shadow};
  --ccp-theme-diy-image-layout: {image_layout};
  background-color: var(--ccp-theme-diy-background) !important;
  background-image: {canvas_background} !important;
  background-position: center !important;
  background-repeat: no-repeat !important;
  background-size: cover !important;
  background-attachment: fixed !important;
  color: var(--ccp-theme-diy-text);
}}

.{scope} body,
.{scope} #root {{
  background-color: transparent !important;
  color: var(--ccp-theme-diy-text);
  font-synthesis: none;
  text-rendering: geometricPrecision;
  -webkit-font-smoothing: antialiased;
}}

.{scope} main,
.{scope} [data-testid="thread-view"],
.{scope} [data-testid="home"] {{
  color: var(--ccp-theme-diy-text);
}}

.{scope} main.main-surface :is(h1, h2, h3, h4, h5, h6, p, span, strong, em, small, label, li, dt, dd, blockquote, code, pre, table, thead, tbody, tr, th, td, time, a):not(:where(dialog, dialog *)),
.{scope} [data-testid="thread-view"] :is(h1, h2, h3, h4, h5, h6, p, span, strong, em, small, label, li, dt, dd, blockquote, code, pre, table, thead, tbody, tr, th, td, time, a):not(:where(dialog, dialog *)) {{
  color: var(--ccp-theme-diy-text) !important;
  -webkit-text-fill-color: currentColor !important;
}}

.{scope} [data-testid="thread-view"] :is(.prose, [class*="markdown"], [class*="message-content"]) *:not(:where(dialog, dialog *)) {{
  color: var(--ccp-theme-diy-text) !important;
  -webkit-text-fill-color: currentColor !important;
}}

.{scope} main,
.{scope} main.main-surface {{
  background: var(--ccp-theme-diy-shell-surface) !important;
  border-color: var(--ccp-theme-diy-border) !important;
  backdrop-filter: none !important;
}}

.{scope} main.main-surface > header.app-header-tint {{
  background: var(--ccp-theme-diy-panel-surface) !important;
  border-color: var(--ccp-theme-diy-border) !important;
  backdrop-filter: none !important;
}}

.{scope} main.main-surface > header.app-header-tint :is(div, section),
.{scope} [data-testid="top-bar"] :is(div, section) {{
  background-color: transparent !important;
  background-image: none !important;
  box-shadow: none !important;
}}

.{scope} nav,
.{scope} aside,
.{scope} [data-testid="left-sidebar"],
.{scope} [data-testid="top-bar"] {{
  background: var(--ccp-theme-diy-panel-surface);
  border-color: var(--ccp-theme-diy-border);
  backdrop-filter: none;
}}

.{scope} aside.app-shell-left-panel {{
  background: var(--ccp-theme-diy-sidebar-surface) !important;
  color: var(--ccp-theme-diy-text) !important;
  backdrop-filter: none !important;
  -webkit-backdrop-filter: none !important;
}}

.{scope} aside.app-shell-left-panel :is(a, button, div, span, p, strong, small, h1, h2, h3, li),
.{scope} [data-testid="left-sidebar"] :is(a, button, div, span, p, strong, small, h1, h2, h3, li) {{
  color: var(--ccp-theme-diy-text) !important;
  -webkit-text-fill-color: currentColor !important;
  text-shadow: 0 1px 2px var(--ccp-theme-diy-text-shadow) !important;
}}

.{scope} :is(main, [role="main"]):has(
  :is([data-feature="game-source"], [data-testid="home-icon"])
) {{
  background: transparent !important;
  color: var(--ccp-theme-diy-text) !important;
  overflow-x: hidden !important;
}}

.{scope} [data-testid="home-icon"] {{
  border: 1px solid var(--ccp-theme-diy-border) !important;
  width: {hero_visual_width} !important;
  height: {hero_visual_height} !important;
  min-width: {hero_visual_width} !important;
  min-height: {hero_visual_height} !important;
  border-color: {hero_visual_border} !important;
  border-radius: {hero_visual_radius} !important;
  background-color: {hero_visual_background_color} !important;
  background-image: {hero_visual_background_image} !important;
  background-position: center !important;
  background-repeat: no-repeat !important;
  background-size: {hero_visual_background_size} !important;
  color: {hero_visual_color} !important;
  box-shadow: {hero_visual_shadow} !important;
  opacity: 1 !important;
  filter: none !important;
  backdrop-filter: none !important;
  overflow: hidden !important;
}}

.{scope} [data-testid="home-icon"] > * {{
  opacity: {hero_content_opacity} !important;
}}

.{scope} [data-feature="game-source"] {{
  color: var(--ccp-theme-diy-text) !important;
  -webkit-text-fill-color: currentColor !important;
  font-weight: 450 !important;
  text-shadow: 0 1px 3px var(--ccp-theme-diy-text-shadow-strong) !important;
}}

.{scope} .group\/home-suggestions button,
.{scope} .composer-surface-chrome,
.{scope} [data-testid="composer"] {{
  display: block !important;
  width: 100% !important;
  min-height: 88px !important;
  border-color: var(--ccp-theme-diy-border) !important;
  background: transparent !important;
  color: var(--ccp-theme-diy-text) !important;
  backdrop-filter: none !important;
}}

.{scope} .group\/home-suggestions button * {{
  color: inherit !important;
  -webkit-text-fill-color: currentColor !important;
  opacity: 1 !important;
  font-weight: 550 !important;
  text-shadow: 0 1px 2px var(--ccp-theme-diy-text-shadow) !important;
}}

.{scope} .composer-surface-chrome :is(div, section, header, footer),
.{scope} [data-testid="composer"] :is(div, section, header, footer),
.{scope} [data-testid="prompt-composer"] :is(div, section, header, footer),
.{scope} form:has(textarea) :is(div, section, header, footer),
.{scope} form:has([contenteditable="true"]) :is(div, section, header, footer),
.{scope} .composer-surface-chrome .ProseMirror {{
  background-color: transparent !important;
  background-image: none !important;
  color: var(--ccp-theme-diy-text) !important;
  box-shadow: none !important;
}}
.{scope} :is(.composer-surface-chrome, [data-testid="composer"], [data-testid="prompt-composer"], form:has(textarea), form:has([contenteditable="true"])) :is(*, *::before, *::after) {{
  background-color: transparent !important;
  background-image: none !important;
}}
.{scope} :is(textarea, [contenteditable="true"]) {{
  display: block !important;
  visibility: visible !important;
  opacity: 1 !important;
  min-height: 32px !important;
  color: var(--ccp-theme-diy-text) !important;
  -webkit-text-fill-color: currentColor !important;
}}
"#,
        mode = settings.mode,
        accent = settings.accent_color,
        background = settings.background_color,
        text = settings.text_color,
        image_layout = settings.image_layout,
        text_shadow = text_shadow,
        strong_text_shadow = strong_text_shadow,
    ))
}

fn rgba_css(color: &str, opacity_percent: u8) -> anyhow::Result<String> {
    let [red, green, blue] = parse_hex_color(color)?;
    Ok(format!(
        "rgba({red}, {green}, {blue}, {:.2})",
        f32::from(opacity_percent) / 100.0
    ))
}

fn parse_hex_color(value: &str) -> anyhow::Result<[u8; 3]> {
    validate_hex_color("颜色", value)?;
    Ok([
        u8::from_str_radix(&value[1..3], 16)?,
        u8::from_str_radix(&value[3..5], 16)?,
        u8::from_str_radix(&value[5..7], 16)?,
    ])
}

fn render_diy_preview(
    settings: &CodexThemeDiySettings,
    background: Option<&DiyBackground>,
) -> anyhow::Result<Vec<u8>> {
    validate_diy_settings(settings)?;
    let [background_red, background_green, background_blue] =
        parse_hex_color(&settings.background_color)?;
    let mut canvas = image::RgbaImage::from_pixel(
        DIY_PREVIEW_WIDTH,
        DIY_PREVIEW_HEIGHT,
        image::Rgba([background_red, background_green, background_blue, 255]),
    );
    let [surface_red, surface_green, surface_blue] = parse_hex_color(&settings.surface_color)?;
    let [text_red, text_green, text_blue] = parse_hex_color(&settings.text_color)?;
    let [accent_red, accent_green, accent_blue] = parse_hex_color(&settings.accent_color)?;

    if let Some(background) =
        background.filter(|_| settings.image_layout == DIY_IMAGE_LAYOUT_FULLSCREEN)
    {
        let ambient = background.image.resize_to_fill(
            DIY_PREVIEW_WIDTH,
            DIY_PREVIEW_HEIGHT,
            image::imageops::FilterType::Triangle,
        );
        overlay_rounded_image(&mut canvas, &ambient.to_rgba8(), 0, 0, 0);
    }

    let fullscreen_background =
        background.is_some() && settings.image_layout == DIY_IMAGE_LAYOUT_FULLSCREEN;
    let workspace_surface = image::Rgba([
        background_red,
        background_green,
        background_blue,
        if fullscreen_background { 199 } else { 255 },
    ]);
    let sidebar_surface = image::Rgba([surface_red, surface_green, surface_blue, 255]);
    let content_surface = image::Rgba([
        surface_red,
        surface_green,
        surface_blue,
        if fullscreen_background { 245 } else { 255 },
    ]);
    let border = image::Rgba([text_red, text_green, text_blue, 62]);
    let text = image::Rgba([text_red, text_green, text_blue, 245]);
    let muted = image::Rgba([text_red, text_green, text_blue, 166]);
    let accent = image::Rgba([accent_red, accent_green, accent_blue, 255]);
    let radius = 8;
    let main_radius = 13;

    fill_rounded_rect(
        &mut canvas,
        0,
        0,
        DIY_PREVIEW_SIDEBAR_WIDTH,
        DIY_PREVIEW_HEIGHT,
        0,
        sidebar_surface,
    );
    fill_rounded_rect(
        &mut canvas,
        DIY_PREVIEW_SIDEBAR_WIDTH,
        0,
        DIY_PREVIEW_WIDTH - DIY_PREVIEW_SIDEBAR_WIDTH,
        DIY_PREVIEW_HEIGHT,
        main_radius,
        workspace_surface,
    );
    stroke_rounded_rect(
        &mut canvas,
        DIY_PREVIEW_SIDEBAR_WIDTH,
        0,
        DIY_PREVIEW_WIDTH - DIY_PREVIEW_SIDEBAR_WIDTH,
        DIY_PREVIEW_HEIGHT,
        main_radius,
        border,
    );

    // Current Codex sidebar: product title, new-task row, projects and recent sessions.
    fill_rounded_rect(&mut canvas, 16, 23, 68, 14, 5, text);
    fill_rounded_rect(&mut canvas, 82, 28, 9, 5, 2, muted);
    fill_rounded_rect(&mut canvas, 151, 24, 13, 13, 6, muted);
    fill_rounded_rect(&mut canvas, 9, 55, 158, 35, radius.min(9), content_surface);
    fill_rounded_rect(&mut canvas, 18, 65, 15, 15, 5, muted);
    fill_rounded_rect(&mut canvas, 42, 67, 82, 11, 4, text);

    for (index, y) in [112_u32, 216, 354].into_iter().enumerate() {
        if index == 1 {
            fill_rounded_rect(
                &mut canvas,
                9,
                y - 7,
                158,
                112,
                radius.min(9),
                content_surface,
            );
        }
        fill_rounded_rect(&mut canvas, 16, y, 17, 13, 4, text);
        fill_rounded_rect(&mut canvas, 42, y + 1, 105, 11, 4, text);
        let child_count = if index == 1 { 3 } else { 2 };
        for child in 0..child_count {
            fill_rounded_rect(
                &mut canvas,
                39,
                y + 29 + child * 24,
                108 - child * 9,
                9,
                4,
                muted,
            );
        }
    }

    fill_rounded_rect(&mut canvas, 0, 556, DIY_PREVIEW_SIDEBAR_WIDTH, 1, 0, border);
    fill_rounded_rect(&mut canvas, 16, 572, 15, 15, 7, muted);
    fill_rounded_rect(&mut canvas, 42, 574, 66, 11, 4, text);
    fill_rounded_rect(&mut canvas, 151, 573, 13, 13, 6, muted);

    // Small window controls in the otherwise quiet main canvas.
    fill_rounded_rect(&mut canvas, 907, 16, 13, 12, 3, muted);
    fill_rounded_rect(&mut canvas, 932, 16, 13, 12, 3, muted);

    let heading_y = if let Some(background) =
        background.filter(|_| settings.image_layout != DIY_IMAGE_LAYOUT_FULLSCREEN)
    {
        let (visual_x, visual_y, visual_width, visual_height, cover) =
            if settings.image_layout == DIY_IMAGE_LAYOUT_BANNER {
                (
                    DIY_PREVIEW_BANNER_X,
                    DIY_PREVIEW_BANNER_Y,
                    DIY_PREVIEW_BANNER_WIDTH,
                    DIY_PREVIEW_BANNER_HEIGHT,
                    true,
                )
            } else {
                (
                    DIY_PREVIEW_HERO_X,
                    DIY_PREVIEW_HERO_Y,
                    DIY_PREVIEW_HERO_WIDTH,
                    DIY_PREVIEW_HERO_HEIGHT,
                    false,
                )
            };
        draw_diy_preview_shadow(
            &mut canvas,
            visual_x,
            visual_y,
            visual_width,
            visual_height,
            DIY_PREVIEW_VISUAL_RADIUS,
        );
        let mut visual = image::RgbaImage::from_pixel(visual_width, visual_height, content_surface);
        let main = if cover {
            background.image.resize_to_fill(
                visual_width,
                visual_height,
                image::imageops::FilterType::Lanczos3,
            )
        } else {
            background.image.resize(
                visual_width,
                visual_height,
                image::imageops::FilterType::Lanczos3,
            )
        }
        .to_rgba8();
        let main_x = visual_width.saturating_sub(main.width()) / 2;
        let main_y = visual_height.saturating_sub(main.height()) / 2;
        overlay_rounded_image(&mut visual, &main, main_x, main_y, 0);
        overlay_rounded_image(
            &mut canvas,
            &visual,
            visual_x,
            visual_y,
            DIY_PREVIEW_VISUAL_RADIUS,
        );
        stroke_rounded_rect_outline(
            &mut canvas,
            visual_x,
            visual_y,
            visual_width,
            visual_height,
            DIY_PREVIEW_VISUAL_RADIUS,
            image::Rgba([text_red, text_green, text_blue, 68]),
        );
        visual_y + visual_height + 16
    } else {
        let hero_size = 77;
        let icon_size = 64;
        let hero_x = DIY_PREVIEW_SIDEBAR_WIDTH
            + (DIY_PREVIEW_WIDTH - DIY_PREVIEW_SIDEBAR_WIDTH - hero_size) / 2;
        let icon_x = hero_x + (hero_size - icon_size) / 2;
        let icon_y = DIY_PREVIEW_HERO_Y + (hero_size - icon_size) / 2;
        fill_rounded_rect(
            &mut canvas,
            icon_x,
            icon_y,
            icon_size,
            icon_size,
            icon_size / 2,
            sidebar_surface,
        );
        stroke_rounded_rect(
            &mut canvas,
            icon_x,
            icon_y,
            icon_size,
            icon_size,
            icon_size / 2,
            border,
        );
        fill_rounded_rect(&mut canvas, icon_x + 17, icon_y + 17, 20, 20, 8, accent);
        DIY_PREVIEW_HERO_Y + hero_size + 16
    };

    // Centered home heading below the visual.
    fill_rounded_rect(&mut canvas, 351, heading_y, 176, 16, 6, text);
    fill_rounded_rect(&mut canvas, 538, heading_y, 248, 16, 6, text);
    fill_rounded_rect(&mut canvas, 538, heading_y + 22, 248, 2, 1, muted);

    // Bottom composer with context strip and input surface.
    fill_rounded_rect(
        &mut canvas,
        DIY_PREVIEW_COMPOSER_X,
        DIY_PREVIEW_COMPOSER_Y,
        DIY_PREVIEW_COMPOSER_WIDTH,
        DIY_PREVIEW_COMPOSER_HEIGHT,
        main_radius,
        content_surface,
    );
    stroke_rounded_rect(
        &mut canvas,
        DIY_PREVIEW_COMPOSER_X,
        DIY_PREVIEW_COMPOSER_Y,
        DIY_PREVIEW_COMPOSER_WIDTH,
        DIY_PREVIEW_COMPOSER_HEIGHT,
        main_radius,
        border,
    );
    fill_rounded_rect(
        &mut canvas,
        DIY_PREVIEW_COMPOSER_X,
        DIY_PREVIEW_COMPOSER_Y + 45,
        DIY_PREVIEW_COMPOSER_WIDTH,
        1,
        0,
        border,
    );
    let composer_x = DIY_PREVIEW_COMPOSER_X;
    let composer_y = DIY_PREVIEW_COMPOSER_Y;
    fill_rounded_rect(
        &mut canvas,
        composer_x + 20,
        composer_y + 16,
        15,
        13,
        4,
        muted,
    );
    fill_rounded_rect(
        &mut canvas,
        composer_x + 44,
        composer_y + 18,
        128,
        10,
        4,
        text,
    );
    fill_rounded_rect(
        &mut canvas,
        composer_x + 194,
        composer_y + 16,
        15,
        13,
        4,
        muted,
    );
    fill_rounded_rect(
        &mut canvas,
        composer_x + 218,
        composer_y + 18,
        42,
        10,
        4,
        text,
    );
    fill_rounded_rect(
        &mut canvas,
        composer_x + 280,
        composer_y + 16,
        15,
        13,
        4,
        muted,
    );
    fill_rounded_rect(
        &mut canvas,
        composer_x + 304,
        composer_y + 18,
        38,
        10,
        4,
        text,
    );
    fill_rounded_rect(
        &mut canvas,
        composer_x + 22,
        composer_y + 68,
        112,
        11,
        4,
        muted,
    );
    fill_rounded_rect(
        &mut canvas,
        composer_x + 21,
        composer_y + 105,
        17,
        17,
        8,
        muted,
    );
    fill_rounded_rect(
        &mut canvas,
        composer_x + 51,
        composer_y + 109,
        66,
        10,
        4,
        accent,
    );
    fill_rounded_rect(
        &mut canvas,
        composer_x + 600,
        composer_y + 105,
        17,
        17,
        8,
        muted,
    );
    fill_rounded_rect(
        &mut canvas,
        composer_x + 625,
        composer_y + 99,
        27,
        27,
        14,
        text,
    );

    let mut cursor = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(canvas)
        .write_to(&mut cursor, image::ImageFormat::Png)
        .context("无法编码 DIY 主题 PNG 预览")?;
    Ok(cursor.into_inner())
}

fn fill_rounded_rect(
    image: &mut image::RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    radius: u32,
    color: image::Rgba<u8>,
) {
    if width == 0 || height == 0 {
        return;
    }
    let radius = radius.min(width / 2).min(height / 2);
    for target_y in y..y.saturating_add(height).min(image.height()) {
        for target_x in x..x.saturating_add(width).min(image.width()) {
            if rounded_rect_contains(target_x, target_y, x, y, width, height, radius) {
                blend_pixel(image.get_pixel_mut(target_x, target_y), color);
            }
        }
    }
}

fn draw_diy_preview_shadow(
    image: &mut image::RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    radius: u32,
) {
    if width == 0 || height == 0 {
        return;
    }
    let padding = DIY_PREVIEW_SHADOW_PADDING;
    let mut shadow = image::RgbaImage::new(
        width.saturating_add(padding.saturating_mul(2)),
        height.saturating_add(padding.saturating_mul(2)),
    );
    for target_y in padding..padding.saturating_add(height) {
        for target_x in padding..padding.saturating_add(width) {
            if rounded_rect_contains(target_x, target_y, padding, padding, width, height, radius) {
                shadow.put_pixel(
                    target_x,
                    target_y,
                    image::Rgba([0, 0, 0, DIY_PREVIEW_SHADOW_ALPHA]),
                );
            }
        }
    }
    let shadow = image::DynamicImage::ImageRgba8(shadow)
        .blur(DIY_PREVIEW_SHADOW_SIGMA)
        .to_rgba8();
    overlay_rounded_image(
        image,
        &shadow,
        x.saturating_sub(padding),
        y.saturating_add(DIY_PREVIEW_SHADOW_OFFSET_Y)
            .saturating_sub(padding),
        0,
    );
}

fn overlay_rounded_image(
    target: &mut image::RgbaImage,
    source: &image::RgbaImage,
    x: u32,
    y: u32,
    radius: u32,
) {
    let width = source.width().min(target.width().saturating_sub(x));
    let height = source.height().min(target.height().saturating_sub(y));
    if width == 0 || height == 0 {
        return;
    }
    let radius = radius.min(width / 2).min(height / 2);
    for source_y in 0..height {
        for source_x in 0..width {
            let target_x = x + source_x;
            let target_y = y + source_y;
            if rounded_rect_contains(target_x, target_y, x, y, width, height, radius) {
                blend_pixel(
                    target.get_pixel_mut(target_x, target_y),
                    *source.get_pixel(source_x, source_y),
                );
            }
        }
    }
}

fn stroke_rounded_rect(
    image: &mut image::RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    radius: u32,
    color: image::Rgba<u8>,
) {
    if width < 3 || height < 3 {
        return;
    }
    fill_rounded_rect(image, x, y, width, 1, radius, color);
    fill_rounded_rect(
        image,
        x,
        y.saturating_add(height - 1),
        width,
        1,
        radius,
        color,
    );
    fill_rounded_rect(image, x, y, 1, height, radius, color);
    fill_rounded_rect(
        image,
        x.saturating_add(width - 1),
        y,
        1,
        height,
        radius,
        color,
    );
}

fn stroke_rounded_rect_outline(
    image: &mut image::RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    radius: u32,
    color: image::Rgba<u8>,
) {
    if width < 3 || height < 3 {
        return;
    }
    let inner_x = x.saturating_add(1);
    let inner_y = y.saturating_add(1);
    let inner_width = width.saturating_sub(2);
    let inner_height = height.saturating_sub(2);
    let inner_radius = radius.saturating_sub(1);
    for target_y in y..y.saturating_add(height).min(image.height()) {
        for target_x in x..x.saturating_add(width).min(image.width()) {
            let inside_outer =
                rounded_rect_contains(target_x, target_y, x, y, width, height, radius);
            let inside_inner = target_x >= inner_x
                && target_y >= inner_y
                && target_x < inner_x.saturating_add(inner_width)
                && target_y < inner_y.saturating_add(inner_height)
                && rounded_rect_contains(
                    target_x,
                    target_y,
                    inner_x,
                    inner_y,
                    inner_width,
                    inner_height,
                    inner_radius,
                );
            if inside_outer && !inside_inner {
                blend_pixel(image.get_pixel_mut(target_x, target_y), color);
            }
        }
    }
}

fn rounded_rect_contains(
    target_x: u32,
    target_y: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    radius: u32,
) -> bool {
    if radius == 0 {
        return true;
    }
    let local_x = target_x.saturating_sub(x);
    let local_y = target_y.saturating_sub(y);
    let right = width.saturating_sub(1);
    let bottom = height.saturating_sub(1);
    let corner_x = if local_x < radius {
        radius
    } else if local_x > right.saturating_sub(radius) {
        right.saturating_sub(radius)
    } else {
        return true;
    };
    let corner_y = if local_y < radius {
        radius
    } else if local_y > bottom.saturating_sub(radius) {
        bottom.saturating_sub(radius)
    } else {
        return true;
    };
    let delta_x = i64::from(local_x) - i64::from(corner_x);
    let delta_y = i64::from(local_y) - i64::from(corner_y);
    delta_x * delta_x + delta_y * delta_y <= i64::from(radius) * i64::from(radius)
}

fn blend_pixel(destination: &mut image::Rgba<u8>, source: image::Rgba<u8>) {
    let alpha = u16::from(source[3]);
    let inverse = 255_u16.saturating_sub(alpha);
    for channel in 0..3 {
        destination[channel] = ((u16::from(source[channel]) * alpha
            + u16::from(destination[channel]) * inverse)
            / 255) as u8;
    }
    destination[3] = 255;
}

fn normalize_css_line_endings(css: String) -> String {
    if css.contains('\r') {
        css.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        css
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
    if manifest.description.chars().count() > 400
        || manifest.description.chars().any(char::is_control)
    {
        bail!("主题描述无效或过长");
    }
    if let Some(diy) = manifest.diy.as_ref() {
        if !manifest.id.starts_with(DIY_THEME_ID_PREFIX) {
            bail!("DIY 元数据只能用于 ccp-diy-* 主题");
        }
        validate_diy_settings(diy)?;
        let expected_scope = diy_scope_class(&manifest.id)?;
        if manifest.root_attributes.classes != [expected_scope]
            || manifest
                .root_attributes
                .attributes
                .get("data-ccp-theme-origin")
                .map(String::as_str)
                != Some("ccp-diy")
        {
            bail!("DIY 主题作用域或来源标记无效");
        }
        let background_declared = manifest
            .asset_variables
            .contains_key(DIY_BACKGROUND_VARIABLE);
        if background_declared != diy.background_file_name.is_some()
            || manifest.asset_variables.len() > usize::from(background_declared)
        {
            bail!("DIY 主题背景元数据与资源清单不一致");
        }
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
    let css = normalize_css_line_endings(
        fs::read_to_string(&style_path).context("主题 CSS 必须是 UTF-8 文本")?,
    );
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

    fn write_manager_background(path: &Path, width: u32, height: u32, color: [u8; 3]) {
        let image = image::RgbImage::from_pixel(width, height, image::Rgb(color));
        image.save(path).unwrap();
    }

    fn diy_input(name: &str) -> CodexThemeDiyInput {
        CodexThemeDiyInput {
            theme_id: None,
            expected_integrity_sha256: None,
            name: name.to_string(),
            author: "CCP Test".to_string(),
            description: "A no-code theme".to_string(),
            settings: CodexThemeDiySettings {
                mode: "dark".to_string(),
                accent_color: "#0A84FF".to_string(),
                background_color: "#121416".to_string(),
                surface_color: "#20242A".to_string(),
                text_color: DIY_TEXT_COLOR_LIGHT.to_string(),
                glass_opacity: 78,
                blur_px: 24,
                radius_px: 8,
                font_scale_percent: 100,
                density: "comfortable".to_string(),
                image_layout: DIY_IMAGE_LAYOUT_CARD.to_string(),
                background_file_name: None,
            },
            background_path: None,
            remove_background: false,
        }
    }

    fn assert_no_active_diy_artifacts(store: &CodexThemeStore) {
        assert_eq!(fs::read_dir(store.journal_dir()).unwrap().count(), 0);
        assert!(fs::read_dir(store.staging_dir()).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(DIY_BUILD_PREFIX)
        }));
    }

    #[test]
    fn diy_theme_create_generates_unique_safe_package_and_png_preview() {
        let temp = tempfile::tempdir().unwrap();
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();

        let first = store.save_diy_theme(diy_input("My Theme")).unwrap();
        let second = store.save_diy_theme(diy_input("My Theme")).unwrap();

        assert!(first.id.starts_with(DIY_THEME_ID_PREFIX));
        assert!(second.id.starts_with(DIY_THEME_ID_PREFIX));
        assert_ne!(first.id, second.id);
        assert!(!first.current);
        assert_eq!(first.version, "1.0.0");
        assert_eq!(first.diy.as_ref().unwrap().accent_color, "#0A84FF");
        assert_eq!(
            first.diy.as_ref().unwrap().image_layout,
            DIY_IMAGE_LAYOUT_CARD
        );
        let preview = first
            .preview_data_uri
            .as_deref()
            .unwrap()
            .strip_prefix("data:image/png;base64,")
            .unwrap();
        let preview = base64::engine::general_purpose::STANDARD
            .decode(preview)
            .unwrap();
        let image = image::load_from_memory_with_format(&preview, image::ImageFormat::Png).unwrap();
        assert_eq!(image.width(), DIY_PREVIEW_WIDTH);
        assert_eq!(image.height(), DIY_PREVIEW_HEIGHT);

        let package_root = store.library_dir().join(&first.id);
        let css = fs::read_to_string(package_root.join("assets/theme.css")).unwrap();
        assert!(css.contains(&format!(".ccp-theme-{}", first.id)));
        assert!(css.contains("User CSS is never inserted"));
        assert!(!css.to_ascii_lowercase().contains("@import"));
        assert!(!css.to_ascii_lowercase().contains("javascript:"));
        let manifest: CodexThemeManifest =
            serde_json::from_slice(&fs::read(package_root.join("theme.json")).unwrap()).unwrap();
        assert!(manifest.diy.is_some());
        assert_no_active_diy_artifacts(&store);
    }

    #[test]
    fn active_diy_payload_regenerates_css_from_manifest_settings() {
        let temp = tempfile::tempdir().unwrap();
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();
        let created = store
            .save_diy_theme(diy_input("Fresh Runtime CSS"))
            .unwrap();
        let style_path = store
            .library_dir()
            .join(&created.id)
            .join("assets/theme.css");
        fs::write(
            &style_path,
            ".legacy { backdrop-filter: blur(40px); } [role=\"dialog\"] { background: white; }",
        )
        .unwrap();

        store.apply_theme(&created.id).unwrap();
        let payload = store.active_theme_payload().unwrap();

        assert!(payload.css.contains("Generated by CCP DIY Theme Builder"));
        assert!(!payload.css.contains("blur(40px)"));
        assert!(!payload.css.contains("[role=\"dialog\"]"));
    }

    #[test]
    fn diy_text_color_follows_automatic_mode_and_uses_opposite_shadow() {
        let temp = tempfile::tempdir().unwrap();
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();
        let mut input = diy_input("Dark text");
        input.settings.text_color = DIY_TEXT_COLOR_DARK.to_string();

        let created = store.save_diy_theme(input).unwrap();
        assert_eq!(
            created.diy.as_ref().unwrap().text_color,
            DIY_TEXT_COLOR_LIGHT
        );
        let css = fs::read_to_string(
            store
                .library_dir()
                .join(&created.id)
                .join("assets/theme.css"),
        )
        .unwrap();
        assert!(css.contains("--ccp-theme-diy-text: #F3F5F7;"));
        assert!(css.contains("--ccp-theme-diy-text-shadow: rgb(0 0 0 / 0.68);"));
        assert!(css.contains("main.main-surface :is(h1, h2, h3"));
        assert!(css.contains(":not(:where(dialog, dialog *))"));

        let mut invalid = diy_input("Invalid text color");
        invalid.settings.text_color = "#123456".to_string();
        let normalized = store.save_diy_theme(invalid).unwrap();
        assert_eq!(
            normalized.diy.as_ref().unwrap().text_color,
            DIY_TEXT_COLOR_LIGHT
        );
    }

    #[test]
    fn diy_theme_edit_preserves_id_versions_package_and_refreshes_active_generation() {
        let temp = tempfile::tempdir().unwrap();
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();
        let created = store.save_diy_theme(diy_input("Editable")).unwrap();
        store.apply_theme(&created.id).unwrap();
        let generation_before = store.read_state().unwrap().generation;

        let mut edited_input = diy_input("Editable Updated");
        edited_input.theme_id = Some(created.id.clone());
        edited_input.expected_integrity_sha256 = created.integrity_sha256.clone();
        edited_input.settings.mode = "system".to_string();
        edited_input.settings.accent_color = "red; } @import x".to_string();
        edited_input.settings.density = "spacious".to_string();
        edited_input.settings.font_scale_percent = 92;
        edited_input.settings.image_layout = DIY_IMAGE_LAYOUT_BANNER.to_string();
        let edited = store.save_diy_theme(edited_input).unwrap();

        assert_eq!(edited.id, created.id);
        assert_eq!(edited.version, "1.0.1");
        assert!(edited.current);
        assert!(edited.previous_version_available);
        let edited_settings = edited.diy.as_ref().unwrap();
        assert_eq!(edited_settings.mode, "dark");
        assert_eq!(edited_settings.accent_color, "#0A84FF");
        assert_eq!(edited_settings.density, "comfortable");
        assert_eq!(edited_settings.font_scale_percent, 100);
        assert_eq!(edited_settings.image_layout, DIY_IMAGE_LAYOUT_BANNER);
        assert_eq!(
            store.read_state().unwrap().generation,
            generation_before + 1
        );
        assert!(store.backups_dir().join(&created.id).is_dir());

        let state_after_edit = store.read_state().unwrap();
        let mut stale_input = diy_input("Stale Edit");
        stale_input.theme_id = Some(created.id.clone());
        stale_input.expected_integrity_sha256 = created.integrity_sha256.clone();
        let error = store.save_diy_theme(stale_input).unwrap_err();
        assert!(error.to_string().contains("编辑期间已发生变化"));
        assert_eq!(store.read_state().unwrap(), state_after_edit);
        assert_no_active_diy_artifacts(&store);
    }

    #[test]
    fn diy_theme_background_can_be_retained_replaced_and_removed() {
        let temp = tempfile::tempdir().unwrap();
        let background_path = temp.path().join("personal-background.png");
        write_manager_background(&background_path, 640, 360, [32, 96, 160]);
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();

        let selected_preview = store.preview_diy_background(&background_path).unwrap();
        assert_eq!(selected_preview.file_name, "personal-background.png");
        assert!(
            selected_preview
                .data_uri
                .starts_with("data:image/png;base64,")
        );

        let mut create = diy_input("Background Theme");
        create.background_path = Some(background_path.to_string_lossy().into_owned());
        let created = store.save_diy_theme(create).unwrap();
        assert_eq!(
            created
                .diy
                .as_ref()
                .unwrap()
                .background_file_name
                .as_deref(),
            Some("personal-background.png")
        );
        let generated_preview = created
            .preview_data_uri
            .as_deref()
            .unwrap()
            .strip_prefix("data:image/png;base64,")
            .unwrap();
        let generated_preview = base64::engine::general_purpose::STANDARD
            .decode(generated_preview)
            .unwrap();
        let generated_preview =
            image::load_from_memory_with_format(&generated_preview, image::ImageFormat::Png)
                .unwrap()
                .to_rgba8();
        assert_eq!(
            &generated_preview
                .get_pixel(
                    DIY_PREVIEW_HERO_X + DIY_PREVIEW_HERO_WIDTH / 2,
                    DIY_PREVIEW_HERO_Y + DIY_PREVIEW_HERO_HEIGHT - 12,
                )
                .0[..3],
            &[32, 96, 160]
        );
        let generated_css = fs::read_to_string(
            store
                .library_dir()
                .join(&created.id)
                .join("assets/theme.css"),
        )
        .unwrap();
        assert!(generated_css.contains("background-image: var(--ccp-theme-art, none) !important;"));
        assert!(generated_css.contains("background-size: contain !important;"));
        assert!(!generated_css.contains("center / contain no-repeat"));
        let installed_preview = store.diy_theme_background_preview(&created.id).unwrap();
        assert_eq!(installed_preview.file_name, "personal-background.png");
        let installed_preview = base64::engine::general_purpose::STANDARD
            .decode(
                installed_preview
                    .data_uri
                    .strip_prefix("data:image/png;base64,")
                    .unwrap(),
            )
            .unwrap();
        let installed_preview =
            image::load_from_memory_with_format(&installed_preview, image::ImageFormat::Png)
                .unwrap();
        assert!(installed_preview.width() <= DIY_BACKGROUND_PREVIEW_MAX_WIDTH);
        assert!(installed_preview.height() <= DIY_BACKGROUND_PREVIEW_MAX_HEIGHT);

        let mut retain = diy_input("Background Theme Retained");
        retain.theme_id = Some(created.id.clone());
        retain.expected_integrity_sha256 = created.integrity_sha256.clone();
        retain.settings.background_color = "#111827".to_string();
        let retained = store.save_diy_theme(retain).unwrap();
        assert_eq!(
            retained
                .diy
                .as_ref()
                .unwrap()
                .background_file_name
                .as_deref(),
            Some("personal-background.png")
        );
        let retained_manifest = store
            .read_state()
            .unwrap()
            .themes
            .into_iter()
            .find(|theme| theme.manifest.id == created.id)
            .unwrap()
            .manifest;
        let retained_asset = retained_manifest
            .asset_variables
            .get(DIY_BACKGROUND_VARIABLE)
            .unwrap();
        let retained_asset_path = store.library_dir().join(&created.id).join(retained_asset);
        assert!(retained_asset_path.is_file());
        let retained_bytes = fs::read(&retained_asset_path).unwrap();

        let replacement_path = temp.path().join("replacement-background.png");
        write_manager_background(&replacement_path, 800, 450, [180, 40, 96]);
        let mut replace = diy_input("Background Theme Replaced");
        replace.theme_id = Some(created.id.clone());
        replace.expected_integrity_sha256 = retained.integrity_sha256.clone();
        replace.background_path = Some(replacement_path.to_string_lossy().into_owned());
        let replaced = store.save_diy_theme(replace).unwrap();
        assert_eq!(replaced.version, "1.0.2");
        assert_eq!(
            replaced
                .diy
                .as_ref()
                .unwrap()
                .background_file_name
                .as_deref(),
            Some("replacement-background.png")
        );
        let replaced_manifest = store
            .read_state()
            .unwrap()
            .themes
            .into_iter()
            .find(|theme| theme.manifest.id == created.id)
            .unwrap()
            .manifest;
        let replaced_asset = replaced_manifest
            .asset_variables
            .get(DIY_BACKGROUND_VARIABLE)
            .unwrap();
        assert_ne!(
            fs::read(store.library_dir().join(&created.id).join(replaced_asset)).unwrap(),
            retained_bytes
        );

        let mut remove = diy_input("Background Theme Without Image");
        remove.theme_id = Some(created.id.clone());
        remove.expected_integrity_sha256 = replaced.integrity_sha256.clone();
        remove.remove_background = true;
        let removed = store.save_diy_theme(remove).unwrap();
        assert_eq!(removed.version, "1.0.3");
        assert!(removed.diy.as_ref().unwrap().background_file_name.is_none());
        let removed_manifest = store
            .read_state()
            .unwrap()
            .themes
            .into_iter()
            .find(|theme| theme.manifest.id == created.id)
            .unwrap()
            .manifest;
        assert!(removed_manifest.asset_variables.is_empty());
        assert!(
            !fs::read_dir(store.library_dir().join(&created.id).join("assets"))
                .unwrap()
                .any(|entry| entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with("background."))
        );
        assert!(store.diy_theme_background_preview(&created.id).is_err());
        assert_no_active_diy_artifacts(&store);
    }

    #[test]
    fn diy_automatic_palette_is_deterministic_and_readable() {
        let temp = tempfile::tempdir().unwrap();
        let dark_path = temp.path().join("dark.png");
        let light_path = temp.path().join("light.png");
        let neutral_path = temp.path().join("neutral.png");
        write_manager_background(&dark_path, 640, 360, [18, 32, 48]);
        write_manager_background(&light_path, 640, 360, [228, 236, 244]);
        write_manager_background(&neutral_path, 640, 360, [128, 128, 128]);
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();

        let dark = store.preview_diy_background(&dark_path).unwrap();
        let dark_again = store.preview_diy_background(&dark_path).unwrap();
        let light = store.preview_diy_background(&light_path).unwrap();
        let neutral = store.preview_diy_background(&neutral_path).unwrap();

        assert_eq!(dark.automatic_palette, dark_again.automatic_palette);
        assert_eq!(dark.automatic_palette.mode, "dark");
        assert_eq!(light.automatic_palette.mode, "light");
        let neutral_accent = parse_hex_color(&neutral.automatic_palette.accent_color).unwrap();
        assert!(neutral_accent[2] > neutral_accent[0]);
        assert!(neutral_accent[2] > neutral_accent[1]);
        assert_eq!(dark.automatic_palette.text_color, DIY_TEXT_COLOR_LIGHT);
        assert_eq!(light.automatic_palette.text_color, DIY_TEXT_COLOR_DARK);
        for palette in [dark.automatic_palette, light.automatic_palette] {
            let accent = parse_hex_color(&palette.accent_color).unwrap();
            let surface = parse_hex_color(&palette.surface_color).unwrap();
            assert!(contrast_ratio(accent, surface) >= 3.0);
        }
    }

    #[test]
    fn diy_image_layout_defaults_for_legacy_settings() {
        let settings = diy_input("Legacy layout").settings;
        let mut value = serde_json::to_value(settings).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .remove("image_layout")
            .unwrap();

        let restored: CodexThemeDiySettings = serde_json::from_value(value).unwrap();
        assert_eq!(restored.image_layout, DIY_IMAGE_LAYOUT_CARD);
    }

    #[test]
    fn diy_image_layouts_generate_distinct_css_and_previews() {
        let temp = tempfile::tempdir().unwrap();
        let background_path = temp.path().join("layout-background.png");
        write_manager_background(&background_path, 640, 360, [32, 96, 160]);
        let background = validate_diy_background_source(&background_path).unwrap();
        let mut settings = diy_input("Layout modes").settings;

        settings.image_layout = DIY_IMAGE_LAYOUT_FULLSCREEN.to_string();
        let fullscreen_css = render_diy_css("ccp-diy-layout-fullscreen", &settings, true).unwrap();
        let fullscreen_preview = render_diy_preview(&settings, Some(&background)).unwrap();
        assert!(fullscreen_css.contains("--ccp-theme-diy-image-layout: fullscreen;"));
        assert!(
            fullscreen_css.contains("background-image: var(--ccp-theme-art, none) !important;")
        );
        assert!(fullscreen_css.contains("--ccp-theme-diy-sidebar-surface: #20242A;"));
        assert!(fullscreen_css.contains("--ccp-theme-diy-shell-surface: rgba(18, 20, 22, 0.78);"));
        assert!(fullscreen_css.contains("--ccp-theme-diy-panel-surface: rgba(18, 20, 22, 0.78);"));
        assert!(
            fullscreen_css.contains("--ccp-theme-diy-content-surface: rgba(32, 36, 42, 0.96);")
        );
        assert!(
            fullscreen_css.contains("background: var(--ccp-theme-diy-sidebar-surface) !important;")
        );
        assert!(fullscreen_css.contains("color: var(--ccp-theme-diy-text) !important;"));
        assert!(fullscreen_css.contains("opacity: 1 !important;"));
        assert!(fullscreen_css.contains("border-radius: 50% !important;"));
        assert!(fullscreen_css.contains("box-shadow: none !important;"));

        settings.image_layout = DIY_IMAGE_LAYOUT_BANNER.to_string();
        let banner_css = render_diy_css("ccp-diy-layout-banner", &settings, true).unwrap();
        let banner_preview = render_diy_preview(&settings, Some(&background)).unwrap();
        assert!(banner_css.contains("--ccp-theme-diy-image-layout: banner;"));
        assert!(banner_css.contains(
            "background-color: var(--ccp-theme-diy-content-surface) !important;\n  background-image: var(--ccp-theme-art, none) !important;"
        ));
        assert!(banner_css.contains("background-size: cover !important;"));
        assert!(!banner_css.contains("background: var(--ccp-theme-art, none) center /"));
        assert!(banner_css.contains("min(88%, 470px)"));
        assert!(banner_css.contains("height: 108px !important;"));
        assert!(banner_css.contains("border-radius: 8px !important;"));
        assert!(banner_css.contains("box-shadow: 0 10px 24px rgb(0 0 0 / 0.18) !important;"));
        assert!(banner_css.contains("opacity: 0 !important;"));
        assert!(banner_css.contains("background-image: none !important;"));

        settings.image_layout = DIY_IMAGE_LAYOUT_CARD.to_string();
        let card_css = render_diy_css("ccp-diy-layout-card", &settings, true).unwrap();
        let card_preview = render_diy_preview(&settings, Some(&background)).unwrap();
        assert!(card_css.contains("--ccp-theme-diy-image-layout: card;"));
        assert!(card_css.contains(
            "background-color: var(--ccp-theme-diy-content-surface) !important;\n  background-image: var(--ccp-theme-art, none) !important;"
        ));
        assert!(card_css.contains("background-size: contain !important;"));
        assert!(!card_css.contains("background: var(--ccp-theme-art, none) center /"));
        assert!(card_css.contains("min(76%, 360px)"));
        assert!(card_css.contains("height: 138px !important;"));
        assert!(card_css.contains("border-radius: 8px !important;"));
        assert!(card_css.contains("box-shadow: 0 10px 24px rgb(0 0 0 / 0.18) !important;"));
        assert!(card_css.contains("opacity: 0 !important;"));
        assert!(card_css.contains("background-image: none !important;"));

        for css in [&fullscreen_css, &banner_css, &card_css] {
            assert!(css.contains("main,\n."));
            assert!(css.contains("main.main-surface {"));
            assert!(css.contains("opacity: 1 !important;"));
            assert!(css.contains("-webkit-text-fill-color: currentColor !important;"));
            assert!(css.contains(":is(main, [role=\"main\"]):has("));
            assert!(!css.contains("flex: 0 0 auto !important;"));
            assert!(!css.contains("padding-top: 40px !important;"));
            assert!(!css.contains("align-items: flex-start !important;"));
            assert!(!css.contains("blur("));
            assert!(!css.contains("[role=\"dialog\"]"));
            assert!(!css.contains("[role=\"menu\"]"));
            assert!(!css.contains("linear-gradient(rgba("));
            assert!(css.contains("font-weight: 450 !important;"));
        }

        assert_eq!(
            (
                DIY_PREVIEW_BANNER_X,
                DIY_PREVIEW_BANNER_Y,
                DIY_PREVIEW_BANNER_WIDTH,
                DIY_PREVIEW_BANNER_HEIGHT,
            ),
            (255, 85, 627, 144),
        );
        assert_eq!(
            (
                DIY_PREVIEW_HERO_X,
                DIY_PREVIEW_HERO_Y,
                DIY_PREVIEW_HERO_WIDTH,
                DIY_PREVIEW_HERO_HEIGHT,
            ),
            (328, 85, 480, 184),
        );
        assert_eq!(DIY_PREVIEW_VISUAL_RADIUS, 8);

        let fullscreen_image =
            image::load_from_memory_with_format(&fullscreen_preview, image::ImageFormat::Png)
                .unwrap()
                .to_rgba8();
        assert_eq!(
            *fullscreen_image.get_pixel(20, 500),
            image::Rgba([32, 36, 42, 255]),
            "fullscreen preview sidebar must stay fully opaque",
        );
        let mut expected_workspace = image::Rgba([32, 96, 160, 255]);
        blend_pixel(&mut expected_workspace, image::Rgba([18, 20, 22, 199]));
        assert_eq!(
            *fullscreen_image.get_pixel(900, 350),
            expected_workspace,
            "fullscreen preview workspace must match the 78% live preview surface",
        );
        let mut expected_composer = expected_workspace;
        blend_pixel(&mut expected_composer, image::Rgba([32, 36, 42, 245]));
        assert_eq!(
            *fullscreen_image.get_pixel(895, 500),
            expected_composer,
            "fullscreen preview composer must match the 96% live preview surface",
        );

        let card_image =
            image::load_from_memory_with_format(&card_preview, image::ImageFormat::Png)
                .unwrap()
                .to_rgba8();
        assert_eq!(
            &card_image
                .get_pixel(
                    DIY_PREVIEW_HERO_X + DIY_PREVIEW_HERO_WIDTH / 2,
                    DIY_PREVIEW_HERO_Y + 2,
                )
                .0[..3],
            &[32, 96, 160],
            "card contain image must use the full preview visual height",
        );
        let shadow_pixel = card_image.get_pixel(
            DIY_PREVIEW_HERO_X + DIY_PREVIEW_HERO_WIDTH / 2,
            DIY_PREVIEW_HERO_Y + DIY_PREVIEW_HERO_HEIGHT + 8,
        );
        assert!(
            shadow_pixel[0] < 18 && shadow_pixel[1] < 20 && shadow_pixel[2] < 22,
            "card preview must include the live-preview shadow below the visual",
        );

        assert_ne!(fullscreen_preview, banner_preview);
        assert_ne!(banner_preview, card_preview);
        for preview in [fullscreen_preview, banner_preview, card_preview] {
            let image =
                image::load_from_memory_with_format(&preview, image::ImageFormat::Png).unwrap();
            assert_eq!(image.width(), DIY_PREVIEW_WIDTH);
            assert_eq!(image.height(), DIY_PREVIEW_HEIGHT);
        }
    }

    #[test]
    fn diy_runtime_payload_keeps_download_theme_compat_isolated() {
        let temp = tempfile::tempdir().unwrap();
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();
        let background_path = temp.path().join("diy-light-background.png");
        write_manager_background(&background_path, 640, 360, [224, 232, 240]);

        let mut input = diy_input("Light DIY compatibility isolation");
        input.settings.mode = "light".to_string();
        input.settings.background_color = "#F1F4F7".to_string();
        input.settings.surface_color = "#FFFFFF".to_string();
        input.settings.text_color = DIY_TEXT_COLOR_DARK.to_string();
        input.settings.image_layout = DIY_IMAGE_LAYOUT_BANNER.to_string();
        input.background_path = Some(background_path.to_string_lossy().into_owned());
        let diy = store.save_diy_theme(input).unwrap();
        store.apply_theme(&diy.id).unwrap();
        let diy_payload = store.active_theme_payload().unwrap();

        assert!(
            diy_payload
                .css
                .contains("Generated by CCP DIY Theme Builder")
        );
        assert!(
            !diy_payload
                .css
                .contains("CCP light-theme runtime compatibility")
        );
        assert!(
            !diy_payload
                .css
                .contains(":root[data-ccp-theme-shell=\"light\"] main:has(")
        );

        let imported_root = temp.path().join("imported-light");
        write_theme_with_runtime_resources(
            &imported_root,
            "imported-light",
            serde_json::json!({}),
            serde_json::json!({
                "classes": ["ccp-theme-imported-light"],
                "attributes": {
                    "data-ccp-theme-shell": "light"
                }
            }),
            serde_json::json!({}),
            &["assets/theme.css"],
        );
        store.import_theme(&imported_root).unwrap();
        store.apply_theme("imported-light").unwrap();
        let imported_payload = store.active_theme_payload().unwrap();

        assert!(
            imported_payload
                .css
                .starts_with(":root.ccp-theme-runtime { background-image: var(--ccp-theme-art); }")
        );
        assert!(
            imported_payload
                .css
                .contains("CCP light-theme runtime compatibility")
        );
        assert!(
            imported_payload
                .css
                .contains(":root[data-ccp-theme-shell=\"light\"] main:has(")
        );
    }

    #[test]
    fn diy_theme_rejects_invalid_structured_values_without_state_changes() {
        let temp = tempfile::tempdir().unwrap();
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();
        let state_before = store.read_state().unwrap();

        let mut invalid_inputs = Vec::new();
        let mut empty_name = diy_input("valid");
        empty_name.name = "  ".to_string();
        invalid_inputs.push(empty_name);
        let mut bad_opacity = diy_input("bad opacity");
        bad_opacity.settings.glass_opacity = 7;
        invalid_inputs.push(bad_opacity);
        let mut bad_blur = diy_input("bad blur");
        bad_blur.settings.blur_px = 49;
        invalid_inputs.push(bad_blur);
        let mut bad_radius = diy_input("bad radius");
        bad_radius.settings.radius_px = 17;
        invalid_inputs.push(bad_radius);
        let mut bad_image_layout = diy_input("bad image layout");
        bad_image_layout.settings.image_layout = "floating".to_string();
        invalid_inputs.push(bad_image_layout);
        let mut empty_id = diy_input("empty id");
        empty_id.theme_id = Some("  ".to_string());
        invalid_inputs.push(empty_id);
        let mut empty_background_path = diy_input("empty background path");
        empty_background_path.background_path = Some("  ".to_string());
        invalid_inputs.push(empty_background_path);

        for input in invalid_inputs {
            assert!(store.save_diy_theme(input).is_err());
            assert_eq!(store.read_state().unwrap(), state_before);
            assert_no_active_diy_artifacts(&store);
        }
    }

    #[test]
    fn diy_theme_rejects_non_diy_overwrite_and_unsafe_backgrounds() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("plain-theme");
        write_theme(
            &source,
            "ccp-diy-plain-import",
            ":root { --ccp-plain-theme: 1; }",
        );
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();
        let imported = store.import_theme(&source).unwrap();
        let state_before = store.read_state().unwrap();
        let css_before = fs::read_to_string(
            store
                .library_dir()
                .join(&imported.id)
                .join("assets/theme.css"),
        )
        .unwrap();
        let mut overwrite = diy_input("Attempted Overwrite");
        overwrite.theme_id = Some(imported.id.clone());
        let error = store.save_diy_theme(overwrite).unwrap_err();
        assert!(format!("{error:#}").contains("仅能编辑"));
        assert_eq!(store.read_state().unwrap(), state_before);
        assert_eq!(
            fs::read_to_string(
                store
                    .library_dir()
                    .join(&imported.id)
                    .join("assets/theme.css")
            )
            .unwrap(),
            css_before
        );

        let actual_png = temp.path().join("actual.png");
        let mismatched = temp.path().join("mismatched.jpg");
        write_manager_background(&actual_png, 320, 180, [10, 20, 30]);
        fs::copy(&actual_png, &mismatched).unwrap();
        let mut bad_background = diy_input("Bad Background");
        bad_background.background_path = Some(mismatched.to_string_lossy().into_owned());
        assert!(store.save_diy_theme(bad_background).is_err());

        let mut directory_background = diy_input("Directory Background");
        directory_background.background_path = Some(temp.path().to_string_lossy().into_owned());
        assert!(store.save_diy_theme(directory_background).is_err());
        assert_eq!(store.read_state().unwrap(), state_before);
        assert_no_active_diy_artifacts(&store);
    }

    #[test]
    fn stale_diy_build_directory_is_removed_during_recovery() {
        let temp = tempfile::tempdir().unwrap();
        let store_root = temp.path().join("store");
        let store = CodexThemeStore::open(&store_root).unwrap();
        let stale = store.staging_dir().join("diy-build-stale");
        fs::create_dir_all(&stale).unwrap();
        fs::write(stale.join("partial"), b"partial").unwrap();
        drop(store);

        CodexThemeStore::open(&store_root).unwrap();
        assert!(!stale.exists());
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
    fn codex_theme_assets_never_become_manager_backgrounds() {
        let temp = tempfile::tempdir().unwrap();
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();

        let default_background = store.active_manager_background().unwrap();
        assert!(default_background.is_default);
        assert!(default_background.data_uri.is_none());
        assert!(default_background.source_variable.is_none());

        let legacy_source = temp.path().join("legacy-source");
        write_theme_with_runtime_resources(
            &legacy_source,
            "legacy-background",
            serde_json::json!({}),
            serde_json::json!({}),
            serde_json::json!({"--ccp-theme-art": "assets/hero.png"}),
            &["assets/theme.css", "assets/hero.png"],
        );
        store.import_theme(&legacy_source).unwrap();
        store.apply_theme("legacy-background").unwrap();
        let legacy_background = store.active_manager_background().unwrap();
        assert!(legacy_background.data_uri.is_none());
        assert!(legacy_background.source_variable.is_none());
        assert!(legacy_background.is_default);
        assert!(!legacy_background.user_override);

        let dedicated_source = temp.path().join("dedicated-source");
        write_theme_with_runtime_resources(
            &dedicated_source,
            "dedicated-background",
            serde_json::json!({}),
            serde_json::json!({}),
            serde_json::json!({
                "--ccp-theme-art": "assets/hero.png",
                "--ccp-theme-manager-background": "assets/hero.png"
            }),
            &["assets/theme.css", "assets/hero.png"],
        );
        store.import_theme(&dedicated_source).unwrap();
        store.apply_theme("dedicated-background").unwrap();
        let dedicated_background = store.active_manager_background().unwrap();
        assert!(dedicated_background.data_uri.is_none());
        assert!(dedicated_background.source_variable.is_none());
        assert_eq!(dedicated_background.theme_id, "dedicated-background");
        assert!(dedicated_background.is_default);
        assert!(!dedicated_background.user_override);

        store.restore_default_theme().unwrap();
        assert!(
            store
                .active_manager_background()
                .unwrap()
                .data_uri
                .is_none()
        );
    }

    #[test]
    fn manager_background_library_deduplicates_switches_persists_and_deletes() {
        let temp = tempfile::tempdir().unwrap();
        let store_root = temp.path().join("store");
        let first_source = temp.path().join("first.png");
        let second_source = temp.path().join("second.png");
        write_manager_background(&first_source, 1920, 1080, [12, 34, 56]);
        write_manager_background(&second_source, 2048, 1152, [78, 90, 123]);
        let first_source_bytes = fs::read(&first_source).unwrap();

        let store = CodexThemeStore::open(&store_root).unwrap();
        let first = store.set_manager_background(&first_source).unwrap();
        assert!(first.user_override);
        assert_eq!(first.width, Some(1920));
        assert_eq!(first.height, Some(1080));
        assert_eq!(first.mime_type.as_deref(), Some("image/png"));
        assert_eq!(
            first.source_variable.as_deref(),
            Some(USER_MANAGER_BACKGROUND_SOURCE)
        );
        assert!(
            first
                .data_uri
                .unwrap()
                .starts_with("data:image/png;base64,")
        );
        assert_eq!(fs::read(&first_source).unwrap(), first_source_bytes);
        let first_library = store.manager_background_library().unwrap();
        assert_eq!(first_library.items.len(), 1);
        let first_id = first_library.current_background_id.clone().unwrap();
        assert!(
            first_library.items[0]
                .preview_data_uri
                .starts_with("data:image/png;base64,")
        );

        let second = store.set_manager_background(&second_source).unwrap();
        assert_eq!(second.width, Some(2048));
        let second_library = store.manager_background_library().unwrap();
        assert_eq!(second_library.items.len(), 2);
        let second_id = second_library.current_background_id.clone().unwrap();
        assert_ne!(first_id, second_id);

        store.set_manager_background(&first_source).unwrap();
        let deduplicated = store.manager_background_library().unwrap();
        assert_eq!(deduplicated.items.len(), 2);
        assert_eq!(
            deduplicated.current_background_id.as_deref(),
            Some(first_id.as_str())
        );

        store.apply_manager_background(&second_id).unwrap();
        let after_delete = store.delete_manager_background(&first_id).unwrap();
        assert_eq!(after_delete.items.len(), 1);
        assert_eq!(
            after_delete.current_background_id.as_deref(),
            Some(second_id.as_str())
        );
        assert!(store.delete_manager_background(&second_id).is_err());

        drop(store);
        let reopened = CodexThemeStore::open(&store_root).unwrap();
        assert!(reopened.active_manager_background().unwrap().user_override);
        let cleared = reopened.clear_manager_background().unwrap();
        assert!(!cleared.user_override);
        let retained = reopened.manager_background_library().unwrap();
        assert_eq!(retained.items.len(), 1);
        assert!(retained.current_background_id.is_none());
        reopened.apply_manager_background(&second_id).unwrap();
        assert_eq!(
            reopened
                .manager_background_library()
                .unwrap()
                .current_background_id
                .as_deref(),
            Some(second_id.as_str())
        );

        let serialized_state = serde_json::to_string(&reopened.read_state().unwrap()).unwrap();
        assert!(!serialized_state.contains(&temp.path().to_string_lossy().to_string()));
    }

    #[test]
    fn manager_background_accepts_landscape_dimensions_in_either_storage_orientation() {
        let temp = tempfile::tempdir().unwrap();
        let landscape = temp.path().join("landscape.jpg");
        let rotated_storage = temp.path().join("rotated-storage.jpg");
        let hd = temp.path().join("hd.jpg");
        write_manager_background(&landscape, 1920, 1200, [42, 96, 150]);
        write_manager_background(&rotated_storage, 1200, 1920, [42, 96, 150]);
        write_manager_background(&hd, 1280, 720, [42, 96, 150]);

        assert!(validate_manager_background_source(&landscape).is_ok());
        assert!(validate_manager_background_source(&rotated_storage).is_ok());
        assert!(validate_manager_background_source(&hd).is_ok());
    }

    #[test]
    fn legacy_manager_background_is_migrated_into_the_library() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("legacy.png");
        write_manager_background(&source, 1920, 1080, [24, 80, 136]);
        let (bytes, legacy_background) = validate_manager_background_source(&source).unwrap();
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();
        fs::create_dir_all(store.manager_background_dir()).unwrap();
        crate::settings::atomic_write(
            &store.manager_background_dir().join(MANAGER_BACKGROUND_FILE),
            &bytes,
        )
        .unwrap();
        let mut legacy_state = store.read_state().unwrap();
        legacy_state.manager_background = Some(legacy_background);
        store.write_state(&legacy_state).unwrap();

        let library = store.manager_background_library().unwrap();
        assert_eq!(library.items.len(), 1);
        assert_eq!(library.items[0].file_name, "已迁移的 CCP 背景");
        assert!(library.items[0].current);
        assert_eq!(
            library.current_background_id.as_deref(),
            Some(library.items[0].id.as_str())
        );
        assert!(
            store
                .manager_background_item_path(&library.items[0].id)
                .unwrap()
                .is_file()
        );
        assert!(
            !store
                .manager_background_dir()
                .join(MANAGER_BACKGROUND_FILE)
                .exists()
        );
        assert!(store.active_manager_background().unwrap().user_override);
    }

    #[test]
    fn manager_background_override_rejects_invalid_images_without_changing_state() {
        let temp = tempfile::tempdir().unwrap();
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();

        let low_resolution = temp.path().join("low.png");
        write_manager_background(&low_resolution, 1279, 720, [1, 2, 3]);
        assert!(
            store
                .set_manager_background(&low_resolution)
                .unwrap_err()
                .to_string()
                .contains("长边至少需要 1280 像素，短边至少需要 720 像素")
        );

        let fake = temp.path().join("fake.png");
        fs::write(&fake, b"not an image").unwrap();
        assert!(store.set_manager_background(&fake).is_err());

        let oversized = temp.path().join("oversized.png");
        fs::write(
            &oversized,
            vec![0_u8; MAX_MANAGER_BACKGROUND_BYTES as usize + 1],
        )
        .unwrap();
        assert!(
            store
                .set_manager_background(&oversized)
                .unwrap_err()
                .to_string()
                .contains("16 MB")
        );

        let active = store.active_manager_background().unwrap();
        assert!(!active.user_override);
        assert!(active.data_uri.is_none());
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
    fn css_line_endings_compile_to_the_same_payload() {
        let temp = tempfile::tempdir().unwrap();
        let lf_source = temp.path().join("lf-source");
        let mixed_source = temp.path().join("mixed-source");
        let canonical_css = ":root {\n  color: red;\n}\n";

        write_theme(&lf_source, "line-ending-theme", canonical_css);
        write_theme(
            &mixed_source,
            "line-ending-theme",
            ":root {\r\n  color: red;\r}\r\n",
        );
        let (_, validated_css, _) = validate_package(&mixed_source).unwrap();
        assert_eq!(validated_css, canonical_css);

        let mut payloads = Vec::new();
        for (index, source) in [lf_source, mixed_source].iter().enumerate() {
            let store = CodexThemeStore::open(temp.path().join(format!("store-{index}"))).unwrap();
            store.import_theme(source).unwrap();
            store.apply_theme("line-ending-theme").unwrap();
            payloads.push(store.active_theme_payload().unwrap());
        }

        assert_eq!(payloads[0], payloads[1]);
        assert_eq!(payloads[0].css, canonical_css);
        assert!(!payloads[0].css.contains('\r'));
    }

    #[test]
    fn dream_skin_runtime_compat_targets_the_current_home_layout_child() {
        let legacy_css = format!(
            r#"html.theme [role="main"]:has(
  {LEGACY_DREAM_SKIN_HOME_LAYOUT_ANCHOR} > div:first-child {{ min-height: 430px; }}"#
        );

        let upgraded = apply_official_theme_runtime_compat("codex-dream-skin-macos", legacy_css);

        assert!(upgraded.contains(CURRENT_DREAM_SKIN_HOME_LAYOUT_ANCHOR));
        assert!(!upgraded.contains(LEGACY_DREAM_SKIN_HOME_LAYOUT_ANCHOR));
        assert!(upgraded.contains(DREAM_SKIN_HOME_LAYOUT_COMPAT_MARKER));
    }

    #[test]
    fn unrelated_theme_runtime_css_is_not_rewritten() {
        let css = format!(
            r#"html.theme [role="main"]:has(
  {LEGACY_DREAM_SKIN_HOME_LAYOUT_ANCHOR} {{ min-height: 100%; }}"#
        );

        assert_eq!(
            apply_official_theme_runtime_compat("aurora-glass", css.clone()),
            css
        );
    }

    #[test]
    fn repository_theme_directories_and_archives_compile_to_the_same_payload() {
        let repository_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let catalog_ids = OFFICIAL_THEMES
            .iter()
            .map(|theme| theme.id)
            .collect::<BTreeSet<_>>();
        let expected_ids = [
            "codex-dream-skin-macos",
            "codex-dream-skin-windows",
            "aurora-glass",
            "clockwork-fox-spirit",
            "cyber-changan",
            "obsidian-gold",
            "verdant-sanctuary",
            "lotus-fire-nezha",
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();
        assert_eq!(catalog_ids, expected_ids);
        for (theme_id, expected_shell) in [
            ("codex-dream-skin-macos", "light"),
            ("codex-dream-skin-windows", "light"),
            ("aurora-glass", "dark"),
            ("clockwork-fox-spirit", "dark"),
            ("cyber-changan", "dark"),
            ("obsidian-gold", "dark"),
            ("verdant-sanctuary", "dark"),
            ("lotus-fire-nezha", "dark"),
        ] {
            let directory = repository_root.join("Theme").join(theme_id);
            let archive = repository_root
                .join("Theme")
                .join(format!("{theme_id}.zip"));
            let definition = OFFICIAL_THEMES
                .iter()
                .find(|theme| theme.id == theme_id)
                .unwrap();
            assert_eq!(
                sha256_bytes(&fs::read(&archive).unwrap()),
                format!("sha256:{}", definition.archive_sha256),
                "official archive hash drifted for {theme_id}"
            );
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
                if expected_shell == "light" {
                    assert!(
                        payload
                            .css
                            .contains("CCP light-theme runtime compatibility")
                    );
                    assert!(payload.css.contains("--color-token-foreground"));
                    assert!(payload.css.contains("--vscode-foreground"));
                    assert!(
                        payload
                            .css
                            .contains(":root[data-ccp-theme-shell=\"light\"] main:has(")
                    );
                } else {
                    assert!(
                        !payload
                            .css
                            .contains("CCP light-theme runtime compatibility")
                    );
                }
                assert!(
                    payload.css.contains(r#"[data-feature="game-source"]"#)
                        || payload.css.contains(r#"[data-testid="home-icon"]"#),
                    "{theme_id} compiled from {source:?} must target a Codex native home fingerprint"
                );
                if theme_id.starts_with("codex-dream-skin-") {
                    assert!(payload.css.contains(DREAM_SKIN_HOME_LAYOUT_COMPAT_MARKER));
                    assert!(payload.css.contains(CURRENT_DREAM_SKIN_HOME_LAYOUT_ANCHOR));
                    assert!(!payload.css.contains(LEGACY_DREAM_SKIN_HOME_LAYOUT_ANCHOR));
                }
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
                    expected_shell
                );
                if expected_shell == "dark" {
                    assert_eq!(
                        payload.root_attributes.attributes["data-ccp-theme-origin"],
                        "theme-studio-curated"
                    );
                }
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
    fn delete_inactive_theme_removes_package_and_previous_versions() {
        let temp = tempfile::tempdir().unwrap();
        let version_one = temp.path().join("delete-v1");
        let version_two = temp.path().join("delete-v2");
        write_theme_version(
            &version_one,
            "delete-theme",
            "1.0.0",
            ":root { --ccp-delete-version: 1; }",
        );
        write_theme_version(
            &version_two,
            "delete-theme",
            "2.0.0",
            ":root { --ccp-delete-version: 2; }",
        );
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();
        store.import_theme(&version_one).unwrap();
        store.import_theme_with_options(&version_two, true).unwrap();
        assert!(store.library_dir().join("delete-theme").exists());
        assert!(store.backups_dir().join("delete-theme").exists());

        let deleted = store.delete_theme("delete-theme").unwrap();
        assert!(deleted.persisted);
        assert!(!deleted.restart_required);
        assert!(!store.library_dir().join("delete-theme").exists());
        assert!(!store.backups_dir().join("delete-theme").exists());
        assert_eq!(store.list_themes().unwrap().themes.len(), 1);
    }

    #[test]
    fn delete_rejects_default_and_current_theme() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("active-delete");
        write_theme(
            &source,
            "active-delete",
            ":root { --ccp-active-delete: 1; }",
        );
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();
        assert!(store.delete_theme(DEFAULT_THEME_ID).is_err());
        store.import_theme(&source).unwrap();
        store.apply_theme("active-delete").unwrap();
        let error = store.delete_theme("active-delete").unwrap_err();
        assert!(format!("{error:#}").contains("正在使用"));
        assert!(store.library_dir().join("active-delete").exists());
    }

    #[test]
    fn unfinished_delete_journal_restores_theme_files() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("delete-recovery");
        write_theme(
            &source,
            "delete-recovery",
            ":root { --ccp-delete-recovery: 1; }",
        );
        let store_root = temp.path().join("store");
        let store = CodexThemeStore::open(&store_root).unwrap();
        store.import_theme(&source).unwrap();
        let before = store.read_state().unwrap();
        let operation_id = "interrupted-delete";
        let staging = store.staging_dir().join(operation_id);
        let staged_theme = staging.join("theme");
        fs::create_dir_all(&staging).unwrap();
        fs::rename(store.library_dir().join("delete-recovery"), &staged_theme).unwrap();
        let journal = MutationJournal {
            operation_id: operation_id.to_string(),
            operation_type: "delete".to_string(),
            theme_id: "delete-recovery".to_string(),
            phase: "files-staged".to_string(),
            started_at: now_secs(),
            state_before: before,
            staging_dir: Some(PathBuf::from("staging").join(operation_id)),
            target_dir: Some(PathBuf::from("library").join("delete-recovery")),
            backup_dir: Some(PathBuf::from("staging").join(operation_id).join("theme")),
            version_backup_dir: Some(PathBuf::from("backups").join("delete-recovery")),
            staged_version_backup_dir: Some(
                PathBuf::from("staging").join(operation_id).join("versions"),
            ),
            finished_at: None,
            result: None,
        };
        store.write_journal(&journal).unwrap();
        drop(store);

        let reopened = CodexThemeStore::open(&store_root).unwrap();
        assert!(reopened.library_dir().join("delete-recovery").exists());
        assert!(
            reopened
                .read_state()
                .unwrap()
                .themes
                .iter()
                .any(|theme| theme.manifest.id == "delete-recovery")
        );
        assert!(!reopened.staging_dir().join(operation_id).exists());
    }

    #[test]
    fn committed_delete_journal_finishes_staging_cleanup() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("committed-delete");
        write_theme(
            &source,
            "committed-delete",
            ":root { --ccp-committed-delete: 1; }",
        );
        let store_root = temp.path().join("store");
        let store = CodexThemeStore::open(&store_root).unwrap();
        store.import_theme(&source).unwrap();
        let before = store.read_state().unwrap();
        let operation_id = "committed-delete-cleanup";
        let staging = store.staging_dir().join(operation_id);
        let staged_theme = staging.join("theme");
        fs::create_dir_all(&staging).unwrap();
        fs::rename(store.library_dir().join("committed-delete"), &staged_theme).unwrap();
        let mut committed = before.clone();
        committed
            .themes
            .retain(|theme| theme.manifest.id != "committed-delete");
        store.write_state(&committed).unwrap();
        let journal = MutationJournal {
            operation_id: operation_id.to_string(),
            operation_type: "delete".to_string(),
            theme_id: "committed-delete".to_string(),
            phase: "state-committed".to_string(),
            started_at: now_secs(),
            state_before: before,
            staging_dir: Some(PathBuf::from("staging").join(operation_id)),
            target_dir: Some(PathBuf::from("library").join("committed-delete")),
            backup_dir: Some(PathBuf::from("staging").join(operation_id).join("theme")),
            version_backup_dir: Some(PathBuf::from("backups").join("committed-delete")),
            staged_version_backup_dir: Some(
                PathBuf::from("staging").join(operation_id).join("versions"),
            ),
            finished_at: None,
            result: None,
        };
        store.write_journal(&journal).unwrap();
        drop(store);

        let reopened = CodexThemeStore::open(&store_root).unwrap();
        assert!(!reopened.library_dir().join("committed-delete").exists());
        assert!(!reopened.staging_dir().join(operation_id).exists());
        assert!(
            !reopened
                .read_state()
                .unwrap()
                .themes
                .iter()
                .any(|theme| theme.manifest.id == "committed-delete")
        );
        assert!(
            reopened
                .history_dir()
                .join(format!("{operation_id}.json"))
                .exists()
        );
    }

    #[tokio::test]
    async fn official_download_rejects_an_installed_theme_before_network_access() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("installed-official");
        write_theme(
            &source,
            "aurora-glass",
            ":root { --ccp-installed-official: 1; }",
        );
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();
        store.import_theme(&source).unwrap();

        let error = store
            .download_official_theme("aurora-glass")
            .await
            .unwrap_err();
        assert!(format!("{error:#}").contains("已安装"));
    }

    #[tokio::test]
    async fn official_download_rejects_unknown_theme_ids() {
        let temp = tempfile::tempdir().unwrap();
        let store = CodexThemeStore::open(temp.path().join("store")).unwrap();
        let error = store
            .download_official_theme("not-in-the-catalog")
            .await
            .unwrap_err();
        assert!(format!("{error:#}").contains("官方下载目录"));
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
            version_backup_dir: None,
            staged_version_backup_dir: None,
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
