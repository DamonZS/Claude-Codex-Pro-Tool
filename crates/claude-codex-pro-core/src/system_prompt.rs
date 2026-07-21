use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use toml_edit::DocumentMut;

const STORE_DIR: &str = "system-prompts";
const STATE_FILE: &str = "state.json";
const MANAGED_FILE: &str = "ccp-system-prompt.md";
const MAX_PROMPTS: usize = 200;
const MAX_CONTENT_BYTES: usize = 1024 * 1024;

const BUILTINS: [(&str, &str, &str, &str, &str); 5] = [
    (
        "builtin-gpt55",
        "gpt-5.5 unrestricted 破甲",
        "gpt5.5-unrestricted.md",
        "方法：先让 AI 分析项目，再按模板指令处理。",
        include_str!("../../../assets/system-prompts/gpt5.5-unrestricted.md"),
    ),
    (
        "builtin-gpt54",
        "gpt-5.4 unrestricted 破甲",
        "gpt5.4-unrestricted.md",
        "兼容 GPT-5.4 的指令模板。",
        include_str!("../../../assets/system-prompts/gpt5.4-unrestricted.md"),
    ),
    (
        "builtin-jeli",
        "gpt5.5-jeli.md",
        "gpt5.5-jeli.md",
        "gpt5.5 大白话（80% 场景）破甲。",
        include_str!("../../../assets/system-prompts/gpt5.5-jeli.md"),
    ),
    (
        "builtin-gpt56-sol",
        "gpt-5.6-sol-unrestricted.md",
        "gpt-5.6-sol-unrestricted.md",
        "gpt5.6-sol 指令提示词。",
        include_str!("../../../assets/system-prompts/gpt-5.6-sol-unrestricted.md"),
    ),
    (
        "builtin-seagull",
        "海鸥3.0破甲.md",
        "海鸥3.0破甲.md",
        "测试生效：海鸥在线，你要整点薯条吗？",
        include_str!("../../../assets/system-prompts/海鸥3.0破甲.md"),
    ),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SystemPromptItem {
    pub id: String,
    pub title: String,
    pub filename: String,
    pub description: String,
    pub category: String,
    pub content: String,
    pub builtin: bool,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SystemPromptMode {
    Preserve,
    Replace,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SystemPromptSnapshot {
    pub prompts: Vec<SystemPromptItem>,
    pub active_prompt_id: Option<String>,
    pub active_title: Option<String>,
    pub active_path: Option<String>,
    pub mode: Option<SystemPromptMode>,
    pub managed: bool,
    pub externally_modified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SaveSystemPromptRequest {
    #[serde(default)]
    pub id: String,
    pub title: String,
    pub filename: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_category")]
    pub category: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PromptState {
    #[serde(default)]
    custom_prompts: Vec<SystemPromptItem>,
    active_prompt_id: Option<String>,
    mode: Option<SystemPromptMode>,
    previous_instruction_path: Option<String>,
    #[serde(default)]
    previous_instruction_present: bool,
}

pub struct SystemPromptStore {
    root: PathBuf,
    codex_home: PathBuf,
}

impl SystemPromptStore {
    pub fn open_default() -> anyhow::Result<Self> {
        Self::open(
            crate::paths::default_app_state_dir().join(STORE_DIR),
            crate::relay_config::default_codex_home_dir(),
        )
    }

    pub fn open(root: impl Into<PathBuf>, codex_home: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let store = Self {
            root: root.into(),
            codex_home: codex_home.into(),
        };
        fs::create_dir_all(&store.root).context("无法创建系统提示词存储目录")?;
        fs::create_dir_all(&store.codex_home).context("无法创建 Codex 配置目录")?;
        if !store.state_path().exists() {
            store.write_state(&PromptState::default())?;
        }
        Ok(store)
    }

    pub fn list(&self) -> anyhow::Result<SystemPromptSnapshot> {
        let state = self.read_state()?;
        let mut prompts = builtin_prompts();
        let mut custom = state.custom_prompts.clone();
        custom.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        prompts.extend(custom);
        let configured = self.configured_instruction_path()?;
        let managed_path = self.managed_path_string();
        let managed =
            state.active_prompt_id.is_some() && configured.as_deref() == Some(&managed_path);
        let externally_modified = state.active_prompt_id.is_some() && !managed;
        let active_title = state
            .active_prompt_id
            .as_ref()
            .and_then(|id| prompts.iter().find(|p| &p.id == id))
            .map(|p| p.title.clone());
        Ok(SystemPromptSnapshot {
            prompts,
            active_prompt_id: state.active_prompt_id,
            active_title,
            active_path: configured,
            mode: state.mode,
            managed,
            externally_modified,
        })
    }

    pub fn save(&self, request: SaveSystemPromptRequest) -> anyhow::Result<SystemPromptSnapshot> {
        validate_request(&request)?;
        let mut state = self.read_state()?;
        let now = now_secs();
        let id = if request.id.trim().is_empty() {
            format!("prompt-{now}-{}", state.custom_prompts.len() + 1)
        } else {
            request.id.trim().to_string()
        };
        if id.starts_with("builtin-") {
            bail!("内置提示词不可编辑");
        }
        let filename = normalize_filename(&request.filename, &request.title)?;
        if let Some(other) = state
            .custom_prompts
            .iter()
            .find(|p| p.id != id && p.filename.eq_ignore_ascii_case(&filename))
        {
            bail!("文件名已被提示词“{}”使用", other.title);
        }
        if let Some(item) = state.custom_prompts.iter_mut().find(|p| p.id == id) {
            item.title = request.title.trim().to_string();
            item.filename = filename;
            item.description = request.description.trim().to_string();
            item.category = clean_category(&request.category);
            item.content = canonical_content(&request.content);
            item.updated_at = now;
        } else {
            if state.custom_prompts.len() >= MAX_PROMPTS {
                bail!("系统提示词数量已达到上限");
            }
            state.custom_prompts.push(SystemPromptItem {
                id,
                title: request.title.trim().to_string(),
                filename,
                description: request.description.trim().to_string(),
                category: clean_category(&request.category),
                content: canonical_content(&request.content),
                builtin: false,
                created_at: now,
                updated_at: now,
            });
        }
        self.write_state(&state)?;
        self.list()
    }

    pub fn import_markdown(&self, path: impl AsRef<Path>) -> anyhow::Result<SystemPromptSnapshot> {
        let path = path.as_ref();
        if path
            .extension()
            .and_then(|v| v.to_str())
            .map(|v| !v.eq_ignore_ascii_case("md"))
            .unwrap_or(true)
        {
            bail!("仅支持导入 Markdown 文件");
        }
        let bytes = fs::read(path).context("读取 Markdown 文件失败")?;
        if bytes.len() > MAX_CONTENT_BYTES {
            bail!("Markdown 文件超过 1 MiB");
        }
        let content = String::from_utf8(bytes).context("Markdown 必须使用 UTF-8 编码")?;
        let filename = path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("imported-prompt.md")
            .to_string();
        let fallback = path
            .file_stem()
            .and_then(|v| v.to_str())
            .unwrap_or("导入的提示词");
        let title = content
            .lines()
            .find_map(|line| line.trim().strip_prefix("# ").map(str::trim))
            .filter(|v| !v.is_empty())
            .unwrap_or(fallback)
            .to_string();
        self.save(SaveSystemPromptRequest {
            id: String::new(),
            title,
            filename,
            description: "从 Markdown 文件导入".to_string(),
            category: "导入".to_string(),
            content,
        })
    }

    pub async fn sync_markdown_url(&self, url: &str) -> anyhow::Result<SystemPromptSnapshot> {
        let parsed = reqwest::Url::parse(url.trim()).context("GitHub 模板地址无效")?;
        if parsed.scheme() != "https" {
            bail!("GitHub 模板地址必须使用 HTTPS");
        }
        let client = crate::http_client::proxied_client("ClaudeCodexPro/SystemPrompt")?;
        let response = client
            .get(parsed.clone())
            .send()
            .await
            .context("下载 GitHub 模板失败")?;
        if !response.status().is_success() {
            bail!("GitHub 模板下载失败：HTTP {}", response.status());
        }
        if response.content_length().unwrap_or(0) > MAX_CONTENT_BYTES as u64 {
            bail!("远程 Markdown 超过 1 MiB");
        }
        let bytes = response.bytes().await.context("读取远程 Markdown 失败")?;
        if bytes.len() > MAX_CONTENT_BYTES {
            bail!("远程 Markdown 超过 1 MiB");
        }
        let content =
            String::from_utf8(bytes.to_vec()).context("远程 Markdown 必须使用 UTF-8 编码")?;
        let filename = parsed
            .path_segments()
            .and_then(|mut parts| parts.next_back())
            .filter(|v| !v.is_empty())
            .unwrap_or("github-prompt.md")
            .to_string();
        if !filename.to_ascii_lowercase().ends_with(".md") {
            bail!("远程地址必须指向 Markdown 文件");
        }
        let fallback = filename.trim_end_matches(".md");
        let title = content
            .lines()
            .find_map(|line| line.trim().strip_prefix("# ").map(str::trim))
            .filter(|v| !v.is_empty())
            .unwrap_or(fallback)
            .to_string();
        self.save(SaveSystemPromptRequest {
            id: String::new(),
            title,
            filename,
            description: format!("同步自 {}", parsed.host_str().unwrap_or("GitHub")),
            category: "GitHub".to_string(),
            content,
        })
    }

    pub fn delete(&self, id: &str) -> anyhow::Result<SystemPromptSnapshot> {
        let mut state = self.read_state()?;
        if state.active_prompt_id.as_deref() == Some(id) {
            bail!("请先停用当前提示词，再执行删除");
        }
        let before = state.custom_prompts.len();
        state.custom_prompts.retain(|item| item.id != id);
        if before == state.custom_prompts.len() {
            bail!("提示词不存在或为不可删除的内置模板");
        }
        self.write_state(&state)?;
        self.list()
    }

    pub fn enable(&self, id: &str, mode: SystemPromptMode) -> anyhow::Result<SystemPromptSnapshot> {
        let mut state = self.read_state()?;
        let prompts = self.list()?.prompts;
        let prompt = prompts
            .into_iter()
            .find(|p| p.id == id)
            .context("提示词不存在")?;
        if state.active_prompt_id.is_some()
            && self.configured_instruction_path()?.as_deref() != Some(&self.managed_path_string())
        {
            bail!("Codex 指令配置已被外部修改，请先处理当前外部配置");
        }
        if state.active_prompt_id.is_none() {
            let current = self.configured_instruction_path()?;
            state.previous_instruction_present = current.is_some();
            state.previous_instruction_path = current;
        }
        let mut content = String::new();
        if mode == SystemPromptMode::Preserve {
            if let Some(path) = state.previous_instruction_path.as_deref() {
                if let Some(original) = self.read_instruction_file(path)? {
                    content.push_str("<!-- CCP preserved instructions -->\n");
                    content.push_str(original.trim());
                    content.push_str("\n\n<!-- CCP selected system prompt -->\n");
                }
            }
        }
        content.push_str(prompt.content.trim());
        content.push('\n');
        crate::settings::atomic_write(&self.managed_path(), content.as_bytes())?;
        self.write_config_instruction(Some(&self.managed_path_string()))?;
        state.active_prompt_id = Some(prompt.id);
        state.mode = Some(mode);
        self.write_state(&state)?;
        self.list()
    }

    pub fn disable(&self) -> anyhow::Result<SystemPromptSnapshot> {
        let mut state = self.read_state()?;
        if state.active_prompt_id.is_none() {
            return self.list();
        }
        if self.configured_instruction_path()?.as_deref() != Some(&self.managed_path_string()) {
            bail!("Codex 指令配置已被外部修改，CCP 未覆盖该外部配置");
        }
        let restore = if state.previous_instruction_present {
            state.previous_instruction_path.as_deref()
        } else {
            None
        };
        self.write_config_instruction(restore)?;
        state.active_prompt_id = None;
        state.mode = None;
        state.previous_instruction_path = None;
        state.previous_instruction_present = false;
        self.write_state(&state)?;
        self.list()
    }

    fn state_path(&self) -> PathBuf {
        self.root.join(STATE_FILE)
    }
    fn managed_path(&self) -> PathBuf {
        self.root.join(MANAGED_FILE)
    }
    fn managed_path_string(&self) -> String {
        self.managed_path().to_string_lossy().to_string()
    }
    fn config_path(&self) -> PathBuf {
        self.codex_home.join("config.toml")
    }
    fn read_state(&self) -> anyhow::Result<PromptState> {
        serde_json::from_slice(&fs::read(self.state_path())?).context("系统提示词状态文件损坏")
    }
    fn write_state(&self, state: &PromptState) -> anyhow::Result<()> {
        crate::settings::atomic_write(&self.state_path(), &serde_json::to_vec_pretty(state)?)
    }

    fn configured_instruction_path(&self) -> anyhow::Result<Option<String>> {
        let path = self.config_path();
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(path).context("读取 Codex config.toml 失败")?;
        let doc = text
            .parse::<DocumentMut>()
            .context("Codex config.toml 格式无效")?;
        Ok(doc
            .get("model_instructions_file")
            .and_then(|v| v.as_str())
            .map(str::to_string))
    }

    fn write_config_instruction(&self, value: Option<&str>) -> anyhow::Result<()> {
        let path = self.config_path();
        let text = if path.exists() {
            fs::read_to_string(&path).context("读取 Codex config.toml 失败")?
        } else {
            String::new()
        };
        let mut doc = text
            .parse::<DocumentMut>()
            .context("Codex config.toml 格式无效，未执行覆盖")?;
        match value {
            Some(path) => doc["model_instructions_file"] = toml_edit::value(path),
            None => {
                doc.remove("model_instructions_file");
            }
        }
        if path.exists() {
            let backup = self.root.join(format!("config.toml.backup-{}", now_secs()));
            crate::settings::atomic_write(&backup, text.as_bytes())?;
        }
        crate::settings::atomic_write(&path, doc.to_string().as_bytes())
    }

    fn read_instruction_file(&self, configured: &str) -> anyhow::Result<Option<String>> {
        let path = PathBuf::from(configured);
        let resolved = if path.is_absolute() {
            path
        } else {
            self.codex_home.join(path)
        };
        if !resolved.exists() {
            return Ok(None);
        }
        let bytes = fs::read(resolved).context("读取原提示词文件失败")?;
        if bytes.len() > MAX_CONTENT_BYTES {
            bail!("原提示词文件超过 1 MiB");
        }
        Ok(Some(
            String::from_utf8(bytes).context("原提示词文件不是 UTF-8")?,
        ))
    }
}

fn builtin_prompts() -> Vec<SystemPromptItem> {
    BUILTINS
        .iter()
        .map(
            |(id, title, filename, description, content)| SystemPromptItem {
                id: (*id).to_string(),
                title: (*title).to_string(),
                filename: (*filename).to_string(),
                description: (*description).to_string(),
                category: "破甲/逆向".to_string(),
                content: (*content).to_string(),
                builtin: true,
                created_at: 0,
                updated_at: 0,
            },
        )
        .collect()
}

fn default_category() -> String {
    "软件开发".to_string()
}
fn clean_category(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        default_category()
    } else {
        value.chars().take(32).collect()
    }
}
fn canonical_content(value: &str) -> String {
    value
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim()
        .to_string()
}
fn validate_request(request: &SaveSystemPromptRequest) -> anyhow::Result<()> {
    if request.title.trim().is_empty() {
        bail!("提示词名称不能为空");
    }
    if request.title.chars().count() > 120 {
        bail!("提示词名称过长");
    }
    if request.content.trim().is_empty() {
        bail!("提示词内容不能为空");
    }
    if request.content.len() > MAX_CONTENT_BYTES {
        bail!("提示词内容超过 1 MiB");
    }
    Ok(())
}
fn normalize_filename(input: &str, fallback: &str) -> anyhow::Result<String> {
    let raw = if input.trim().is_empty() {
        fallback
    } else {
        input.trim()
    }
    .trim_end_matches(".md");
    let mut out = String::new();
    let mut dash = false;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch.to_ascii_lowercase());
            dash = false;
        } else if ch.is_whitespace() && !dash {
            out.push('-');
            dash = true;
        } else if !ch.is_ascii() {
            out.push(ch);
        }
    }
    let out = out.trim_matches('-');
    if out.is_empty() || out == "." || out == ".." {
        bail!("提示词文件名无效");
    }
    Ok(format!("{out}.md"))
}
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn store() -> (tempfile::TempDir, SystemPromptStore) {
        let root = tempdir().unwrap();
        let store =
            SystemPromptStore::open(root.path().join("state"), root.path().join("codex")).unwrap();
        (root, store)
    }

    #[test]
    fn includes_five_bundled_prompts() {
        let (_root, store) = store();
        let snapshot = store.list().unwrap();
        assert_eq!(snapshot.prompts.len(), 5);
        assert!(snapshot.prompts.iter().all(|item| item.builtin));
    }

    #[test]
    fn replace_enable_and_disable_restore_config() {
        let (_root, store) = store();
        fs::write(
            store.config_path(),
            "model = \"gpt-test\"\nmodel_instructions_file = \"original.md\"\n",
        )
        .unwrap();
        fs::write(store.codex_home.join("original.md"), "UNIQUE ORIGINAL RULE").unwrap();
        let active = store
            .enable("builtin-gpt55", SystemPromptMode::Replace)
            .unwrap();
        assert!(active.managed);
        assert!(
            !fs::read_to_string(store.managed_path())
                .unwrap()
                .contains("UNIQUE ORIGINAL RULE")
        );
        store.disable().unwrap();
        assert_eq!(
            store.configured_instruction_path().unwrap().as_deref(),
            Some("original.md")
        );
    }

    #[test]
    fn preserve_mode_combines_original_and_selected() {
        let (_root, store) = store();
        fs::write(
            store.config_path(),
            "model_instructions_file = \"original.md\"\n",
        )
        .unwrap();
        fs::write(store.codex_home.join("original.md"), "ORIGINAL RULE").unwrap();
        store
            .enable("builtin-gpt54", SystemPromptMode::Preserve)
            .unwrap();
        let content = fs::read_to_string(store.managed_path()).unwrap();
        assert!(content.contains("ORIGINAL RULE"));
        assert!(content.contains("GPT-5.4"));
    }

    #[test]
    fn external_config_change_is_not_overwritten() {
        let (_root, store) = store();
        store
            .enable("builtin-gpt55", SystemPromptMode::Replace)
            .unwrap();
        fs::write(
            store.config_path(),
            "model_instructions_file = \"external.md\"\n",
        )
        .unwrap();
        assert!(
            store
                .disable()
                .unwrap_err()
                .to_string()
                .contains("外部修改")
        );
        assert_eq!(
            store.configured_instruction_path().unwrap().as_deref(),
            Some("external.md")
        );
    }
}
