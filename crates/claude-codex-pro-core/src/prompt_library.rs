//! Prompt 内容库：从本仓库 GitHub 读取清单、下载提示词与技能、按清单安装与卸载。
//!
//! 清单地址写死在 `Prompt/` 目录下的 `index.json`。下载内容只落到应用缓存，
//! 不直接写客户端目录；写入客户端由 `client_deploy` 与 `install_skills` 负责。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::settings::atomic_write;

pub const INDEX_URL: &str =
    "https://raw.githubusercontent.com/DamonZS/Claude-Codex-Pro-Tool/main/Prompt/index.json";
const RAW_ROOT: &str = "https://raw.githubusercontent.com/DamonZS/Claude-Codex-Pro-Tool/main";
pub const LOCAL_PROJECT_ROOT: &str = r"D:\Project\Claude-Codex-Pro-Tool\Prompt";
const SKILL_ENTRY: &str = "SKILL.md";
const MAX_FILE_BYTES: usize = 1024 * 1024;
const MAX_TOOL_FILE_BYTES: usize = 8 * 1024 * 1024;
const MAX_TOOL_PACKAGE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PromptEntry {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub version: String,
    /// 适用目标；每个客户端内容不同，缺省表示适配全部。
    #[serde(default)]
    pub targets: Vec<String>,
    /// 随该版本一起部署的资源 ID；为空表示不附带资源。
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillEntry {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub path: String,
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolFileEntry {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolEntry {
    pub id: String,
    pub title: String,
    pub version: String,
    pub description: String,
    pub source_repo: String,
    pub source_revision: String,
    pub license_id: String,
    pub license_path: String,
    pub platforms: Vec<String>,
    pub size: u64,
    pub files: Vec<ToolFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PromptIndex {
    #[serde(default)]
    pub version: u64,
    #[serde(default)]
    pub prompts: Vec<PromptEntry>,
    #[serde(default)]
    pub skills: Vec<SkillEntry>,
    #[serde(default)]
    pub tools: Vec<ToolEntry>,
}

pub fn parse_index(raw: &str) -> anyhow::Result<PromptIndex> {
    let index: PromptIndex = serde_json::from_str(raw).context("Prompt 清单格式无效")?;
    let known: Vec<String> = crate::client_deploy::targets()
        .into_iter()
        .map(|target| target.id)
        .collect();
    for entry in &index.prompts {
        validate_prompt_entry(entry)?;
        for target in &entry.targets {
            if !known.iter().any(|id| id == target) {
                bail!("提示词 {} 声明的目标 {target} 不在支持列表内", entry.id);
            }
        }
    }
    for entry in &index.skills {
        validate_skill_entry(entry)?;
    }
    for entry in &index.tools {
        validate_tool_entry(entry)?;
    }
    let skill_names: std::collections::BTreeSet<_> =
        index.skills.iter().map(|item| item.name.as_str()).collect();
    let tool_ids: std::collections::BTreeSet<_> =
        index.tools.iter().map(|item| item.id.as_str()).collect();
    for entry in &index.prompts {
        if entry
            .skills
            .iter()
            .any(|name| !skill_names.contains(name.as_str()))
        {
            bail!("提示词 {} 关联了不存在的技能", entry.id);
        }
        if entry.tools.iter().any(|id| !tool_ids.contains(id.as_str())) {
            bail!("提示词 {} 关联了不存在的工具", entry.id);
        }
    }
    Ok(index)
}

fn manifest_components(value: &str) -> anyhow::Result<Vec<&str>> {
    if value.is_empty()
        || value.contains('\\')
        || value.starts_with('/')
        || value.contains('\0')
        || value.contains(':')
        || Path::new(value).is_absolute()
    {
        bail!("清单路径不合法：{value}");
    }
    let parts: Vec<_> = value.split('/').collect();
    if parts.iter().any(|part| {
        part.is_empty()
            || *part == "."
            || *part == ".."
            || !matches!(
                Path::new(part).components().next(),
                Some(Component::Normal(_))
            )
    }) {
        bail!("清单路径不合法：{value}");
    }
    Ok(parts)
}

fn validate_name_component(name: &str) -> anyhow::Result<()> {
    let parts = manifest_components(name)?;
    if parts.len() != 1 {
        bail!("技能名不合法：{name}");
    }
    Ok(())
}

fn validate_prompt_entry(entry: &PromptEntry) -> anyhow::Result<()> {
    validate_name_component(&entry.id).context("提示词 ID 不合法")?;
    let parts = manifest_components(&entry.path)?;
    if parts.len() < 2 || parts[0] != "prompts" || !parts.last().unwrap().ends_with(".md") {
        bail!("清单中的提示词路径不合法：{}", entry.path);
    }
    Ok(())
}

fn validate_skill_entry(entry: &SkillEntry) -> anyhow::Result<()> {
    validate_name_component(&entry.name).context("技能名不合法")?;
    let root = manifest_components(&entry.path)?;
    if root.as_slice() != ["skills", entry.name.as_str()] {
        bail!("清单中的技能路径不合法：{}", entry.path);
    }
    let mut has_entry = false;
    let mut seen = std::collections::BTreeSet::new();
    for file in &entry.files {
        let parts = manifest_components(file)?;
        if parts.len() <= root.len() || parts[..root.len()] != root {
            bail!("技能 {} 的文件路径越界：{file}", entry.name);
        }
        if parts.as_slice() == ["skills", entry.name.as_str(), SKILL_ENTRY] {
            has_entry = true;
        }
        if !seen.insert(file) {
            bail!("技能 {} 的文件路径重复：{file}", entry.name);
        }
    }
    if !has_entry {
        bail!("技能 {} 未包含 {SKILL_ENTRY}", entry.name);
    }
    Ok(())
}

fn validate_tool_entry(entry: &ToolEntry) -> anyhow::Result<()> {
    validate_name_component(&entry.id).context("工具 ID 不合法")?;
    validate_name_component(&entry.version).context("工具版本不合法")?;
    if entry.title.trim().is_empty()
        || entry.description.trim().is_empty()
        || entry.license_id.trim().is_empty()
        || !entry
            .source_revision
            .chars()
            .all(|ch| ch.is_ascii_hexdigit())
        || entry.source_revision.len() != 40
        || !entry.source_repo.starts_with("https://github.com/")
    {
        bail!("工具 {} 缺少有效来源、版本或许可证信息", entry.id);
    }
    if entry.platforms.is_empty()
        || entry
            .platforms
            .iter()
            .any(|platform| !matches!(platform.as_str(), "windows" | "macos" | "linux"))
    {
        bail!("工具 {} 的平台声明无效", entry.id);
    }
    let root = format!("tools/{}/", entry.id);
    let mut seen = std::collections::BTreeSet::new();
    let mut total = 0_u64;
    let mut has_license = false;
    if entry.files.is_empty() {
        bail!("工具 {} 没有文件", entry.id);
    }
    for file in &entry.files {
        let parts = manifest_components(&file.path)?;
        if parts.len() < 3 || !file.path.starts_with(&root) {
            bail!("工具 {} 的文件路径越界：{}", entry.id, file.path);
        }
        if !seen.insert(&file.path) {
            bail!("工具 {} 的文件路径重复：{}", entry.id, file.path);
        }
        if file.size == 0 || file.size > MAX_TOOL_FILE_BYTES as u64 {
            bail!("工具文件 {} 大小无效或超过限制", file.path);
        }
        if file.sha256.len() != 64 || !file.sha256.chars().all(|ch| ch.is_ascii_hexdigit()) {
            bail!("工具文件 {} 的 SHA-256 无效", file.path);
        }
        total = total.checked_add(file.size).context("工具包大小溢出")?;
        has_license |= file.path == entry.license_path;
    }
    if !has_license || total != entry.size || total > MAX_TOOL_PACKAGE_BYTES as u64 {
        bail!("工具 {} 的许可证文件或包大小声明无效", entry.id);
    }
    Ok(())
}

fn relative_skill_file(entry: &SkillEntry, file: &str) -> anyhow::Result<PathBuf> {
    let root = manifest_components(&entry.path)?;
    let parts = manifest_components(file)?;
    if parts.len() <= root.len() || parts[..root.len()] != root {
        bail!("技能 {} 的文件路径越界：{file}", entry.name);
    }
    Ok(parts[root.len()..].iter().collect())
}

fn ensure_no_symlink(path: &Path) -> anyhow::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!("拒绝符号链接路径：{}", path.display());
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("无法检查路径 {}", path.display())),
    }
}

fn path_exists(path: &Path) -> anyhow::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("无法检查路径 {}", path.display())),
    }
}

fn ensure_no_symlink_below(root: &Path, relative: &Path) -> anyhow::Result<()> {
    ensure_no_symlink(root)?;
    match fs::symlink_metadata(root) {
        Ok(metadata) if !metadata.is_dir() => bail!("路径不是目录：{}", root.display()),
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| format!("无法检查路径 {}", root.display()));
        }
    }
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            bail!("拒绝非相对路径：{}", relative.display());
        };
        current.push(part);
        ensure_no_symlink(&current)?;
        match fs::symlink_metadata(&current) {
            Ok(metadata) if !metadata.is_dir() && current != root.join(relative) => {
                bail!("路径组件不是目录：{}", current.display());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| format!("无法检查路径 {}", current.display()));
            }
        }
    }
    Ok(())
}

fn ensure_tree_has_no_symlinks(root: &Path) -> anyhow::Result<()> {
    ensure_no_symlink(root)?;
    let metadata =
        fs::symlink_metadata(root).with_context(|| format!("无法读取目录 {}", root.display()))?;
    if !metadata.is_dir() {
        bail!("路径不是目录：{}", root.display());
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in
            fs::read_dir(&current).with_context(|| format!("无法读取目录 {}", current.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                bail!("拒绝包含符号链接的目录：{}", path.display());
            } else if file_type.is_dir() {
                stack.push(path);
            } else if !file_type.is_file() {
                bail!("拒绝非普通文件：{}", path.display());
            }
        }
    }
    Ok(())
}

struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    fn create(parent: &Path, prefix: &str) -> anyhow::Result<Self> {
        fs::create_dir_all(parent).with_context(|| format!("无法创建目录 {}", parent.display()))?;
        ensure_no_symlink(parent)?;
        for _ in 0..3 {
            let path = parent.join(format!("{prefix}-{}", uuid::Uuid::new_v4()));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("无法创建暂存目录 {}", path.display()));
                }
            }
        }
        bail!("无法分配唯一暂存目录")
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn backup_path(parent: &Path, prefix: &str) -> PathBuf {
    parent.join(format!("{prefix}-{}", uuid::Uuid::new_v4()))
}

fn publish_directory(stage: &Path, destination: &Path) -> anyhow::Result<Option<PathBuf>> {
    ensure_no_symlink(destination.parent().unwrap_or_else(|| Path::new(".")))?;
    ensure_no_symlink(destination)?;
    if destination.exists() {
        ensure_tree_has_no_symlinks(destination)?;
    }
    let backup = destination.exists().then(|| {
        backup_path(
            destination.parent().unwrap_or_else(|| Path::new(".")),
            ".ccp-skill-backup",
        )
    });
    if let Some(path) = &backup {
        fs::rename(destination, path)
            .with_context(|| format!("无法暂存旧目录 {}", destination.display()))?;
    }
    if let Err(error) = fs::rename(stage, destination) {
        if let Some(path) = &backup {
            if let Err(restore_error) = fs::rename(path, destination) {
                return Err(error).with_context(|| {
                    format!(
                        "发布目录失败；旧目录仍保留于 {}，恢复失败：{restore_error}",
                        path.display()
                    )
                });
            }
        }
        return Err(error).with_context(|| format!("无法发布目录 {}", destination.display()));
    }
    Ok(backup)
}

fn rollback_published_directory(destination: &Path, backup: Option<&Path>) -> anyhow::Result<()> {
    if destination.exists() {
        fs::remove_dir_all(destination)
            .with_context(|| format!("无法移除新目录 {}", destination.display()))?;
    }
    if let Some(backup) = backup {
        fs::rename(backup, destination)
            .with_context(|| format!("无法恢复旧目录 {}", backup.display()))?;
    }
    Ok(())
}

pub fn skill_install_dir(target_root: &Path) -> PathBuf {
    target_root.join("skills")
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn package_sha256(files: &BTreeMap<String, String>) -> String {
    let manifest = files
        .iter()
        .map(|(path, digest)| format!("{path}\0{digest}\n"))
        .collect::<String>();
    sha256_hex(manifest.as_bytes())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstalledSkill {
    pub name: String,
    /// 安装到的客户端数据目录。
    pub target_home: String,
    pub installed_at_unix: u64,
    /// 相对技能目录的文件路径 → SHA-256。
    pub files: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstalledTool {
    pub id: String,
    pub version: String,
    pub installed_at_unix: u64,
    pub files: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolActionOutcome {
    pub id: String,
    pub version: String,
    pub ok: bool,
    pub install_path: String,
    pub sha256: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct LibraryState {
    #[serde(default)]
    skills: Vec<InstalledSkill>,
    #[serde(default)]
    tools: Vec<InstalledTool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillActionOutcome {
    pub name: String,
    pub target_id: String,
    pub ok: bool,
    pub message: String,
}

pub struct PromptLibrary {
    root: PathBuf,
    tool_root: PathBuf,
    project_root: PathBuf,
    state_path: PathBuf,
    #[cfg(test)]
    fail_next_state_write: std::sync::atomic::AtomicBool,
}

impl PromptLibrary {
    pub fn open(root: impl Into<PathBuf>) -> anyhow::Result<Self> {
        Self::open_with_project(root, PathBuf::from(LOCAL_PROJECT_ROOT))
    }

    pub fn open_with_project(
        root: impl Into<PathBuf>,
        project_root: impl Into<PathBuf>,
    ) -> anyhow::Result<Self> {
        let root = root.into();
        let project_root = project_root.into();
        ensure_no_symlink(&root)?;
        fs::create_dir_all(&root).context("无法创建内容缓存目录")?;
        ensure_no_symlink(&root)?;
        let library = Self {
            state_path: root.join("skills-state.json"),
            tool_root: root.parent().unwrap_or(&root).join("prompt-tools"),
            project_root,
            root,
            #[cfg(test)]
            fail_next_state_write: std::sync::atomic::AtomicBool::new(false),
        };
        match fs::symlink_metadata(&library.state_path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                bail!("拒绝符号链接状态文件：{}", library.state_path.display());
            }
            Ok(metadata) if !metadata.is_file() => {
                bail!(
                    "技能库状态路径不是普通文件：{}",
                    library.state_path.display()
                );
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                library.write_state(&LibraryState::default())?;
            }
            Err(error) => return Err(error).context("无法检查技能库状态文件"),
        }
        Ok(library)
    }

    pub fn open_default() -> anyhow::Result<Self> {
        Self::open(crate::paths::default_app_state_dir().join("prompt-cache"))
    }

    pub fn cache_dir(&self) -> &Path {
        &self.root
    }

    pub fn project_dir(&self) -> &Path {
        &self.project_root
    }

    pub fn tool_install_dir(&self) -> &Path {
        &self.tool_root
    }

    pub fn prompt_source_path(&self, entry: &PromptEntry) -> PathBuf {
        self.project_root.join(&entry.path)
    }

    pub fn skill_source_path(&self, entry: &SkillEntry) -> PathBuf {
        self.project_root.join(&entry.path)
    }

    pub fn tool_source_path(&self, entry: &ToolEntry) -> PathBuf {
        self.project_root.join("tools").join(&entry.id)
    }

    pub fn prompt_cache_path(&self, entry: &PromptEntry) -> PathBuf {
        self.root.join("prompts").join(format!("{}.md", entry.id))
    }

    pub fn skill_cache_path(&self, entry: &SkillEntry) -> PathBuf {
        self.root.join("skills").join(&entry.name)
    }

    pub fn tool_cache_path(&self, entry: &ToolEntry) -> PathBuf {
        self.root.join("tools").join(&entry.id).join(&entry.version)
    }

    fn read_local_file(&self, relative: &str, max_bytes: usize) -> anyhow::Result<Vec<u8>> {
        let relative_path = Path::new(relative);
        ensure_no_symlink_below(&self.project_root, relative_path)?;
        let path = self.project_root.join(relative_path);
        let metadata =
            fs::metadata(&path).with_context(|| format!("本地内容不存在：{}", path.display()))?;
        if !metadata.is_file() || metadata.len() > max_bytes as u64 {
            bail!("本地内容无效或超过大小限制：{}", path.display());
        }
        fs::read(&path).with_context(|| format!("无法读取本地内容：{}", path.display()))
    }

    /// 拉取远端清单。网络失败时返回错误，由调用方回退内置模板。
    pub async fn fetch_index(&self) -> anyhow::Result<PromptIndex> {
        if let Ok(raw) = fs::read_to_string(self.project_root.join("index.json")) {
            if let Ok(index) = parse_index(&raw) {
                return Ok(index);
            }
        }
        let client = crate::http_client::proxied_client("claude-codex-pro")?;
        let response = client
            .get(INDEX_URL)
            .send()
            .await
            .context("无法访问 Prompt 清单")?;
        if !response.status().is_success() {
            bail!("Prompt 清单请求失败：HTTP {}", response.status());
        }
        let raw = response.text().await.context("无法读取 Prompt 清单")?;
        parse_index(&raw)
    }

    /// 按条目下载提示词正文并缓存，返回本地绝对路径。
    pub async fn fetch_prompt(&self, entry: &PromptEntry) -> anyhow::Result<PathBuf> {
        validate_prompt_entry(entry)?;
        let relative = Path::new("prompts").join(format!("{}.md", entry.id));
        if let Ok(bytes) = self.read_local_file(&entry.path, MAX_FILE_BYTES) {
            ensure_no_symlink_below(&self.root, Path::new("prompts"))?;
            fs::create_dir_all(self.root.join("prompts")).context("无法创建提示词缓存目录")?;
            ensure_no_symlink_below(&self.root, &relative)?;
            let cache_path = self.root.join(relative);
            atomic_write(&cache_path, &bytes)?;
            return Ok(cache_path);
        }
        let url = format!("{RAW_ROOT}/{}", entry.path);
        let client = crate::http_client::proxied_client("claude-codex-pro")?;
        let response = client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("无法下载提示词 {}", entry.id))?;
        if !response.status().is_success() {
            bail!("提示词 {} 下载失败：HTTP {}", entry.id, response.status());
        }
        let bytes = response.bytes().await.context("无法读取提示词内容")?;
        if bytes.len() > MAX_FILE_BYTES {
            bail!("提示词 {} 超过 1 MiB 上限", entry.id);
        }
        ensure_no_symlink_below(&self.root, Path::new("prompts"))?;
        fs::create_dir_all(self.root.join("prompts")).context("无法创建提示词缓存目录")?;
        ensure_no_symlink_below(&self.root, &relative)?;
        let cache_path = self.root.join(relative);
        atomic_write(&cache_path, &bytes)?;
        Ok(cache_path)
    }

    /// 按技能条目下载全部文件并缓存，返回缓存目录。
    pub async fn fetch_skill(&self, entry: &SkillEntry) -> anyhow::Result<PathBuf> {
        validate_skill_entry(entry)?;
        let skills_root = self.root.join("skills");
        ensure_no_symlink_below(&self.root, Path::new("skills"))?;
        fs::create_dir_all(&skills_root).context("无法创建技能缓存目录")?;
        ensure_no_symlink_below(&self.root, Path::new("skills"))?;
        let stage = TemporaryDirectory::create(&skills_root, ".ccp-skill-stage")?;
        let local_files = entry
            .files
            .iter()
            .map(|file| self.read_local_file(file, MAX_FILE_BYTES))
            .collect::<anyhow::Result<Vec<_>>>();
        let files = if let Ok(files) = local_files {
            files
        } else {
            let client = crate::http_client::proxied_client("claude-codex-pro")?;
            let mut files = Vec::with_capacity(entry.files.len());
            for file in &entry.files {
                let url = format!("{RAW_ROOT}/{file}");
                let response = client
                    .get(&url)
                    .send()
                    .await
                    .with_context(|| format!("无法下载技能文件 {file}"))?;
                if !response.status().is_success() {
                    bail!("技能文件 {file} 下载失败：HTTP {}", response.status());
                }
                let bytes = response.bytes().await.context("无法读取技能文件")?;
                if bytes.len() > MAX_FILE_BYTES {
                    bail!("技能文件 {file} 超过 1 MiB 上限");
                }
                files.push(bytes.to_vec());
            }
            files
        };
        for (file, bytes) in entry.files.iter().zip(files) {
            if bytes.len() > MAX_FILE_BYTES {
                bail!("技能文件 {file} 超过 1 MiB 上限");
            }
            let relative = relative_skill_file(entry, file)?;
            let path = stage.path().join(relative);
            let parent = path.parent().context("技能文件路径没有父目录")?;
            fs::create_dir_all(parent).context("无法创建技能暂存目录")?;
            atomic_write(&path, &bytes)?;
        }
        ensure_tree_has_no_symlinks(stage.path())?;
        if !stage.path().join(SKILL_ENTRY).is_file() {
            bail!("下载的技能缺少 {SKILL_ENTRY}");
        }
        let destination = skills_root.join(&entry.name);
        let backup = publish_directory(stage.path(), &destination)?;
        if let Some(path) = backup {
            let _ = fs::remove_dir_all(path);
        }
        Ok(destination)
    }

    pub async fn fetch_tool(&self, entry: &ToolEntry) -> anyhow::Result<PathBuf> {
        validate_tool_entry(entry)?;
        let cache_root = self.root.join("tools").join(&entry.id).join(&entry.version);
        let parent = cache_root.parent().context("工具缓存目录无父目录")?;
        fs::create_dir_all(parent).context("无法创建工具缓存目录")?;
        ensure_no_symlink_below(
            &self.root,
            &Path::new("tools").join(&entry.id).join(&entry.version),
        )?;
        let stage = TemporaryDirectory::create(parent, ".ccp-tool-stage")?;
        let local_files = entry
            .files
            .iter()
            .map(|file| self.read_local_file(&file.path, MAX_TOOL_FILE_BYTES))
            .collect::<anyhow::Result<Vec<_>>>()
            .ok()
            .filter(|files| {
                files.iter().zip(&entry.files).all(|(bytes, file)| {
                    bytes.len() as u64 == file.size
                        && sha256_hex(bytes) == file.sha256.to_ascii_lowercase()
                })
            });
        let files = if let Some(files) = local_files {
            files
        } else {
            let client = crate::http_client::proxied_client("claude-codex-pro")?;
            let mut files = Vec::with_capacity(entry.files.len());
            for file in &entry.files {
                let mut response = client
                    .get(format!("{RAW_ROOT}/{}", file.path))
                    .send()
                    .await
                    .with_context(|| format!("无法下载工具文件 {}", file.path))?;
                if !response.status().is_success() {
                    bail!(
                        "工具文件 {} 下载失败：HTTP {}",
                        file.path,
                        response.status()
                    );
                }
                if response
                    .content_length()
                    .is_some_and(|length| length > file.size || length > MAX_TOOL_FILE_BYTES as u64)
                {
                    bail!("工具文件 {} 超过清单大小或限制", file.path);
                }
                let mut bytes = Vec::with_capacity(file.size as usize);
                while let Some(chunk) = response.chunk().await.context("无法读取工具文件")?
                {
                    let next_len = bytes.len().saturating_add(chunk.len());
                    if next_len > file.size as usize || next_len > MAX_TOOL_FILE_BYTES {
                        bail!("工具文件 {} 超过清单大小或限制", file.path);
                    }
                    bytes.extend_from_slice(&chunk);
                }
                if bytes.len() as u64 != file.size
                    || sha256_hex(&bytes) != file.sha256.to_ascii_lowercase()
                {
                    bail!("工具文件 {} 的大小或 SHA-256 不匹配", file.path);
                }
                files.push(bytes);
            }
            files
        };
        for (file, bytes) in entry.files.iter().zip(files) {
            let relative =
                Path::new(&file.path).strip_prefix(Path::new("tools").join(&entry.id))?;
            let path = stage.path().join(relative);
            fs::create_dir_all(path.parent().context("工具文件路径无父目录")?)?;
            atomic_write(&path, &bytes)?;
        }
        ensure_tree_has_no_symlinks(stage.path())?;
        let backup = publish_directory(stage.path(), &cache_root)?;
        if let Some(path) = backup {
            let _ = fs::remove_dir_all(path);
        }
        Ok(cache_root)
    }

    pub fn list_installed_tools(&self) -> anyhow::Result<Vec<InstalledTool>> {
        Ok(self.read_state()?.tools)
    }

    pub fn install_tool(&self, entry: &ToolEntry) -> anyhow::Result<ToolActionOutcome> {
        validate_tool_entry(entry)?;
        let source = self.root.join("tools").join(&entry.id).join(&entry.version);
        ensure_no_symlink_below(
            &self.root,
            &Path::new("tools").join(&entry.id).join(&entry.version),
        )?;
        let actual = fingerprint(&source)?;
        let expected: BTreeMap<_, _> = entry
            .files
            .iter()
            .map(|file| {
                let relative = Path::new(&file.path)
                    .strip_prefix(Path::new("tools").join(&entry.id))
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                (relative, file.sha256.to_ascii_lowercase())
            })
            .collect();
        if actual != expected {
            bail!("工具 {} 缓存文件与清单不一致", entry.id);
        }
        let destination = self.tool_root.join(&entry.id).join(&entry.version);
        ensure_no_symlink_below(&self.tool_root, &Path::new(&entry.id).join(&entry.version))?;
        let mut state = self.read_state()?;
        if let Some(installed) = state
            .tools
            .iter()
            .find(|tool| tool.id == entry.id && tool.version == entry.version)
        {
            if installed.files == expected && fingerprint(&destination)? == expected {
                return Ok(ToolActionOutcome {
                    id: entry.id.clone(),
                    version: entry.version.clone(),
                    ok: true,
                    install_path: destination.display().to_string(),
                    sha256: package_sha256(&expected),
                    message: "已安装".to_string(),
                });
            }
            bail!("工具 {} {} 已安装内容发生外部修改", entry.id, entry.version);
        }
        fs::create_dir_all(destination.parent().context("工具安装目录无父目录")?)?;
        if path_exists(&destination)? {
            bail!(
                "目标工具目录已存在且不受 CCP 管理：{}",
                destination.display()
            );
        }
        let stage = TemporaryDirectory::create(destination.parent().unwrap(), ".ccp-tool-install")?;
        copy_tree(&source, stage.path())?;
        ensure_tree_has_no_symlinks(stage.path())?;
        let backup = publish_directory(stage.path(), &destination)?;
        debug_assert!(backup.is_none());
        state.tools.push(InstalledTool {
            id: entry.id.clone(),
            version: entry.version.clone(),
            installed_at_unix: now_unix(),
            files: expected.clone(),
        });
        if let Err(error) = self.write_state(&state) {
            let _ = fs::remove_dir_all(&destination);
            return Err(error).context("保存工具安装状态失败，已回滚工具目录");
        }
        Ok(ToolActionOutcome {
            id: entry.id.clone(),
            version: entry.version.clone(),
            ok: true,
            install_path: destination.display().to_string(),
            sha256: package_sha256(&expected),
            message: "已安装到 CCP 数据目录；未执行".to_string(),
        })
    }

    pub fn uninstall_tool(&self, id: &str, version: &str) -> anyhow::Result<ToolActionOutcome> {
        validate_name_component(id)?;
        validate_name_component(version)?;
        let destination = self.tool_root.join(id).join(version);
        ensure_no_symlink_below(&self.tool_root, &Path::new(id).join(version))?;
        let mut state = self.read_state()?;
        let Some(index) = state
            .tools
            .iter()
            .position(|tool| tool.id == id && tool.version == version)
        else {
            return Ok(ToolActionOutcome {
                id: id.to_string(),
                version: version.to_string(),
                ok: false,
                install_path: destination.display().to_string(),
                sha256: String::new(),
                message: "不在 CCP 安装记录内".to_string(),
            });
        };
        let installed = state.tools[index].clone();
        if fingerprint(&destination)? != installed.files {
            bail!("工具 {} {} 内容已变更，保留目录并报告冲突", id, version);
        }
        let before = serde_json::to_vec(&self.read_state()?)?;
        let backup = backup_path(destination.parent().unwrap(), ".ccp-tool-uninstall");
        fs::rename(&destination, &backup).context("无法暂存工具目录")?;
        state.tools.remove(index);
        if let Err(error) = self.write_state(&state) {
            let restore_state = atomic_write(&self.state_path, &before);
            let restore_tool = fs::rename(&backup, &destination);
            if restore_state.is_err() || restore_tool.is_err() {
                return Err(error).context("工具状态写入失败，且回滚未能完全恢复");
            }
            return Err(error).context("工具状态写入失败，已恢复工具目录和状态");
        }
        fs::remove_dir_all(&backup).context("工具已从记录移除，但暂存目录清理失败")?;
        Ok(ToolActionOutcome {
            id: id.to_string(),
            version: version.to_string(),
            ok: true,
            install_path: destination.display().to_string(),
            sha256: package_sha256(&installed.files),
            message: "已卸载".to_string(),
        })
    }

    /// 适用于指定目标的提示词；`targets` 为空表示适配全部目标。
    pub fn prompts_for_target<'a>(index: &'a PromptIndex, target_id: &str) -> Vec<&'a PromptEntry> {
        index
            .prompts
            .iter()
            .filter(|entry| {
                entry.targets.is_empty() || entry.targets.iter().any(|id| id == target_id)
            })
            .collect()
    }

    pub fn list_installed(&self) -> anyhow::Result<Vec<InstalledSkill>> {
        Ok(self.read_state()?.skills)
    }

    /// 把缓存中的技能安装到目标客户端。已存在同名目录且不属本库时跳过。
    pub fn install_skill(
        &self,
        target_id: &str,
        target_home: &Path,
        name: &str,
    ) -> anyhow::Result<SkillActionOutcome> {
        validate_name_component(name)?;
        let source = self.root.join("skills").join(name);
        ensure_no_symlink_below(&self.root, &Path::new("skills").join(name))?;
        if !source.join(SKILL_ENTRY).is_file() {
            bail!("技能 {name} 尚未下载或缺少 {SKILL_ENTRY}");
        }
        ensure_tree_has_no_symlinks(&source)?;
        let destination = skill_install_dir(target_home).join(name);
        ensure_no_symlink_below(target_home, &Path::new("skills").join(name))?;
        let home_metadata = fs::symlink_metadata(target_home)
            .with_context(|| format!("无法读取客户端目录 {}", target_home.display()))?;
        if !home_metadata.is_dir() {
            bail!("客户端目录不是目录：{}", target_home.display());
        }
        let mut state = self.read_state()?;
        let state_before = fs::read(&self.state_path).context("无法备份技能库状态")?;
        let known = state.skills.iter().position(|skill| {
            skill.name == name && skill.target_home == target_home.to_string_lossy()
        });

        let destination_exists = path_exists(&destination)?;
        if known.is_none() && destination_exists {
            return Ok(SkillActionOutcome {
                name: name.to_string(),
                target_id: target_id.to_string(),
                ok: false,
                message: "目标已存在同名技能且不属本库管理，已跳过".to_string(),
            });
        }
        if let Some(index) = known {
            if destination_exists {
                ensure_tree_has_no_symlinks(&destination)?;
                if fingerprint(&destination)? != state.skills[index].files {
                    return Ok(SkillActionOutcome {
                        name: name.to_string(),
                        target_id: target_id.to_string(),
                        ok: false,
                        message: "内容已变更，拒绝覆盖".to_string(),
                    });
                }
            }
        }
        let files = fingerprint(&source)?;
        let skills_root = skill_install_dir(target_home);
        ensure_no_symlink_below(target_home, Path::new("skills"))?;
        fs::create_dir_all(&skills_root).context("无法创建目标技能目录")?;
        ensure_no_symlink_below(target_home, Path::new("skills"))?;
        let stage = TemporaryDirectory::create(&skills_root, ".ccp-skill-install")?;
        copy_tree(&source, stage.path())?;

        let record = InstalledSkill {
            name: name.to_string(),
            target_home: target_home.to_string_lossy().to_string(),
            installed_at_unix: now_unix(),
            files,
        };
        match known {
            Some(index) => state.skills[index] = record,
            None => state.skills.push(record),
        }
        let backup = publish_directory(stage.path(), &destination)?;
        if let Err(error) = self.write_state(&state) {
            let mut rollback_errors = Vec::new();
            if let Err(restore_error) = atomic_write(&self.state_path, &state_before) {
                rollback_errors.push(format!("恢复状态失败：{restore_error}"));
            }
            if let Err(rollback_error) =
                rollback_published_directory(&destination, backup.as_deref())
            {
                rollback_errors.push(format!("恢复目标目录失败：{rollback_error}"));
            }
            if rollback_errors.is_empty() {
                return Err(error).context("保存技能状态失败，已回滚目标目录与状态");
            }
            return Err(error).context(format!("保存技能状态失败；{}", rollback_errors.join("；")));
        }
        if let Some(path) = backup {
            let _ = fs::remove_dir_all(path);
        }
        Ok(SkillActionOutcome {
            name: name.to_string(),
            target_id: target_id.to_string(),
            ok: true,
            message: format!("已安装到 {}", destination.display()),
        })
    }

    /// 从目标客户端卸载技能。指纹已变化时拒绝，避免删除用户改动过的内容。
    pub fn uninstall_skill(
        &self,
        target_id: &str,
        target_home: &Path,
        name: &str,
    ) -> anyhow::Result<SkillActionOutcome> {
        validate_name_component(name)?;
        let mut state = self.read_state()?;
        let home = target_home.to_string_lossy().to_string();
        let Some(index) = state
            .skills
            .iter()
            .position(|skill| skill.name == name && skill.target_home == home)
        else {
            return Ok(SkillActionOutcome {
                name: name.to_string(),
                target_id: target_id.to_string(),
                ok: false,
                message: "不在本库记录内，未做改动".to_string(),
            });
        };
        let record = state.skills[index].clone();
        let path = skill_install_dir(target_home).join(&record.name);
        ensure_no_symlink_below(target_home, &Path::new("skills").join(&record.name))?;
        let installed_path_exists = path_exists(&path)?;
        if installed_path_exists {
            ensure_tree_has_no_symlinks(&path)?;
            if fingerprint(&path)? != record.files {
                return Ok(SkillActionOutcome {
                    name: name.to_string(),
                    target_id: target_id.to_string(),
                    ok: false,
                    message: "内容已变更，拒绝删除".to_string(),
                });
            }
        }
        let state_before = fs::read(&self.state_path).context("无法备份技能库状态")?;
        state.skills.remove(index);
        let backup = if installed_path_exists {
            let backup = backup_path(path.parent().unwrap_or(target_home), ".ccp-skill-uninstall");
            fs::rename(&path, &backup).with_context(|| format!("无法暂存技能 {name}"))?;
            Some(backup)
        } else {
            None
        };
        if let Err(error) = self.write_state(&state) {
            let mut rollback_errors = Vec::new();
            if let Err(restore_error) = atomic_write(&self.state_path, &state_before) {
                rollback_errors.push(format!("恢复状态失败：{restore_error}"));
            }
            if let Some(backup) = &backup {
                if let Err(restore_error) = fs::rename(backup, &path) {
                    rollback_errors.push(format!("恢复技能目录失败：{restore_error}"));
                }
            }
            if rollback_errors.is_empty() {
                return Err(error).context("保存技能状态失败，已恢复技能目录与状态");
            }
            return Err(error).context(format!("保存技能状态失败；{}", rollback_errors.join("；")));
        }
        if let Some(backup) = backup {
            let _ = fs::remove_dir_all(backup);
        }
        Ok(SkillActionOutcome {
            name: name.to_string(),
            target_id: target_id.to_string(),
            ok: true,
            message: "已卸载".to_string(),
        })
    }

    fn read_state(&self) -> anyhow::Result<LibraryState> {
        ensure_no_symlink(&self.state_path)?;
        let raw = fs::read_to_string(&self.state_path).context("无法读取技能库状态文件")?;
        serde_json::from_str(&raw).context("无法解析技能库状态文件")
    }

    fn write_state(&self, state: &LibraryState) -> anyhow::Result<()> {
        #[cfg(test)]
        if self
            .fail_next_state_write
            .swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            bail!("测试注入的状态写入错误");
        }
        ensure_no_symlink(&self.state_path)?;
        atomic_write(&self.state_path, &serde_json::to_vec_pretty(state)?)
    }

    #[cfg(test)]
    fn fail_next_state_write(&self) {
        self.fail_next_state_write
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

fn fingerprint(dir: &Path) -> anyhow::Result<BTreeMap<String, String>> {
    ensure_tree_has_no_symlinks(dir)?;
    let mut files = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in
            fs::read_dir(&current).with_context(|| format!("无法读取目录 {}", current.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                bail!("拒绝对包含符号链接的技能计算指纹：{}", path.display());
            } else if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() {
                let relative = path
                    .strip_prefix(dir)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                let bytes =
                    fs::read(&path).with_context(|| format!("无法读取 {}", path.display()))?;
                files.insert(relative, sha256_hex(&bytes));
            }
        }
    }
    Ok(files)
}

fn copy_tree(source: &Path, destination: &Path) -> anyhow::Result<()> {
    ensure_tree_has_no_symlinks(source)?;
    ensure_no_symlink(destination)?;
    fs::create_dir_all(destination).context("无法创建技能目录")?;
    for entry in fs::read_dir(source).with_context(|| format!("无法读取 {}", source.display()))?
    {
        let entry = entry?;
        let from = entry.path();
        let to = destination.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            bail!("拒绝复制符号链接：{}", from.display());
        } else if file_type.is_dir() {
            copy_tree(&from, &to)?;
        } else if file_type.is_file() {
            let bytes = fs::read(&from).with_context(|| format!("无法读取 {}", from.display()))?;
            atomic_write(&to, &bytes)?;
        } else {
            bail!("拒绝复制特殊文件：{}", from.display());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library() -> (tempfile::TempDir, PromptLibrary) {
        let dir = tempfile::tempdir().unwrap();
        let library = PromptLibrary::open(dir.path().join("cache")).unwrap();
        (dir, library)
    }

    /// 在缓存目录里放一个可用技能，模拟已下载状态。
    fn stage_skill(library: &PromptLibrary, name: &str, files: &[(&str, &str)]) {
        let root = library.cache_dir().join("skills").join(name);
        for (relative, body) in files {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, body).unwrap();
        }
    }

    fn skill_entry(name: &str, files: &[&str]) -> SkillEntry {
        SkillEntry {
            name: name.to_string(),
            description: String::new(),
            path: format!("skills/{name}"),
            files: files.iter().map(|file| file.to_string()).collect(),
        }
    }

    fn tool_entry(id: &str, version: &str, files: &[(&str, &[u8])]) -> ToolEntry {
        let files = files
            .iter()
            .map(|(path, bytes)| ToolFileEntry {
                path: format!("tools/{id}/{path}"),
                size: bytes.len() as u64,
                sha256: sha256_hex(bytes),
            })
            .collect::<Vec<_>>();
        ToolEntry {
            id: id.to_string(),
            title: id.to_string(),
            version: version.to_string(),
            description: "Test tool".to_string(),
            source_repo: "https://github.com/example/tool".to_string(),
            source_revision: "a".repeat(40),
            license_id: "MIT".to_string(),
            license_path: format!("tools/{id}/LICENSE"),
            platforms: vec!["windows".to_string()],
            size: files.iter().map(|file| file.size).sum(),
            files,
        }
    }

    fn stage_tool(library: &PromptLibrary, entry: &ToolEntry, bodies: &[(&str, &str)]) {
        let root = library
            .cache_dir()
            .join("tools")
            .join(&entry.id)
            .join(&entry.version);
        for (relative, body) in bodies {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, body).unwrap();
        }
    }

    #[test]
    fn index_parses_valid_manifest() {
        let raw = r#"{
          "version": 1,
          "prompts": [{"id":"a","title":"A","category":"通用","description":"","path":"prompts/a.md"}],
          "skills": [{"name":"s","description":"","path":"skills/s","files":["skills/s/SKILL.md"]}]
        }"#;
        let index = parse_index(raw).unwrap();
        assert_eq!(index.prompts.len(), 1);
        assert_eq!(index.skills.len(), 1);
        assert_eq!(index.prompts[0].id, "a");
    }

    #[tokio::test]
    async fn local_project_is_used_before_remote_sources() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("Prompt");
        fs::create_dir_all(project.join("prompts")).unwrap();
        fs::write(
            project.join("index.json"),
            r#"{"version":1,"prompts":[{"id":"local","title":"本地","path":"prompts/local.md"}],"skills":[]}"#,
        )
        .unwrap();
        fs::write(project.join("prompts/local.md"), "来自本地项目\n").unwrap();

        let library = PromptLibrary::open_with_project(dir.path().join("cache"), &project).unwrap();
        let index = library.fetch_index().await.unwrap();
        let path = library.fetch_prompt(&index.prompts[0]).await.unwrap();

        assert_eq!(index.prompts[0].id, "local");
        assert_eq!(fs::read_to_string(path).unwrap(), "来自本地项目\n");
        assert_eq!(library.project_dir(), project.as_path());
    }

    #[tokio::test]
    async fn local_skill_and_tool_files_are_cached_without_remote_access() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("Prompt");
        fs::create_dir_all(project.join("skills/local")).unwrap();
        fs::create_dir_all(project.join("tools/tool")).unwrap();
        fs::write(project.join("skills/local/SKILL.md"), "本地技能\n").unwrap();
        fs::write(project.join("tools/tool/LICENSE"), "MIT").unwrap();
        fs::write(project.join("tools/tool/main.py"), "print(1)\n").unwrap();
        let tool = tool_entry(
            "tool",
            "1.0",
            &[("LICENSE", b"MIT"), ("main.py", b"print(1)\n")],
        );
        let library = PromptLibrary::open_with_project(dir.path().join("cache"), &project).unwrap();
        let skill = skill_entry("local", &["skills/local/SKILL.md"]);

        let skill_path = library.fetch_skill(&skill).await.unwrap();
        let tool_path = library.fetch_tool(&tool).await.unwrap();

        assert_eq!(
            fs::read_to_string(skill_path.join("SKILL.md")).unwrap(),
            "本地技能\n"
        );
        assert_eq!(
            fs::read_to_string(tool_path.join("main.py")).unwrap(),
            "print(1)\n"
        );
    }

    #[test]
    fn index_rejects_prompt_path_outside_prompts_dir() {
        let raw = r#"{"version":1,"prompts":[{"id":"a","path":"../secret.md"}],"skills":[]}"#;
        let error = parse_index(raw).unwrap_err().to_string();
        assert!(error.contains("路径不合法"), "实际：{error}");
    }

    #[test]
    fn index_rejects_skill_without_entry_file() {
        let raw = r#"{"version":1,"prompts":[],"skills":[{"name":"s","path":"skills/s","files":["skills/s/readme.txt"]}]}"#;
        let error = parse_index(raw).unwrap_err().to_string();
        assert!(error.contains("SKILL.md"), "实际：{error}");
    }

    #[test]
    fn index_rejects_skill_file_escaping_its_directory() {
        let raw = r#"{"version":1,"prompts":[],"skills":[{"name":"s","path":"skills/s","files":["skills/other/SKILL.md"]}]}"#;
        let error = parse_index(raw).unwrap_err().to_string();
        assert!(error.contains("越界"), "实际：{error}");
    }

    #[test]
    fn index_rejects_malformed_json() {
        assert!(parse_index("{").is_err());
    }

    #[test]
    fn install_copies_skill_and_records_fingerprint() {
        let (dir, library) = library();
        stage_skill(
            &library,
            "alpha",
            &[("SKILL.md", "---\nname: alpha\n---\n正文\n")],
        );
        let home = dir.path().join("client");
        fs::create_dir_all(&home).unwrap();

        let outcome = library.install_skill("codex", &home, "alpha").unwrap();
        assert!(outcome.ok, "{outcome:?}");
        assert!(skill_install_dir(&home).join("alpha/SKILL.md").is_file());

        let installed = library.list_installed().unwrap();
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].name, "alpha");
        assert!(installed[0].files.contains_key("SKILL.md"));
    }

    #[test]
    fn install_refuses_user_owned_directory_with_same_name() {
        let (dir, library) = library();
        stage_skill(&library, "alpha", &[("SKILL.md", "来自缓存\n")]);
        let home = dir.path().join("client");
        let existing = skill_install_dir(&home).join("alpha");
        fs::create_dir_all(&existing).unwrap();
        fs::write(existing.join("SKILL.md"), "用户自己的\n").unwrap();

        let outcome = library.install_skill("codex", &home, "alpha").unwrap();
        assert!(!outcome.ok, "不得覆盖用户自有目录");
        assert!(outcome.message.contains("已跳过"));
        assert_eq!(
            fs::read_to_string(existing.join("SKILL.md")).unwrap(),
            "用户自己的\n"
        );
    }

    #[test]
    fn install_without_downloaded_content_fails() {
        let (dir, library) = library();
        let home = dir.path().join("client");
        fs::create_dir_all(&home).unwrap();
        let error = library
            .install_skill("codex", &home, "ghost")
            .unwrap_err()
            .to_string();
        assert!(error.contains("尚未下载"), "实际：{error}");
    }

    #[test]
    fn reinstall_replaces_managed_skill_without_duplicate_records() {
        let (dir, library) = library();
        stage_skill(&library, "alpha", &[("SKILL.md", "第一版\n")]);
        let home = dir.path().join("client");
        fs::create_dir_all(&home).unwrap();
        library.install_skill("codex", &home, "alpha").unwrap();

        stage_skill(&library, "alpha", &[("SKILL.md", "第二版\n")]);
        let outcome = library.install_skill("codex", &home, "alpha").unwrap();
        assert!(outcome.ok);
        assert_eq!(
            library.list_installed().unwrap().len(),
            1,
            "不得产生重复记录"
        );
        assert_eq!(
            fs::read_to_string(skill_install_dir(&home).join("alpha/SKILL.md")).unwrap(),
            "第二版\n"
        );
    }

    #[test]
    fn uninstall_removes_only_managed_skill() {
        let (dir, library) = library();
        stage_skill(&library, "alpha", &[("SKILL.md", "正文\n")]);
        let home = dir.path().join("client");
        fs::create_dir_all(&home).unwrap();
        library.install_skill("codex", &home, "alpha").unwrap();

        let outcome = library.uninstall_skill("codex", &home, "alpha").unwrap();
        assert!(outcome.ok, "{outcome:?}");
        assert!(!skill_install_dir(&home).join("alpha").exists());
        assert!(library.list_installed().unwrap().is_empty());
    }

    #[test]
    fn uninstall_refuses_skill_whose_content_changed() {
        let (dir, library) = library();
        stage_skill(&library, "alpha", &[("SKILL.md", "正文\n")]);
        let home = dir.path().join("client");
        fs::create_dir_all(&home).unwrap();
        library.install_skill("codex", &home, "alpha").unwrap();
        fs::write(skill_install_dir(&home).join("alpha/SKILL.md"), "被改过\n").unwrap();

        let outcome = library.uninstall_skill("codex", &home, "alpha").unwrap();
        assert!(!outcome.ok);
        assert!(outcome.message.contains("内容已变更"));
        assert!(skill_install_dir(&home).join("alpha/SKILL.md").is_file());
    }

    #[test]
    fn uninstall_reports_unmanaged_skill_and_touches_nothing() {
        let (dir, library) = library();
        let home = dir.path().join("client");
        let user_skill = skill_install_dir(&home).join("user-made");
        fs::create_dir_all(&user_skill).unwrap();
        fs::write(user_skill.join("SKILL.md"), "用户自己的\n").unwrap();

        let outcome = library
            .uninstall_skill("codex", &home, "user-made")
            .unwrap();
        assert!(!outcome.ok);
        assert!(outcome.message.contains("不在本库记录内"));
        assert!(
            user_skill.join("SKILL.md").is_file(),
            "清单外目录不得被删除"
        );
    }

    #[test]
    fn install_tracks_nested_files_and_per_target_records() {
        let (dir, library) = library();
        stage_skill(
            &library,
            "alpha",
            &[("SKILL.md", "正文\n"), ("references/deep.md", "深层\n")],
        );
        let codex_home = dir.path().join("codex");
        let claude_home = dir.path().join("claude");
        fs::create_dir_all(&codex_home).unwrap();
        fs::create_dir_all(&claude_home).unwrap();

        library
            .install_skill("codex", &codex_home, "alpha")
            .unwrap();
        library
            .install_skill("claude-code", &claude_home, "alpha")
            .unwrap();

        assert!(
            skill_install_dir(&codex_home)
                .join("alpha/references/deep.md")
                .is_file()
        );
        assert!(
            skill_install_dir(&claude_home)
                .join("alpha/references/deep.md")
                .is_file()
        );
        let installed = library.list_installed().unwrap();
        assert_eq!(installed.len(), 2, "同一技能在不同目标各自成记录");
        assert!(
            installed
                .iter()
                .all(|skill| skill.files.contains_key("references/deep.md"))
        );
    }

    #[test]
    fn skill_entry_files_must_include_entry_within_its_path() {
        let entry = skill_entry(
            "alpha",
            &["skills/alpha/SKILL.md", "skills/alpha/references/x.md"],
        );
        assert!(
            entry
                .files
                .iter()
                .all(|file| file.starts_with("skills/alpha/"))
        );
    }

    #[test]
    fn prompt_entries_filter_by_target_and_reject_unknown() {
        let raw = r#"{
          "version": 1,
          "prompts": [
            {"id":"a","title":"A","path":"prompts/a.md","targets":["codex"]},
            {"id":"b","title":"B","path":"prompts/b.md","targets":["zcode","cursor"]},
            {"id":"c","title":"C","path":"prompts/c.md"}
          ],
          "skills": []
        }"#;
        let index = parse_index(raw).unwrap();

        let codex: Vec<&str> = PromptLibrary::prompts_for_target(&index, "codex")
            .iter()
            .map(|entry| entry.id.as_str())
            .collect();
        assert_eq!(codex, vec!["a", "c"], "空 targets 视为适配全部");

        let zcode: Vec<&str> = PromptLibrary::prompts_for_target(&index, "zcode")
            .iter()
            .map(|entry| entry.id.as_str())
            .collect();
        assert_eq!(zcode, vec!["b", "c"]);
        assert!(!zcode.contains(&"a"), "codex 专用条目不得投给 zcode");
    }

    #[test]
    fn prompt_entry_with_unknown_target_is_rejected() {
        let raw = r#"{"version":1,"prompts":[{"id":"a","path":"prompts/a.md","targets":["nope"]}],"skills":[]}"#;
        let error = parse_index(raw).unwrap_err().to_string();
        assert!(error.contains("不在支持列表内"), "实际：{error}");
    }

    #[test]
    fn prompt_entry_version_is_parsed() {
        let raw = r#"{"version":1,"prompts":[{"id":"a","path":"prompts/a.md","version":"V5","targets":[]}],"skills":[]}"#;
        let index = parse_index(raw).unwrap();
        assert_eq!(index.prompts[0].version, "V5");
    }

    #[test]
    fn tool_manifest_requires_license_hash_size_and_scoped_paths() {
        let valid = tool_entry(
            "tool",
            "1.0",
            &[("LICENSE", b"MIT"), ("main.py", b"print(1)")],
        );
        validate_tool_entry(&valid).unwrap();

        let mut invalid = valid.clone();
        invalid.files[1].path = "tools/other/main.py".to_string();
        assert!(validate_tool_entry(&invalid).is_err());

        let mut invalid = valid.clone();
        invalid.files[1].sha256 = "bad".to_string();
        assert!(validate_tool_entry(&invalid).is_err());

        let mut invalid = valid;
        invalid.size += 1;
        assert!(validate_tool_entry(&invalid).is_err());
    }

    #[test]
    fn tool_install_and_uninstall_are_fingerprint_managed() {
        let (_dir, library) = library();
        let entry = tool_entry(
            "tool",
            "1.0",
            &[("LICENSE", b"MIT"), ("main.py", b"print(1)")],
        );
        stage_tool(
            &library,
            &entry,
            &[("LICENSE", "MIT"), ("main.py", "print(1)")],
        );

        let outcome = library.install_tool(&entry).unwrap();
        assert!(outcome.ok);
        assert_eq!(
            Path::new(&outcome.install_path),
            library.tool_root.join("tool/1.0")
        );
        assert!(Path::new(&outcome.install_path).join("main.py").is_file());
        assert_eq!(library.list_installed_tools().unwrap().len(), 1);
        assert!(library.install_tool(&entry).unwrap().ok, "重试不应重复安装");

        fs::write(
            Path::new(&outcome.install_path).join("main.py"),
            "externally edited",
        )
        .unwrap();
        assert!(library.uninstall_tool("tool", "1.0").is_err());
        assert!(Path::new(&outcome.install_path).join("main.py").is_file());
        assert_eq!(library.list_installed_tools().unwrap().len(), 1);
    }

    #[test]
    fn tool_state_write_failure_rolls_back_install_directory() {
        let (_dir, library) = library();
        let entry = tool_entry(
            "tool",
            "1.0",
            &[("LICENSE", b"MIT"), ("main.py", b"print(1)")],
        );
        stage_tool(
            &library,
            &entry,
            &[("LICENSE", "MIT"), ("main.py", "print(1)")],
        );
        library.fail_next_state_write();

        assert!(library.install_tool(&entry).is_err());
        assert!(!library.tool_root.join("tool/1.0").exists());
        assert!(library.list_installed_tools().unwrap().is_empty());
    }
}
