//! 多客户端提示词投放：目标注册表、投放、可逆还原。
//!
//! 覆盖本机 7 个客户端的公开扩展点。Codex 走既有 `SystemPromptStore`
//! （配置行 + 受管文件），其余客户端写入全局指令文件，用受管标记块包裹，
//! 块外用户自有内容一律不动。
//!
//! 正文由调用方提供；本模块不内置任何提示词内容。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::settings::atomic_write;

const BEGIN_MARK: &str = "<!-- CCP-INJECT:BEGIN -->";
const END_MARK: &str = "<!-- CCP-INJECT:END -->";
const MAX_CONTENT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InjectionKind {
    /// 独立受管文件：配置行指向该文件（仅 Codex）。
    ManagedFile,
    /// 写入客户端全局指令文件，用受管标记块包裹。
    MarkedBlock,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClientTarget {
    pub id: String,
    pub display_name: String,
    /// 数据目录候选，按优先级排列。
    pub home_candidates: Vec<String>,
    /// 需要设置的配置键；`MarkedBlock` 为 `None`。
    pub config_key: Option<String>,
    /// 配置文件相对数据目录的路径。
    pub config_relative: Option<String>,
    pub kind: InjectionKind,
    /// 受管文件名（`ManagedFile`）或指令文件相对路径（`MarkedBlock`）。
    pub instruction_path: String,
}

fn home_dir() -> String {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default()
}

/// 七个本地客户端。写入的均是各客户端公开的全局指令位置。
pub fn targets() -> Vec<ClientTarget> {
    let home = home_dir();
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    vec![
        ClientTarget {
            id: "codex".into(),
            display_name: "Codex".into(),
            home_candidates: vec![format!("{home}/.codex")],
            config_key: Some("model_instructions_file".into()),
            config_relative: Some("config.toml".into()),
            kind: InjectionKind::ManagedFile,
            instruction_path: "ccp-client-prompt.md".into(),
        },
        ClientTarget {
            id: "claude-code".into(),
            display_name: "Claude Code".into(),
            home_candidates: vec![format!("{home}/.claude")],
            config_key: None,
            config_relative: None,
            kind: InjectionKind::MarkedBlock,
            instruction_path: "CLAUDE.md".into(),
        },
        ClientTarget {
            id: "deepseek-harness".into(),
            display_name: "DeepSeek Harness".into(),
            home_candidates: vec![format!("{home}/.dsh")],
            config_key: None,
            config_relative: None,
            kind: InjectionKind::MarkedBlock,
            instruction_path: "AGENTS.md".into(),
        },
        ClientTarget {
            id: "zcode".into(),
            display_name: "ZCode".into(),
            home_candidates: vec![format!("{home}/.zcode")],
            config_key: None,
            config_relative: None,
            kind: InjectionKind::MarkedBlock,
            instruction_path: "AGENTS.md".into(),
        },
        ClientTarget {
            id: "workbuddy".into(),
            display_name: "WorkBuddy 国际版".into(),
            home_candidates: vec![format!("{home}/.workbuddy-ai")],
            config_key: None,
            config_relative: None,
            kind: InjectionKind::MarkedBlock,
            instruction_path: "MEMORY.md".into(),
        },
        ClientTarget {
            id: "workbuddy-cn".into(),
            display_name: "WorkBuddy 国内版".into(),
            home_candidates: vec![format!("{home}/.workbuddy")],
            config_key: None,
            config_relative: None,
            kind: InjectionKind::MarkedBlock,
            instruction_path: "MEMORY.md".into(),
        },
        ClientTarget {
            id: "cursor".into(),
            display_name: "Cursor".into(),
            home_candidates: vec![
                format!("{appdata}/Cursor/User/rules"),
                format!("{home}/.cursor/rules"),
            ],
            config_key: None,
            config_relative: None,
            kind: InjectionKind::MarkedBlock,
            instruction_path: "ccp-user-rules.md".into(),
        },
    ]
}

pub fn find_target(id: &str) -> Option<ClientTarget> {
    targets().into_iter().find(|target| target.id == id)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TargetStatus {
    pub target_id: String,
    pub display_name: String,
    pub installed: bool,
    pub home: Option<String>,
    /// 投放将写入的绝对路径；未检测到目标时为 `None`。
    pub planned_path: Option<String>,
    pub managed: bool,
    pub baseline_exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentRecord {
    pub target_id: String,
    pub kind: InjectionKind,
    pub instruction_path: String,
    /// Canonical client data directory captured when the deployment was written.
    #[serde(default)]
    pub home_path: String,
    /// Filesystem identity prevents restoring into a replacement directory at the same path.
    #[serde(default)]
    pub home_identity: String,
    pub baseline_path: String,
    /// 投放前指令文件不存在；`ManagedFile` 还原时应删除该文件。
    pub baseline_empty: bool,
    /// 注入正文的 SHA-256，用于识别外部修改。
    pub content_sha256: String,
    pub config_path: Option<String>,
    pub config_key: Option<String>,
    pub previous_config: Option<String>,
    pub config_line_added: bool,
    pub deployed_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct DeploymentState {
    #[serde(default)]
    records: Vec<DeploymentRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOutcome {
    pub target_id: String,
    pub restored_path: Option<String>,
    pub removed_path: Option<String>,
    /// 兜底清扫结果，独立于主流程。
    pub sweep_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeployOutcome {
    pub target_id: String,
    pub ok: bool,
    pub message: String,
    pub record: Option<DeploymentRecord>,
    /// 该目标当前是否受管（用于批量投放时的跳过说明）。
    pub managed: bool,
    /// 投放后实际写入的路径。
    pub applied_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RestoreReport {
    pub target_id: String,
    pub ok: bool,
    pub message: String,
    pub restored_path: Option<String>,
    pub removed_path: Option<String>,
    /// 兜底清扫结果，独立于主流程。
    pub sweep_message: String,
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

fn marked(content: &str) -> String {
    format!("{BEGIN_MARK}\n{}\n{END_MARK}", content.trim_end())
}

fn marked_bounds(text: &str) -> anyhow::Result<Option<(usize, usize)>> {
    let starts: Vec<usize> = text
        .match_indices(BEGIN_MARK)
        .map(|(index, _)| index)
        .collect();
    let ends: Vec<usize> = text
        .match_indices(END_MARK)
        .map(|(index, _)| index)
        .collect();
    if starts.is_empty() && ends.is_empty() {
        return Ok(None);
    }
    if starts.len() != 1 || ends.len() != 1 {
        bail!("投放标记损坏或重复，拒绝修改指令文件");
    }
    let body_start = starts[0] + BEGIN_MARK.len();
    if ends[0] <= body_start {
        bail!("投放标记顺序错误，拒绝修改指令文件");
    }
    Ok(Some((starts[0], ends[0])))
}

/// 取出受管标记内的正文。
fn extract_marked(text: &str) -> anyhow::Result<Option<String>> {
    let Some((start, end)) = marked_bounds(text)? else {
        return Ok(None);
    };
    let body_start = start + BEGIN_MARK.len();
    let mut body = &text[body_start..end];
    body = body
        .strip_prefix("\r\n")
        .or_else(|| body.strip_prefix('\n'))
        .unwrap_or(body);
    body = body
        .strip_suffix("\r\n")
        .or_else(|| body.strip_suffix('\n'))
        .unwrap_or(body);
    Ok(Some(body.to_string()))
}

/// 摘除受管块，保留块外用户正文。
fn strip_marked(text: &str) -> anyhow::Result<Option<String>> {
    let Some((start, end)) = marked_bounds(text)? else {
        return Ok(None);
    };
    let mut out = String::with_capacity(text.len());
    out.push_str(&text[..start]);
    out.push_str(&text[end + END_MARK.len()..]);
    while out.contains("\n\n\n") {
        out = out.replace("\n\n\n", "\n\n");
    }
    Ok(Some(out.trim_end().to_string()))
}

#[derive(Clone)]
struct FileMutation {
    path: PathBuf,
    contents: Option<Vec<u8>>,
}

fn reject_symlink(path: &Path) -> anyhow::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!("受管路径已变为符号链接，拒绝修改：{}", path.display())
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("无法检查路径：{}", path.display())),
    }
}

fn apply_file_mutations(mutations: &[FileMutation]) -> anyhow::Result<()> {
    let mut snapshots = Vec::with_capacity(mutations.len());
    for mutation in mutations {
        reject_symlink(&mutation.path)?;
        let snapshot = match fs::read(&mutation.path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("无法读取：{}", mutation.path.display()));
            }
        };
        snapshots.push(snapshot);
    }

    for (index, mutation) in mutations.iter().enumerate() {
        let result = match &mutation.contents {
            Some(contents) => atomic_write(&mutation.path, contents),
            None => match fs::remove_file(&mutation.path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error.into()),
            },
        };
        if let Err(error) = result {
            let mut rollback_errors = Vec::new();
            for rollback_index in (0..=index).rev() {
                let path = &mutations[rollback_index].path;
                let rollback = match &snapshots[rollback_index] {
                    Some(contents) => atomic_write(path, contents),
                    None => match fs::remove_file(path) {
                        Ok(()) => Ok(()),
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                        Err(error) => Err(error.into()),
                    },
                };
                if let Err(rollback_error) = rollback {
                    rollback_errors.push(format!("{}: {rollback_error}", path.display()));
                }
            }
            if rollback_errors.is_empty() {
                return Err(error).context(format!("写入失败，已回滚 {} 项文件", index + 1));
            }
            bail!(
                "写入失败：{error}；回滚也失败：{}",
                rollback_errors.join("; ")
            );
        }
    }
    Ok(())
}

fn canonical_home_identity(home: &Path) -> anyhow::Result<(PathBuf, String)> {
    let canonical = fs::canonicalize(home)
        .with_context(|| format!("无法解析客户端目录：{}", home.display()))?;
    let metadata = fs::metadata(&canonical)
        .with_context(|| format!("无法读取客户端目录：{}", canonical.display()))?;
    if !metadata.is_dir() {
        bail!("客户端路径不是目录：{}", canonical.display());
    }
    #[cfg(unix)]
    let identity = {
        use std::os::unix::fs::MetadataExt;
        format!("{}:{}", metadata.dev(), metadata.ino())
    };
    #[cfg(windows)]
    let identity = format!(
        "{}:{}",
        canonical.to_string_lossy(),
        metadata
            .created()
            .context("无法读取客户端目录创建时间")?
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    #[cfg(not(any(unix, windows)))]
    let identity = canonical.to_string_lossy().to_string();
    Ok((canonical, identity))
}

pub struct DeploymentEngine {
    root: PathBuf,
    state_path: PathBuf,
}

impl DeploymentEngine {
    pub fn open(root: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root).context("无法创建投放存储目录")?;
        let engine = Self {
            state_path: root.join("state.json"),
            root,
        };
        match fs::read(&engine.state_path) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                engine.write_state(&DeploymentState::default())?;
            }
            Err(error) => return Err(error).context("无法检查投放状态文件"),
        }
        Ok(engine)
    }

    pub fn open_default() -> anyhow::Result<Self> {
        Self::open(crate::paths::default_app_state_dir().join("client-deploy"))
    }

    pub fn statuses(&self) -> anyhow::Result<Vec<TargetStatus>> {
        let state = self.read_state()?;
        Ok(targets()
            .into_iter()
            .map(|target| {
                let installed = self.resolve(&target);
                let record = state
                    .records
                    .iter()
                    .find(|record| record.target_id == target.id);
                TargetStatus {
                    target_id: target.id.clone(),
                    display_name: target.display_name.clone(),
                    installed: installed.is_some(),
                    planned_path: installed
                        .as_ref()
                        .map(|home| home.join(&target.instruction_path))
                        .map(|path| path.to_string_lossy().to_string()),
                    home: installed.map(|home| home.to_string_lossy().to_string()),
                    managed: record.is_some(),
                    baseline_exists: record
                        .map(|record| Path::new(&record.baseline_path).exists())
                        .unwrap_or(false),
                }
            })
            .collect())
    }

    /// 探测目标数据目录。候选均不存在时返回 `None`，不创建目录。
    pub fn resolve(&self, target: &ClientTarget) -> Option<PathBuf> {
        target
            .home_candidates
            .iter()
            .map(PathBuf::from)
            .find(|path| path.is_dir())
    }

    /// 投放到注册表中的给定目标；逐个执行，单个失败不影响其余目标。
    ///
    /// 已受管且内容未变化的目标会跳过，避免重复投放刷新部署时间。
    pub fn deploy_many(&self, target_ids: &[String], content: &str) -> Vec<DeployOutcome> {
        let resolved: Vec<(ClientTarget, PathBuf)> = target_ids
            .iter()
            .filter_map(|target_id| match find_target(target_id) {
                Some(target) => self.resolve(&target).map(|home| (target, home)),
                None => None,
            })
            .collect();
        let settled = self.deploy_resolved(&resolved, content);

        // 未解析出目录的目标（未安装或未注册）单独报告，不影响已解析目标的结果。
        target_ids
            .iter()
            .map(|target_id| {
                settled
                    .iter()
                    .find(|outcome| &outcome.target_id == target_id)
                    .cloned()
                    .unwrap_or_else(|| DeployOutcome {
                        target_id: target_id.clone(),
                        ok: false,
                        managed: false,
                        applied_path: None,
                        message: match find_target(target_id) {
                            Some(target) => format!("未检测到 {}，无法投放", target.display_name),
                            None => format!("未知投放目标 {target_id}"),
                        },
                        record: None,
                    })
            })
            .collect()
    }

    /// 逐目标投放各自的内容。每个客户端内容不同，因此不做「一份正文投所有目标」。
    /// 单个目标失败不影响其余目标。
    pub fn deploy_assignments(&self, assignments: &[(String, String)]) -> Vec<DeployOutcome> {
        assignments
            .iter()
            .map(
                |(target_id, content)| match self.deploy(target_id, content) {
                    Ok(record) => DeployOutcome {
                        target_id: target_id.clone(),
                        ok: true,
                        managed: true,
                        applied_path: Some(record.instruction_path.clone()),
                        message: format!("已投放到 {}", record.instruction_path),
                        record: Some(record),
                    },
                    Err(error) => DeployOutcome {
                        target_id: target_id.clone(),
                        ok: false,
                        managed: self.record_for(target_id).ok().flatten().is_some(),
                        applied_path: self
                            .record_for(target_id)
                            .ok()
                            .flatten()
                            .map(|record| record.instruction_path),
                        message: error.to_string(),
                        record: None,
                    },
                },
            )
            .collect()
    }

    /// 对已解析出的 (目标, 数据目录) 批量投放。测试与内部复用此入口。
    pub fn deploy_resolved(
        &self,
        items: &[(ClientTarget, PathBuf)],
        content: &str,
    ) -> Vec<DeployOutcome> {
        let wanted = sha256_hex(content.trim_end().as_bytes());
        items
            .iter()
            .map(|(target, home)| {
                let existing = match self.record_for(&target.id) {
                    Ok(record) => record,
                    Err(error) => {
                        return DeployOutcome {
                            target_id: target.id.clone(),
                            ok: false,
                            managed: false,
                            applied_path: None,
                            message: error.to_string(),
                            record: None,
                        };
                    }
                };
                if existing.as_ref().is_some_and(|record| {
                    record.content_sha256 == wanted
                        && self.current_deployment_matches(record, target, home)
                }) {
                    let record = existing.as_ref().expect("checked above");
                    return DeployOutcome {
                        target_id: target.id.clone(),
                        ok: true,
                        managed: true,
                        applied_path: Some(record.instruction_path.clone()),
                        message: "内容与当前一致，已跳过".to_string(),
                        record: None,
                    };
                }
                match self.deploy_to(target, home, content) {
                    Ok(record) => DeployOutcome {
                        target_id: target.id.clone(),
                        ok: true,
                        managed: true,
                        applied_path: Some(record.instruction_path.clone()),
                        message: format!("已投放到 {}", record.instruction_path),
                        record: Some(record),
                    },
                    Err(error) => DeployOutcome {
                        target_id: target.id.clone(),
                        ok: false,
                        managed: existing.is_some(),
                        applied_path: existing.map(|record| record.instruction_path),
                        message: error.to_string(),
                        record: None,
                    },
                }
            })
            .collect()
    }

    fn record_for(&self, target_id: &str) -> anyhow::Result<Option<DeploymentRecord>> {
        Ok(self
            .read_state()?
            .records
            .into_iter()
            .find(|record| record.target_id == target_id))
    }

    fn current_deployment_matches(
        &self,
        record: &DeploymentRecord,
        target: &ClientTarget,
        home: &Path,
    ) -> bool {
        let Ok((canonical_home, identity)) = canonical_home_identity(home) else {
            return false;
        };
        let instruction_path = home.join(&target.instruction_path);
        if record.instruction_path != instruction_path.to_string_lossy()
            || (!record.home_path.is_empty()
                && (record.home_path != canonical_home.to_string_lossy()
                    || record.home_identity != identity))
            || reject_symlink(&instruction_path).is_err()
        {
            return false;
        }
        let Ok(current) = fs::read_to_string(&instruction_path) else {
            return false;
        };
        let content_matches = match record.kind {
            InjectionKind::MarkedBlock => extract_marked(&current)
                .ok()
                .flatten()
                .is_some_and(|content| sha256_hex(content.as_bytes()) == record.content_sha256),
            InjectionKind::ManagedFile => {
                sha256_hex(current.trim_end().as_bytes()) == record.content_sha256
            }
        };
        if !content_matches {
            return false;
        }
        if let (Some(path), Some(key)) = (&record.config_path, &record.config_key) {
            if reject_symlink(Path::new(path)).is_err() {
                return false;
            }
            let desired = format!(
                "{key} = \"{}\"",
                instruction_path.to_string_lossy().replace('\\', "/")
            );
            return edit_config_line(Path::new(path), key, "")
                .ok()
                .and_then(|(_, _, line)| line)
                .as_deref()
                == Some(desired.as_str());
        }
        true
    }

    /// 按注册表投放。重复投放沿用首次基线。
    pub fn deploy(&self, target_id: &str, content: &str) -> anyhow::Result<DeploymentRecord> {
        let target = find_target(target_id).with_context(|| format!("未知投放目标 {target_id}"))?;
        let home = self
            .resolve(&target)
            .with_context(|| format!("未检测到 {}，无法投放", target.display_name))?;
        self.deploy_to(&target, &home, content)
    }

    /// 向指定目标与数据目录投放。
    pub fn deploy_to(
        &self,
        target: &ClientTarget,
        home: &Path,
        content: &str,
    ) -> anyhow::Result<DeploymentRecord> {
        if content.trim().is_empty() {
            bail!("提示词内容为空");
        }
        if content.len() > MAX_CONTENT_BYTES {
            bail!("提示词内容超过 1 MiB 上限");
        }
        if content.contains(BEGIN_MARK) || content.contains(END_MARK) {
            bail!("提示词正文包含保留投放标记");
        }

        // Codex 的 model_instructions_file 同时归系统提示词页使用。
        // 若该键当前指向系统提示词页的受管文件，这里拒绝写入，避免双写。
        if target.id == "codex" && managed_by_system_prompt_page(home) {
            bail!("Codex 指令当前由系统提示词页托管，请先在该页停用后再投放");
        }

        let mut state = self.read_state()?;
        let existing = state
            .records
            .iter()
            .position(|record| record.target_id == target.id);
        let (canonical_home, home_identity) = canonical_home_identity(home)?;
        let instruction_relative = Path::new(&target.instruction_path);
        if instruction_relative.is_absolute()
            || instruction_relative.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir
                        | std::path::Component::RootDir
                        | std::path::Component::Prefix(_)
                )
            })
        {
            bail!("投放路径必须位于客户端目录内");
        }
        let instruction_path = home.join(instruction_relative);
        reject_symlink(&instruction_path)?;
        let backups = self.root.join("baselines");
        let baseline_path = existing
            .map(|index| PathBuf::from(&state.records[index].baseline_path))
            .unwrap_or_else(|| backups.join(format!("{}.bak", target.id)));
        let injected = content.trim_end().to_string();

        if let Some(index) = existing {
            let prior = &state.records[index];
            if prior.instruction_path != instruction_path.to_string_lossy()
                || prior.home_path != canonical_home.to_string_lossy()
                || prior.home_identity != home_identity
            {
                bail!("客户端目录或投放路径已漂移，拒绝覆盖原部署");
            }
            if !prior.baseline_empty && !Path::new(&prior.baseline_path).is_file() {
                bail!("原始基线文件缺失，拒绝覆盖部署");
            }
        }

        // 仅在首次投放时保存基线；重复投放沿用首次结论。
        let baseline_empty = match existing {
            Some(index) => state.records[index].baseline_empty,
            None => match fs::read(&instruction_path) {
                Ok(_) => false,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
                Err(error) => return Err(error).context("无法读取指令文件以建立基线"),
            },
        };

        let original = if existing.is_none() && !baseline_empty {
            Some(fs::read(&instruction_path).context("无法读取指令文件以建立基线")?)
        } else {
            None
        };
        let current = match fs::read_to_string(&instruction_path) {
            Ok(text) => Some(text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error).context("无法读取当前指令文件"),
        };
        let body = match target.kind {
            InjectionKind::MarkedBlock => {
                let base = current.as_deref().unwrap_or_default();
                let current_block = extract_marked(base)?;
                if let Some(index) = existing {
                    match current_block.as_ref() {
                        Some(block)
                            if sha256_hex(block.as_bytes())
                                == state.records[index].content_sha256 => {}
                        Some(_) => {
                            bail!("指令文件中的 CCP 注入内容已被外部修改，拒绝覆盖")
                        }
                        None => bail!("受管标记块已被移除，拒绝覆盖"),
                    }
                } else if current_block.is_some() {
                    bail!("指令文件包含未登记的 CCP 投放标记，拒绝覆盖");
                }
                let clean = strip_marked(base)?.unwrap_or_else(|| base.to_string());
                if clean.trim().is_empty() {
                    format!("{}\n", marked(&injected))
                } else {
                    format!("{}\n\n{}\n", clean.trim_end(), marked(&injected))
                }
            }
            InjectionKind::ManagedFile => {
                if let (Some(index), Some(current)) = (existing, current.as_deref()) {
                    if sha256_hex(current.trim_end().as_bytes())
                        != state.records[index].content_sha256
                    {
                        bail!("受管文件已被外部修改，拒绝覆盖");
                    }
                }
                format!("{injected}\n")
            }
        };

        let mut config_path = None;
        let mut previous_config = None;
        let mut config_line_added = false;
        let mut mutations = Vec::new();
        if let Some(original) = original {
            mutations.push(FileMutation {
                path: baseline_path.clone(),
                contents: Some(original),
            });
        }
        mutations.push(FileMutation {
            path: instruction_path.clone(),
            contents: Some(body.into_bytes()),
        });
        if let (Some(key), Some(relative)) = (
            target.config_key.as_deref(),
            target.config_relative.as_deref(),
        ) {
            let relative = Path::new(relative);
            if relative.is_absolute()
                || relative.components().any(|component| {
                    matches!(
                        component,
                        std::path::Component::ParentDir
                            | std::path::Component::RootDir
                            | std::path::Component::Prefix(_)
                    )
                })
            {
                bail!("配置路径必须位于客户端目录内");
            }
            let path = canonical_home.join(relative);
            reject_symlink(&path)?;
            let desired = format!(
                "{key} = \"{}\"",
                instruction_path.to_string_lossy().replace('\\', "/")
            );
            let (updated, added, previous) = edit_config_line(&path, key, &desired)?;
            if let Some(index) = existing {
                let (_, _, current_line) = edit_config_line(&path, key, "")?;
                if current_line.as_deref() != Some(desired.as_str()) {
                    bail!("客户端配置中的投放项已被外部修改，拒绝覆盖");
                }
                previous_config = state.records[index].previous_config.clone();
                config_line_added = state.records[index].config_line_added;
            } else {
                previous_config = previous;
                config_line_added = added;
            }
            mutations.push(FileMutation {
                path: path.clone(),
                contents: Some(updated.into_bytes()),
            });
            config_path = Some(path.to_string_lossy().to_string());
        }

        let record = DeploymentRecord {
            target_id: target.id.clone(),
            kind: target.kind,
            instruction_path: instruction_path.to_string_lossy().to_string(),
            home_path: canonical_home.to_string_lossy().to_string(),
            home_identity,
            baseline_path: baseline_path.to_string_lossy().to_string(),
            baseline_empty,
            content_sha256: sha256_hex(injected.as_bytes()),
            config_path,
            config_key: target.config_key.clone(),
            previous_config,
            config_line_added,
            deployed_at_unix: now_unix(),
        };
        match existing {
            Some(index) => state.records[index] = record.clone(),
            None => state.records.push(record.clone()),
        }
        mutations.push(FileMutation {
            path: self.state_path.clone(),
            contents: Some(serde_json::to_vec_pretty(&state)?),
        });
        apply_file_mutations(&mutations)?;
        Ok(record)
    }

    /// 批量还原。逐个执行，单个失败不影响其余目标，失败原因单独上报。
    pub fn restore_many(&self, target_ids: &[String]) -> Vec<RestoreReport> {
        target_ids
            .iter()
            .map(|target_id| match self.restore(target_id) {
                Ok(outcome) => RestoreReport {
                    target_id: target_id.clone(),
                    ok: true,
                    message: format!("已还原 {}", target_id),
                    restored_path: outcome.restored_path,
                    removed_path: outcome.removed_path,
                    sweep_message: outcome.sweep_message,
                },
                Err(error) => RestoreReport {
                    target_id: target_id.clone(),
                    ok: false,
                    message: error.to_string(),
                    restored_path: None,
                    removed_path: None,
                    sweep_message: String::new(),
                },
            })
            .collect()
    }

    /// 还原全部受管目标。
    pub fn restore_all(&self) -> anyhow::Result<Vec<RestoreReport>> {
        let ids: Vec<String> = self
            .read_state()?
            .records
            .into_iter()
            .map(|record| record.target_id)
            .collect();
        Ok(self.restore_many(&ids))
    }

    /// 按注册表还原。
    pub fn restore(&self, target_id: &str) -> anyhow::Result<RestoreOutcome> {
        let mut state = self.read_state()?;
        let index = state
            .records
            .iter()
            .position(|record| record.target_id == target_id)
            .with_context(|| format!("目标 {target_id} 当前未被 CCP 管理"))?;
        let record = state.records[index].clone();
        let (outcome, mut mutations) = self.restore_plan(&record)?;
        state.records.remove(index);
        mutations.push(FileMutation {
            path: self.state_path.clone(),
            contents: Some(serde_json::to_vec_pretty(&state)?),
        });
        apply_file_mutations(&mutations)?;
        Ok(outcome)
    }

    /// 还原一条投放记录。CCP 写出的内容被外部改动时拒绝覆盖。
    pub fn restore_record(&self, record: &DeploymentRecord) -> anyhow::Result<RestoreOutcome> {
        let (outcome, mutations) = self.restore_plan(record)?;
        apply_file_mutations(&mutations)?;
        Ok(outcome)
    }

    fn restore_plan(
        &self,
        record: &DeploymentRecord,
    ) -> anyhow::Result<(RestoreOutcome, Vec<FileMutation>)> {
        let instruction_path = PathBuf::from(&record.instruction_path);
        let baseline = PathBuf::from(&record.baseline_path);
        self.verify_record_location(record)?;
        let mut restored_path = None;
        let mut removed_path = None;
        let mut mutations = Vec::new();

        if instruction_path.exists() {
            let current = fs::read_to_string(&instruction_path).context("无法读取受管指令文件")?;
            match record.kind {
                InjectionKind::MarkedBlock => match extract_marked(&current)? {
                    Some(injected) => {
                        if sha256_hex(injected.as_bytes()) != record.content_sha256 {
                            bail!(
                                "指令文件中的 CCP 注入内容已被外部修改，拒绝覆盖：{}",
                                instruction_path.display()
                            );
                        }
                        match strip_marked(&current)? {
                            Some(clean) if !clean.trim().is_empty() => {
                                mutations.push(FileMutation {
                                    path: instruction_path.clone(),
                                    contents: Some(format!("{clean}\n").into_bytes()),
                                });
                                restored_path =
                                    Some(instruction_path.to_string_lossy().to_string());
                            }
                            _ => {
                                mutations.push(FileMutation {
                                    path: instruction_path.clone(),
                                    contents: None,
                                });
                                removed_path = Some(instruction_path.to_string_lossy().to_string());
                            }
                        }
                    }
                    // 注入块已不存在，视为已被外部移除。
                    None => {}
                },
                InjectionKind::ManagedFile => {
                    if sha256_hex(current.trim_end().as_bytes()) != record.content_sha256 {
                        bail!(
                            "受管文件已被外部修改，拒绝覆盖：{}",
                            instruction_path.display()
                        );
                    }
                    if record.baseline_empty {
                        mutations.push(FileMutation {
                            path: instruction_path.clone(),
                            contents: None,
                        });
                        removed_path = Some(instruction_path.to_string_lossy().to_string());
                    } else {
                        let original = fs::read(&baseline).context("无法读取基线文件")?;
                        mutations.push(FileMutation {
                            path: instruction_path.clone(),
                            contents: Some(original),
                        });
                        restored_path = Some(instruction_path.to_string_lossy().to_string());
                    }
                }
            }
        }

        if let (Some(path), Some(key)) =
            (record.config_path.as_deref(), record.config_key.as_deref())
        {
            let path = PathBuf::from(path);
            let expected = format!(
                "{key} = \"{}\"",
                instruction_path.to_string_lossy().replace('\\', "/")
            );
            let (_, _, current_line) = edit_config_line(&path, key, "")?;
            if current_line.as_deref() != Some(expected.as_str()) {
                bail!("客户端配置中的投放项已被外部修改，拒绝还原");
            }
            let desired = if record.config_line_added {
                String::new()
            } else {
                record.previous_config.clone().unwrap_or_default()
            };
            let (updated, _, _) = edit_config_line(&path, key, &desired)?;
            mutations.push(FileMutation {
                path,
                contents: Some(updated.into_bytes()),
            });
        }

        if baseline.exists() {
            mutations.push(FileMutation {
                path: baseline,
                contents: None,
            });
        }
        let sweep_message = format!("兜底清扫完成：处理 {} 项", mutations.len());
        Ok((
            RestoreOutcome {
                target_id: record.target_id.clone(),
                restored_path,
                removed_path,
                sweep_message,
            },
            mutations,
        ))
    }

    fn verify_record_location(&self, record: &DeploymentRecord) -> anyhow::Result<()> {
        let expected_baseline = self
            .root
            .join("baselines")
            .join(format!("{}.bak", record.target_id));
        if Path::new(&record.baseline_path) != expected_baseline {
            bail!("投放基线路径与记录不匹配，拒绝还原");
        }
        reject_symlink(Path::new(&record.instruction_path))?;
        if let Some(config_path) = record.config_path.as_deref() {
            reject_symlink(Path::new(config_path))?;
        }
        if !record.home_path.is_empty() {
            let (canonical, identity) = canonical_home_identity(Path::new(&record.home_path))?;
            if canonical.to_string_lossy() != record.home_path || identity != record.home_identity {
                bail!("客户端目录已被替换或重定向，拒绝还原");
            }
            for path in std::iter::once(record.instruction_path.as_str())
                .chain(record.config_path.as_deref())
            {
                let parent = Path::new(path)
                    .parent()
                    .with_context(|| format!("无效的受管路径：{path}"))?;
                let canonical_parent = fs::canonicalize(parent)
                    .with_context(|| format!("无法解析受管路径目录：{}", parent.display()))?;
                if !canonical_parent.starts_with(&canonical) {
                    bail!("受管路径已移出原客户端目录，拒绝还原");
                }
            }
        } else {
            bail!("部署记录缺少客户端目录身份，拒绝自动还原");
        }
        Ok(())
    }

    fn read_state(&self) -> anyhow::Result<DeploymentState> {
        let raw = match fs::read_to_string(&self.state_path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(error).context("投放状态文件丢失");
            }
            Err(error) => return Err(error).context("无法读取投放状态文件"),
        };
        if raw.trim().is_empty() {
            bail!("投放状态文件为空，拒绝按未受管状态继续");
        }
        serde_json::from_str(&raw).context("无法解析投放状态文件")
    }

    fn write_state(&self, state: &DeploymentState) -> anyhow::Result<()> {
        let body = serde_json::to_vec_pretty(state)?;
        atomic_write(&self.state_path, &body)
    }
}

/// Codex 指令是否正由系统提示词页托管。
fn managed_by_system_prompt_page(codex_home: &Path) -> bool {
    let Ok(text) = fs::read_to_string(codex_home.join("config.toml")) else {
        return false;
    };
    let Ok(doc) = text.parse::<toml_edit::DocumentMut>() else {
        return false;
    };
    doc.get("model_instructions_file")
        .and_then(|value| value.as_str())
        .and_then(|value| Path::new(value).file_name().map(|name| name.to_owned()))
        .map(|name| name == crate::system_prompt::MANAGED_FILE_NAME)
        .unwrap_or(false)
}

/// 只改目标行，其余行原样保留。`desired` 为空表示删除该行。
fn edit_config_line(
    path: &Path,
    key: &str,
    desired: &str,
) -> anyhow::Result<(String, bool, Option<String>)> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(error).with_context(|| format!("无法读取配置文件：{}", path.display()));
        }
    };
    let mut found = false;
    let mut previous = None;
    let mut out = String::with_capacity(text.len() + desired.len() + 2);
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let is_target = trimmed
            .strip_prefix(key)
            .map(|rest| rest.trim_start().starts_with('='))
            .unwrap_or(false);
        if !found && is_target {
            found = true;
            previous = Some(line.trim_end_matches(['\r', '\n']).to_string());
            if !desired.is_empty() {
                out.push_str(desired);
                out.push('\n');
            }
        } else {
            out.push_str(line);
        }
    }
    if !found && !desired.is_empty() {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(desired);
        out.push('\n');
    }
    Ok((out, !found, previous))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> (tempfile::TempDir, DeploymentEngine) {
        let dir = tempfile::tempdir().unwrap();
        let engine = DeploymentEngine::open(dir.path().join("state")).unwrap();
        (dir, engine)
    }

    /// 临时目标，避免测试触碰真实客户端目录。
    fn target(kind: InjectionKind, instruction: &str) -> ClientTarget {
        ClientTarget {
            id: "test-target".into(),
            display_name: "测试目标".into(),
            home_candidates: vec![],
            config_key: match kind {
                InjectionKind::ManagedFile => Some("model_instructions_file".into()),
                InjectionKind::MarkedBlock => None,
            },
            config_relative: match kind {
                InjectionKind::ManagedFile => Some("config.toml".into()),
                InjectionKind::MarkedBlock => None,
            },
            kind,
            instruction_path: instruction.into(),
        }
    }

    #[test]
    fn registry_covers_seven_local_clients() {
        let ids: Vec<String> = targets().into_iter().map(|entry| entry.id).collect();
        for expected in [
            "codex",
            "claude-code",
            "deepseek-harness",
            "zcode",
            "workbuddy",
            "workbuddy-cn",
            "cursor",
        ] {
            assert!(ids.iter().any(|id| id == expected), "缺少目标 {expected}");
        }
        assert_eq!(ids.len(), 7);
        assert_ne!(
            find_target("workbuddy").unwrap().home_candidates,
            find_target("workbuddy-cn").unwrap().home_candidates,
            "两个 WorkBuddy 发行版必须独立探测"
        );
        assert_eq!(
            find_target("codex").unwrap().config_key.as_deref(),
            Some("model_instructions_file")
        );
        // 豆包不在本地投放目标内（其全局记忆只在云端）。
        assert!(find_target("doubao").is_none());
    }

    #[test]
    fn resolve_picks_first_existing_candidate_and_creates_nothing() {
        let (dir, engine) = engine();
        let missing = dir.path().join("missing");
        let present = dir.path().join("present");
        fs::create_dir_all(&present).unwrap();
        let mut probe = target(InjectionKind::MarkedBlock, "AGENTS.md");
        probe.home_candidates = vec![
            missing.to_string_lossy().to_string(),
            present.to_string_lossy().to_string(),
        ];
        assert_eq!(engine.resolve(&probe), Some(present.clone()));

        let mut absent = target(InjectionKind::MarkedBlock, "AGENTS.md");
        absent.home_candidates = vec![missing.to_string_lossy().to_string()];
        assert_eq!(engine.resolve(&absent), None);
        assert!(!missing.exists(), "探测不得创建目录");
    }

    #[test]
    fn marked_block_keeps_user_content_and_removes_only_injected_text() {
        let (dir, engine) = engine();
        let home = dir.path().join("client");
        fs::create_dir_all(&home).unwrap();
        let path = home.join("AGENTS.md");
        fs::write(&path, "# 用户自己的规则\n保留我\n").unwrap();

        engine
            .deploy_to(
                &target(InjectionKind::MarkedBlock, "AGENTS.md"),
                &home,
                "注入正文",
            )
            .unwrap();
        let deployed = fs::read_to_string(&path).unwrap();
        assert!(deployed.contains("# 用户自己的规则"), "用户正文必须保留");
        assert!(deployed.contains("注入正文"));
        assert!(deployed.contains(BEGIN_MARK) && deployed.contains(END_MARK));

        let outcome = engine.restore("test-target").unwrap();
        assert!(outcome.sweep_message.contains("兜底清扫"));
        let restored = fs::read_to_string(&path).unwrap();
        assert!(restored.contains("# 用户自己的规则"));
        assert!(!restored.contains("注入正文"), "注入内容必须被移除");
        assert!(!restored.contains(BEGIN_MARK));
    }

    #[test]
    fn repeat_deploy_keeps_first_baseline_and_replaces_old_block() {
        let (dir, engine) = engine();
        let home = dir.path().join("client");
        fs::create_dir_all(&home).unwrap();
        let path = home.join("AGENTS.md");
        fs::write(&path, "原文\n").unwrap();
        let probe = target(InjectionKind::MarkedBlock, "AGENTS.md");

        engine.deploy_to(&probe, &home, "第一次").unwrap();
        let baseline = dir.path().join("state/baselines/test-target.bak");
        let first = fs::read_to_string(&baseline).unwrap();
        assert_eq!(first, "原文\n");

        engine.deploy_to(&probe, &home, "第二次").unwrap();
        assert_eq!(
            fs::read_to_string(&baseline).unwrap(),
            first,
            "基线不得被覆盖"
        );
        let current = fs::read_to_string(&path).unwrap();
        assert!(current.contains("第二次"));
        assert!(!current.contains("第一次"), "旧受管块应被替换");
        assert!(current.contains("原文"), "用户正文保留");
    }

    #[test]
    fn managed_file_replaces_and_restores_existing_config_line() {
        let (dir, engine) = engine();
        let home = dir.path().join("codex");
        fs::create_dir_all(&home).unwrap();
        let config = home.join("config.toml");
        fs::write(
            &config,
            "model = \"gpt-5\"\nmodel_instructions_file = \"/old.md\"\n",
        )
        .unwrap();

        let record = engine
            .deploy_to(
                &target(InjectionKind::ManagedFile, "ccp-client-prompt.md"),
                &home,
                "投放正文",
            )
            .unwrap();
        assert!(record.config_path.is_some());
        assert!(!record.config_line_added, "原行存在时应记为替换");
        let deployed = fs::read_to_string(&config).unwrap();
        assert!(deployed.contains("model = \"gpt-5\""), "无关配置行必须保留");
        assert!(!deployed.contains("/old.md"));
        assert!(deployed.contains("ccp-client-prompt.md"));

        engine.restore("test-target").unwrap();
        let restored = fs::read_to_string(&config).unwrap();
        assert!(restored.contains("model = \"gpt-5\""));
        assert!(
            restored.contains("model_instructions_file = \"/old.md\""),
            "原配置行必须还原"
        );
        assert!(!restored.contains("ccp-client-prompt.md"));
    }

    #[test]
    fn restore_removes_config_line_that_was_added() {
        let (dir, engine) = engine();
        let home = dir.path().join("codex");
        fs::create_dir_all(&home).unwrap();
        let config = home.join("config.toml");
        fs::write(&config, "model = \"gpt-5\"\n").unwrap();

        let record = engine
            .deploy_to(
                &target(InjectionKind::ManagedFile, "ccp-client-prompt.md"),
                &home,
                "投放正文",
            )
            .unwrap();
        assert!(record.config_line_added, "新行应记为新增");
        engine.restore("test-target").unwrap();
        assert_eq!(
            fs::read_to_string(&config).unwrap(),
            "model = \"gpt-5\"\n",
            "新增行必须移除，其余内容不变"
        );
    }

    #[test]
    fn restore_refuses_when_injected_content_was_edited() {
        let (dir, engine) = engine();
        let home = dir.path().join("client");
        fs::create_dir_all(&home).unwrap();
        let path = home.join("AGENTS.md");
        engine
            .deploy_to(
                &target(InjectionKind::MarkedBlock, "AGENTS.md"),
                &home,
                "原始注入",
            )
            .unwrap();
        let edited = fs::read_to_string(&path)
            .unwrap()
            .replace("原始注入", "被改过的注入");
        fs::write(&path, edited).unwrap();

        let error = engine.restore("test-target").unwrap_err().to_string();
        assert!(
            error.contains("外部修改"),
            "应拒绝并说明原因，实际：{error}"
        );
        assert!(
            fs::read_to_string(&path).unwrap().contains("被改过的注入"),
            "不得覆盖"
        );
    }

    #[test]
    fn restore_allows_user_edits_outside_the_injected_block() {
        let (dir, engine) = engine();
        let home = dir.path().join("client");
        fs::create_dir_all(&home).unwrap();
        let path = home.join("AGENTS.md");
        engine
            .deploy_to(
                &target(InjectionKind::MarkedBlock, "AGENTS.md"),
                &home,
                "注入正文",
            )
            .unwrap();
        let mut text = fs::read_to_string(&path).unwrap();
        text.push_str("用户后加的正文\n");
        fs::write(&path, text).unwrap();

        engine.restore("test-target").unwrap();
        let restored = fs::read_to_string(&path).unwrap();
        assert!(!restored.contains("注入正文"));
        assert!(restored.contains("用户后加的正文"), "块外正文必须保留");
    }

    #[test]
    fn managed_file_refuses_restore_after_external_edit() {
        let (dir, engine) = engine();
        let home = dir.path().join("codex");
        fs::create_dir_all(&home).unwrap();
        let path = home.join("ccp-client-prompt.md");
        engine
            .deploy_to(
                &target(InjectionKind::ManagedFile, "ccp-client-prompt.md"),
                &home,
                "投放正文",
            )
            .unwrap();
        fs::write(&path, "被外部改写的正文\n").unwrap();

        let error = engine.restore("test-target").unwrap_err().to_string();
        assert!(error.contains("外部修改"), "实际：{error}");
        assert_eq!(fs::read_to_string(&path).unwrap(), "被外部改写的正文\n");
    }

    #[test]
    fn deploy_rejects_empty_content_and_oversize_payload() {
        let (dir, engine) = engine();
        let home = dir.path().join("client");
        fs::create_dir_all(&home).unwrap();
        let probe = target(InjectionKind::MarkedBlock, "AGENTS.md");
        assert!(engine.deploy_to(&probe, &home, "   ").is_err());
        let huge = "x".repeat(MAX_CONTENT_BYTES + 1);
        assert!(engine.deploy_to(&probe, &home, &huge).is_err());
    }

    /// 只读断言：不得调用注册表目标，避免写入真实用户目录。
    #[test]
    fn restore_and_unknown_target_are_reported() {
        let (_dir, engine) = engine();
        assert!(engine.restore("codex").is_err(), "未受管时应拒绝还原");
        let error = engine
            .deploy("no-such-target", "正文")
            .unwrap_err()
            .to_string();
        assert!(error.contains("未知投放目标"), "实际：{error}");
    }

    #[test]
    fn batch_deploy_updates_content_and_reuses_first_baseline() {
        let (dir, engine) = engine();
        let home = dir.path().join("client");
        fs::create_dir_all(&home).unwrap();
        let path = home.join("AGENTS.md");
        fs::write(&path, "用户原文\n").unwrap();
        let probe = target(InjectionKind::MarkedBlock, "AGENTS.md");

        engine.deploy_to(&probe, &home, "第一份正文").unwrap();
        let baseline = dir.path().join("state/baselines/test-target.bak");
        let first = fs::read_to_string(&baseline).unwrap();
        assert_eq!(first, "用户原文\n");

        // 同一目标第二次投放：内容更新，基线保持首次那份。
        let outcomes = engine.deploy_resolved(&[(probe.clone(), home.clone())], "第二份正文");
        assert_eq!(outcomes.len(), 1);
        assert!(outcomes[0].ok, "{outcomes:?}");
        assert_eq!(
            outcomes[0].applied_path.as_deref(),
            Some(path.to_string_lossy().as_ref())
        );
        let current = fs::read_to_string(&path).unwrap();
        assert!(current.contains("第二份正文"));
        assert!(!current.contains("第一份正文"), "旧受管块应被替换");
        assert!(current.contains("用户原文"), "用户正文保留");
        assert_eq!(
            fs::read_to_string(&baseline).unwrap(),
            first,
            "基线不得被改写"
        );

        // 内容一致时跳过，不产生新记录。
        let again = engine.deploy_resolved(&[(probe.clone(), home.clone())], "第二份正文");
        assert!(again[0].ok);
        assert!(again[0].record.is_none(), "一致时应跳过并返回无新记录");

        // 还原后回到用户原文，且注入块消失。
        engine.restore("test-target").unwrap();
        let restored = fs::read_to_string(&path).unwrap();
        assert_eq!(restored, "用户原文\n");
    }

    #[test]
    fn batch_deploy_reports_undetected_and_unknown_targets() {
        let (_dir, engine) = engine();
        let ids = vec!["zcode".to_string(), "no-such-target".to_string()];
        let outcomes = engine.deploy_many(&ids, "正文");
        assert_eq!(outcomes.len(), 2);
        assert!(!outcomes[0].ok, "未安装目标应失败");
        assert!(!outcomes[1].ok, "未知目标应失败");
        assert!(outcomes[1].message.contains("未知投放目标"));
    }

    #[test]
    fn batch_restore_reports_failures_instead_of_silently_skipping() {
        let (_dir, engine) = engine();
        let ids = vec!["codex".to_string(), "no-such-target".to_string()];
        let reports = engine.restore_many(&ids);
        assert_eq!(reports.len(), 2, "失败目标也必须出现在报告里");
        assert!(reports.iter().all(|report| !report.ok));
        assert!(reports[0].message.contains("未被 CCP 管理"));
        assert!(reports[1].message.contains("未被 CCP 管理"));
    }

    #[test]
    fn restore_all_clears_every_managed_target() {
        let (dir, engine) = engine();
        let home = dir.path().join("client");
        fs::create_dir_all(&home).unwrap();
        engine
            .deploy_to(
                &target(InjectionKind::MarkedBlock, "AGENTS.md"),
                &home,
                "正文",
            )
            .unwrap();
        assert_eq!(
            engine
                .statuses()
                .unwrap()
                .iter()
                .filter(|s| s.managed)
                .count(),
            0
        );

        let records = engine.read_state().unwrap().records;
        assert_eq!(records.len(), 1);
        let outcomes = engine.restore_all().unwrap();
        assert_eq!(outcomes.len(), 1);
        assert!(
            engine.read_state().unwrap().records.is_empty(),
            "记录应被清空"
        );
        assert!(
            !home.join("AGENTS.md").exists(),
            "无块外正文时应删除受管文件"
        );
    }

    #[test]
    fn codex_deploy_is_refused_while_system_prompt_page_manages_it() {
        let (dir, engine) = engine();
        let home = dir.path().join("codex");
        fs::create_dir_all(&home).unwrap();
        let codex = find_target("codex").unwrap();
        let config = home.join("config.toml");

        // 配置指向系统提示词页的受管文件 → 必须拒绝，避免双写。
        fs::write(
            &config,
            "model_instructions_file = \"/somewhere/ccp-system-prompt.md\"\n",
        )
        .unwrap();
        let error = engine
            .deploy_to(&codex, &home, "投放正文")
            .unwrap_err()
            .to_string();
        assert!(error.contains("系统提示词页"), "实际：{error}");

        // 指向其他文件时不冲突，可正常投放。
        fs::write(&config, "model_instructions_file = \"/other/user.md\"\n").unwrap();
        let record = engine.deploy_to(&codex, &home, "投放正文").unwrap();
        assert!(record.config_path.is_some());
        assert!(
            fs::read_to_string(&config)
                .unwrap()
                .contains("ccp-client-prompt.md")
        );
    }

    #[test]
    fn config_line_edit_preserves_untouched_lines_byte_for_byte() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let original = "# 注释\nmodel = \"gpt-5\"\n\n[features]\nfoo = true\n";
        fs::write(&path, original).unwrap();

        let (updated, added, previous) = edit_config_line(
            &path,
            "model_instructions_file",
            "model_instructions_file = \"/x.md\"",
        )
        .unwrap();
        assert!(added);
        assert!(previous.is_none());
        assert!(updated.starts_with(original), "原有内容必须原样保留");
        assert!(updated.ends_with("model_instructions_file = \"/x.md\"\n"));

        let (removed, _, restored) = edit_config_line(&path, "model", "model = \"gpt-6\"").unwrap();
        assert!(!removed.contains("gpt-5"));
        assert_eq!(restored.as_deref(), Some("model = \"gpt-5\""));
    }

    #[test]
    fn statuses_report_every_registered_target() {
        let (_dir, engine) = engine();
        let rows = engine.statuses().unwrap();
        assert_eq!(rows.len(), 7);
        for row in &rows {
            assert!(!row.display_name.is_empty());
            assert!(!row.managed, "初始状态不应受管");
            assert!(!row.baseline_exists);
            assert_eq!(row.installed, row.home.is_some());
        }
        // 已探测目标必须给出投放位置。
        for row in rows.iter().filter(|row| row.installed) {
            assert!(row.planned_path.is_some(), "{} 缺少投放位置", row.target_id);
        }
    }

    #[test]
    fn malformed_marker_blocks_are_rejected_without_changing_the_file() {
        for malformed in [
            format!("{BEGIN_MARK}\na\n{END_MARK}\n{BEGIN_MARK}\nb\n{END_MARK}"),
            format!("{BEGIN_MARK}\n{BEGIN_MARK}\na\n{END_MARK}\n{END_MARK}"),
            format!("{END_MARK}\na\n{BEGIN_MARK}"),
            format!("{BEGIN_MARK}\na"),
        ] {
            let (dir, engine) = engine();
            let home = dir.path().join("client");
            fs::create_dir_all(&home).unwrap();
            let path = home.join("AGENTS.md");
            fs::write(&path, &malformed).unwrap();

            assert!(
                engine
                    .deploy_to(
                        &target(InjectionKind::MarkedBlock, "AGENTS.md"),
                        &home,
                        "new content",
                    )
                    .is_err(),
                "malformed marker block must be rejected"
            );
            assert_eq!(fs::read_to_string(&path).unwrap(), malformed);
        }
    }

    #[test]
    fn empty_state_file_is_not_treated_as_an_unmanaged_state() {
        let (_dir, engine) = engine();
        fs::write(&engine.state_path, "").unwrap();
        assert!(engine.statuses().is_err());
    }

    #[test]
    fn identical_prompt_is_not_skipped_when_managed_content_was_changed() {
        let (dir, engine) = engine();
        let home = dir.path().join("client");
        fs::create_dir_all(&home).unwrap();
        let path = home.join("AGENTS.md");
        let probe = target(InjectionKind::MarkedBlock, "AGENTS.md");
        engine.deploy_to(&probe, &home, "original").unwrap();
        fs::write(&path, "externally replaced\n").unwrap();

        let outcome = engine.deploy_resolved(&[(probe, home)], "original");
        assert!(!outcome[0].ok, "外部修改必须报告冲突");
        assert_eq!(fs::read_to_string(&path).unwrap(), "externally replaced\n");
    }

    #[test]
    fn restore_does_not_overwrite_an_externally_changed_config_line() {
        let (dir, engine) = engine();
        let home = dir.path().join("codex");
        fs::create_dir_all(&home).unwrap();
        let config = home.join("config.toml");
        fs::write(&config, "model = \"gpt-5\"\n").unwrap();
        engine
            .deploy_to(
                &target(InjectionKind::ManagedFile, "ccp-client-prompt.md"),
                &home,
                "prompt",
            )
            .unwrap();
        fs::write(
            &config,
            "model = \"gpt-5\"\nmodel_instructions_file = \"user-change.md\"\n",
        )
        .unwrap();

        assert!(engine.restore("test-target").is_err());
        assert_eq!(
            fs::read_to_string(&config).unwrap(),
            "model = \"gpt-5\"\nmodel_instructions_file = \"user-change.md\"\n"
        );
        assert!(
            engine
                .read_state()
                .unwrap()
                .records
                .iter()
                .any(|r| r.target_id == "test-target")
        );
    }

    #[test]
    fn failed_config_write_rolls_back_the_instruction_file() {
        let (dir, engine) = engine();
        let home = dir.path().join("codex");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir(home.join("config.toml")).unwrap();

        assert!(
            engine
                .deploy_to(
                    &target(InjectionKind::ManagedFile, "ccp-client-prompt.md"),
                    &home,
                    "prompt",
                )
                .is_err()
        );
        assert!(!home.join("ccp-client-prompt.md").exists());
        assert!(engine.read_state().unwrap().records.is_empty());
    }
}
