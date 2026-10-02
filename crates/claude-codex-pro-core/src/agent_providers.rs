//! Supplier storage and the shared write engine for the "config file" agents.
//!
//! Agents such as OpenCode, OpenClaw, Pi and WorkBuddy own a native config
//! file. CCP never rewrites the whole file: it stores the supplier profiles in
//! `ccp.db` and patches only the nodes it owns (CC Switch style). Every write
//! goes through [`write_with_backup`], which keeps a byte-for-byte copy of the
//! original file the first time CCP touches it.

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

pub mod openclaw;
pub mod opencode;
pub mod pi;
pub mod workbuddy;

/// Every agent that keeps its own supplier list (spec
/// `supplier-routing-ccswitch.md`, "Agent 范围").
pub const AGENT_PROVIDER_APP_IDS: [&str; 9] = [
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

/// How CCP applies a supplier to an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApplyMode {
    /// Only one supplier is active; switching rewrites the key fields.
    Switch,
    /// Many suppliers live side by side in the agent's own file.
    Additive,
    /// Settings live in encrypted app storage; CCP only shows the values.
    Manual,
}

/// The apply mode of `app_id`, or `None` for an unknown agent.
pub fn apply_mode(app_id: &str) -> Option<ApplyMode> {
    match app_id {
        "gemini" | "grok" => Some(ApplyMode::Switch),
        "cursor" => Some(ApplyMode::Manual),
        "opencode" | "openclaw" | "hermes" | "pi" | "mcode" | "workbuddy" => {
            Some(ApplyMode::Additive)
        }
        _ => None,
    }
}

/// Wire protocol the upstream endpoint speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiFormat {
    #[serde(rename = "openai-chat")]
    OpenaiChat,
    #[serde(rename = "openai-responses")]
    OpenaiResponses,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "gemini-native")]
    GeminiNative,
}

/// The formats `app_id` supports, in display order.
pub fn allowed_formats(app_id: &str) -> &'static [ApiFormat] {
    use ApiFormat::{Anthropic, GeminiNative, OpenaiChat, OpenaiResponses};
    match app_id {
        "gemini" => &[GeminiNative],
        "grok" => &[OpenaiResponses, OpenaiChat],
        "opencode" => &[OpenaiChat, Anthropic, OpenaiResponses],
        "openclaw" | "pi" => &[OpenaiChat, OpenaiResponses, Anthropic],
        "hermes" => &[OpenaiChat, Anthropic],
        "mcode" => &[Anthropic, OpenaiChat, OpenaiResponses],
        "workbuddy" | "cursor" => &[OpenaiChat],
        _ => &[],
    }
}

/// A supplier stored for one config-file agent.
///
/// `api_key` is write-only: it is never serialized back to the frontend, which
/// only sees `has_api_key`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProvider {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub base_url: String,
    #[serde(default, skip_serializing)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub has_api_key: bool,
    pub api_format: ApiFormat,
    pub models: Vec<String>,
    pub default_model: String,
    pub notes: String,
    pub sort_index: i64,
}

/// Per-app bookkeeping: which supplier is live in the agent's own file.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProviderState {
    pub active_id: Option<String>,
    pub applied_ids: Vec<String>,
    pub config_path: String,
    pub installed: bool,
}

/// Normalize a supplier in place: trim, de-duplicate, and fill the defaults
/// that the validation rules expect.
fn normalize(provider: &mut AgentProvider) {
    provider.app_id = provider.app_id.trim().to_string();
    provider.name = provider.name.trim().to_string();
    provider.base_url = provider.base_url.trim().trim_end_matches('/').to_string();
    provider.notes = provider.notes.trim().to_string();
    provider.default_model = provider.default_model.trim().to_string();

    let mut models: Vec<String> = Vec::new();
    for model in &provider.models {
        let model = model.trim();
        if model.is_empty() || models.iter().any(|kept| kept == model) {
            continue;
        }
        models.push(model.to_string());
    }
    provider.models = models;

    if provider.default_model.is_empty()
        && matches!(apply_mode(&provider.app_id), Some(ApplyMode::Switch))
        && let Some(first) = provider.models.first()
    {
        provider.default_model = first.clone();
    }
}

/// Check a supplier before it is stored or written.
///
/// `key_present` is `true` when the provider already has a stored key or the
/// caller supplied one; a supplier without a key can never be applied.
pub fn validate(provider: &AgentProvider, key_present: bool) -> anyhow::Result<()> {
    if !AGENT_PROVIDER_APP_IDS.contains(&provider.app_id.as_str()) {
        bail!("未知的 Agent 应用：{}", provider.app_id);
    }
    if provider.name.trim().is_empty() {
        bail!("供应商名称不能为空");
    }
    if provider.name.trim().chars().count() > 80 {
        bail!("供应商名称不能超过 80 个字符");
    }

    let base_url = provider.base_url.trim();
    if base_url.is_empty() {
        bail!("Base URL 不能为空");
    }
    let parsed =
        url::Url::parse(base_url).map_err(|_| anyhow::anyhow!("Base URL 不是合法的地址"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        bail!("Base URL 必须以 http:// 或 https:// 开头");
    }
    if parsed.host_str().is_none_or(str::is_empty) {
        bail!("Base URL 缺少主机名");
    }

    if !key_present {
        bail!("请先填写 API Key");
    }

    if provider.models.is_empty() {
        bail!("至少需要填写一个模型");
    }
    if provider.models.iter().any(|model| model.trim().is_empty()) {
        bail!("模型名称不能为空");
    }

    if !allowed_formats(&provider.app_id).contains(&provider.api_format) {
        bail!("该 Agent 不支持所选协议");
    }

    if matches!(apply_mode(&provider.app_id), Some(ApplyMode::Switch))
        && !provider.default_model.trim().is_empty()
        && !provider
            .models
            .iter()
            .any(|model| model == provider.default_model.trim())
    {
        bail!("默认模型必须包含在模型列表中");
    }

    Ok(())
}

/// Open the CCP database and return a store over it.
pub struct AgentProviderStore {
    conn: Connection,
}

impl AgentProviderStore {
    /// Open `<app state dir>/ccp.db`.
    pub fn open_default() -> anyhow::Result<Self> {
        let path = crate::ccp_db::default_db_path();
        Self::open(&path)
    }

    /// Open the database at `path`.
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let conn = crate::ccp_db::open(path)?;
        Ok(Self { conn })
    }

    /// Every supplier, ordered by app then position. Keys are never read out.
    pub fn list(&self) -> anyhow::Result<Vec<AgentProvider>> {
        let mut statement = self.conn.prepare(
            "SELECT id, app_id, position, profile_json, api_key
             FROM agent_providers ORDER BY app_id, position, id",
        )?;
        let mut rows = statement.query([])?;
        let mut providers = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let app_id: String = row.get(1)?;
            let position: i64 = row.get(2)?;
            let profile_json: String = row.get(3)?;
            let api_key: String = row.get(4)?;

            let mut provider: AgentProvider = match serde_json::from_str(&profile_json) {
                Ok(provider) => provider,
                Err(error) => {
                    eprintln!("供应商记录 {id} 解析失败，已跳过: {error}");
                    continue;
                }
            };
            provider.id = id;
            provider.app_id = app_id;
            provider.sort_index = position;
            provider.has_api_key = !api_key.is_empty();
            provider.api_key = None;
            providers.push(provider);
        }
        Ok(providers)
    }

    /// One supplier together with its stored API key.
    pub fn get_with_key(&self, id: &str) -> anyhow::Result<Option<(AgentProvider, String)>> {
        let row = self
            .conn
            .query_row(
                "SELECT app_id, position, profile_json, api_key FROM agent_providers WHERE id = ?1",
                [id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?;

        let Some((app_id, position, profile_json, api_key)) = row else {
            return Ok(None);
        };
        let mut provider: AgentProvider = serde_json::from_str(&profile_json)
            .with_context(|| format!("供应商记录 {id} 解析失败"))?;
        provider.id = id.to_string();
        provider.app_id = app_id;
        provider.sort_index = position;
        provider.has_api_key = !api_key.is_empty();
        provider.api_key = None;
        Ok(Some((provider, api_key)))
    }

    /// Insert or update a supplier and return the stored row.
    ///
    /// An empty `id` creates a supplier with a fresh `ap-…` id at the end of
    /// its app's list. `api_key` is only written when a non-empty key is given,
    /// so re-saving a form that did not echo the key keeps the stored one.
    pub fn save(&mut self, provider: &mut AgentProvider) -> anyhow::Result<AgentProvider> {
        let mut candidate = provider.clone();
        normalize(&mut candidate);

        let previous_key = if candidate.id.is_empty() {
            String::new()
        } else {
            self.conn
                .query_row(
                    "SELECT api_key FROM agent_providers WHERE id = ?1",
                    [candidate.id.as_str()],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
                .unwrap_or_default()
        };
        let incoming_key = candidate
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .map(str::to_string);
        let stored_key = incoming_key.unwrap_or(previous_key);
        validate(&candidate, !stored_key.is_empty())?;

        let is_new = candidate.id.is_empty();
        if is_new {
            candidate.id = new_provider_id();
        }

        let profile_json = serde_json::to_string(&candidate)?;
        let transaction = self.conn.transaction()?;
        if is_new {
            let next_position: i64 = transaction.query_row(
                "SELECT COALESCE(MAX(position) + 1, 0) FROM agent_providers WHERE app_id = ?1",
                [candidate.app_id.as_str()],
                |row| row.get(0),
            )?;
            candidate.sort_index = next_position;
            transaction.execute(
                "INSERT INTO agent_providers (id, app_id, position, profile_json, api_key)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    candidate.id,
                    candidate.app_id,
                    candidate.sort_index,
                    serde_json::to_string(&candidate)?,
                    stored_key
                ],
            )?;
        } else {
            let updated = transaction.execute(
                "UPDATE agent_providers
                 SET app_id = ?2, position = ?3, profile_json = ?4, api_key = ?5
                 WHERE id = ?1",
                rusqlite::params![
                    candidate.id,
                    candidate.app_id,
                    candidate.sort_index,
                    profile_json,
                    stored_key
                ],
            )?;
            if updated == 0 {
                bail!("供应商不存在：{}", candidate.id);
            }
        }
        transaction.commit()?;

        candidate.api_key = None;
        candidate.has_api_key = !stored_key.is_empty();
        *provider = candidate.clone();
        Ok(candidate)
    }

    /// Delete a supplier.
    pub fn delete(&mut self, id: &str) -> anyhow::Result<()> {
        self.conn
            .execute("DELETE FROM agent_providers WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Rewrite the positions of `ids` for `app_id`, in the given order.
    pub fn reorder(&mut self, app_id: &str, ids: &[String]) -> anyhow::Result<()> {
        let transaction = self.conn.transaction()?;
        for (position, id) in ids.iter().enumerate() {
            transaction.execute(
                "UPDATE agent_providers SET position = ?3 WHERE id = ?1 AND app_id = ?2",
                rusqlite::params![id, app_id, position as i64],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// The stored state of `app_id`, with the config path resolved from disk.
    pub fn state(&self, app_id: &str) -> anyhow::Result<AgentProviderState> {
        let row = self
            .conn
            .query_row(
                "SELECT active_id, applied_ids_json FROM agent_provider_state WHERE app_id = ?1",
                [app_id],
                |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;

        let (active_id, applied_ids) = match row {
            Some((active_id, applied_json)) => {
                let applied_ids = serde_json::from_str::<Vec<String>>(&applied_json)
                    .unwrap_or_else(|_| Vec::new());
                (active_id, applied_ids)
            }
            None => (None, Vec::new()),
        };

        let config_path = config_path_for(app_id);
        let installed = config_path.as_ref().is_some_and(|path| path.is_file());
        Ok(AgentProviderState {
            active_id,
            applied_ids,
            config_path: config_path
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default(),
            installed,
        })
    }

    /// Remember which supplier is live in a switch-mode agent.
    pub fn set_active(&mut self, app_id: &str, active_id: Option<&str>) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO agent_provider_state (app_id, active_id, applied_ids_json)
             VALUES (?1, ?2, '[]')
             ON CONFLICT(app_id) DO UPDATE SET active_id = excluded.active_id",
            rusqlite::params![app_id, active_id],
        )?;
        Ok(())
    }

    /// Remember which suppliers CCP has written into an additive agent.
    pub fn set_applied(&mut self, app_id: &str, ids: &[String]) -> anyhow::Result<()> {
        let applied_json = serde_json::to_string(ids)?;
        self.conn.execute(
            "INSERT INTO agent_provider_state (app_id, active_id, applied_ids_json)
             VALUES (?1, NULL, ?2)
             ON CONFLICT(app_id) DO UPDATE SET applied_ids_json = excluded.applied_ids_json",
            rusqlite::params![app_id, applied_json],
        )?;
        Ok(())
    }
}

/// A fresh supplier id: `ap-` plus the first 12 characters of a UUID v4.
fn new_provider_id() -> String {
    let uuid = uuid::Uuid::new_v4().simple().to_string();
    format!("ap-{}", &uuid[..12])
}

/// Resolve the config file an agent owns, for display purposes.
pub fn config_path_for(app_id: &str) -> Option<PathBuf> {
    let dir = default_dir(app_id)?;
    let writer = writer_for(app_id);
    match writer {
        Some(writer) => Some(writer.config_path(&dir)),
        None => None,
    }
}

/// The default config directory of an agent, honouring its env overrides.
pub fn default_dir(app_id: &str) -> Option<PathBuf> {
    let home = home_dir()?;
    match app_id {
        "gemini" => Some(home.join(".gemini")),
        "grok" => Some(home.join(".grok")),
        "opencode" => Some(home.join(".config").join("opencode")),
        "openclaw" => Some(home.join(".openclaw")),
        "hermes" => {
            if let Some(dir) = env_path("HERMES_HOME") {
                return Some(dir);
            }
            if cfg!(windows)
                && let Some(local) = env_path("LOCALAPPDATA")
            {
                return Some(local.join("hermes"));
            }
            Some(home.join(".hermes"))
        }
        "pi" => {
            if let Some(dir) = env_path("PI_CODING_AGENT_DIR") {
                return Some(dir);
            }
            Some(home.join(".pi").join("agent"))
        }
        "mcode" => {
            if let Some(dir) = env_path("MINIMAX_DATA_DIR") {
                return Some(dir);
            }
            if let Some(dir) = env_path("MAVIS_DATA_DIR") {
                return Some(dir);
            }
            Some(home.join(".minimax"))
        }
        "workbuddy" => Some(home.join(".workbuddy")),
        // Cursor keeps its models in its own settings UI, not in a file.
        "cursor" => None,
        _ => None,
    }
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

fn home_dir() -> Option<PathBuf> {
    directories::UserDirs::new().map(|dirs| dirs.home_dir().to_path_buf())
}

/// The writer that owns `app_id`'s config file, if CCP writes one yet.
pub fn writer_for(app_id: &str) -> Option<Box<dyn AgentWriter>> {
    match app_id {
        "workbuddy" => Some(Box::new(workbuddy::WorkBuddyWriter)),
        "opencode" => Some(Box::new(opencode::OpenCodeWriter)),
        "openclaw" => Some(Box::new(openclaw::OpenClawWriter)),
        "pi" => Some(Box::new(pi::PiWriter)),
        _ => None,
    }
}

/// One writer per app file.
///
/// Golden rule: a writer only ever touches the nodes CCP owns. Every other key
/// in the file must survive a round trip byte for byte in value.
pub trait AgentWriter {
    /// The file this writer patches, inside `dir`.
    fn config_path(&self, dir: &Path) -> PathBuf;

    /// Additive: write or replace the entry with this id.
    /// Switch: write the key fields to the current provider.
    fn apply(&self, dir: &Path, provider: &AgentProvider, api_key: &str) -> anyhow::Result<()>;

    /// Additive: remove the entry with this id. Switch: a no-op.
    fn unapply(&self, dir: &Path, provider_id: &str) -> anyhow::Result<()>;
}

/// Write `bytes` to `path`, keeping the original safe.
///
/// The first write of any file copies the existing bytes to
/// `<path>.ccp-first-write.bak`; later writes never touch that backup again.
/// The new content lands in `<path>.ccp.tmp` and is renamed over the target,
/// so a failure leaves the original file untouched.
pub fn write_with_backup(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建配置目录失败：{}", parent.display()))?;
    }

    let backup = first_write_backup_path(path);
    if path.is_file() && !backup.exists() {
        std::fs::copy(path, &backup)
            .with_context(|| format!("备份配置文件失败：{}", path.display()))?;
    }

    let temp = path.with_extension(format!(
        "{}ccp.tmp",
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| format!("{ext}."))
            .unwrap_or_default()
    ));
    std::fs::write(&temp, bytes)
        .with_context(|| format!("写入临时文件失败：{}", temp.display()))?;
    if let Err(error) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        return Err(anyhow::Error::new(error))
            .with_context(|| format!("替换配置文件失败：{}", path.display()));
    }
    Ok(())
}

/// `<path>.ccp-first-write.bak`, the byte-for-byte original of the first write.
pub fn first_write_backup_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(".ccp-first-write.bak");
    path.with_file_name(name)
}

/// Read a config file, treating a missing file as `None`. A UTF-8 BOM is
/// stripped so JSON5/JSON parsers never choke on it.
pub(crate) fn read_optional(path: &Path) -> anyhow::Result<Option<String>> {
    match std::fs::read(path) {
        Ok(bytes) => {
            let text = String::from_utf8(bytes)
                .with_context(|| format!("配置文件不是 UTF-8 编码：{}", path.display()))?;
            Ok(Some(text.trim_start_matches('\u{feff}').to_string()))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(anyhow::Error::new(error))
            .with_context(|| format!("读取配置文件失败：{}", path.display())),
    }
}

/// Parse `text` as a JSON object, or fail with the app-specific message.
pub(crate) fn parse_object(
    app_label: &str,
    text: &str,
) -> anyhow::Result<serde_json::Map<String, serde_json::Value>> {
    let value: serde_json::Value = json5::from_str(text)
        .map_err(|_| anyhow::anyhow!("{app_label} 配置文件无法解析，未做修改"))?;
    match value {
        serde_json::Value::Object(map) => Ok(map),
        _ => bail!("{app_label} 配置文件无法解析，未做修改"),
    }
}

/// Serialize an object back to the file, pretty printed.
pub(crate) fn write_object(
    path: &Path,
    map: &serde_json::Map<String, serde_json::Value>,
) -> anyhow::Result<()> {
    let mut text = serde_json::to_string_pretty(&serde_json::Value::Object(map.clone()))?;
    text.push('\n');
    write_with_backup(path, text.as_bytes())
}

/// The `api` field value OpenClaw and Pi expect for a format.
pub(crate) fn api_name(format: ApiFormat) -> &'static str {
    match format {
        ApiFormat::OpenaiChat => "openai-completions",
        ApiFormat::OpenaiResponses => "openai-responses",
        ApiFormat::Anthropic => "anthropic-messages",
        ApiFormat::GeminiNative => "gemini-native",
    }
}

/// A provider value as a JSON object, ready for `as_object_mut`.
pub(crate) fn object_or_reset<'a>(
    root: &'a mut serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> &'a mut serde_json::Map<String, serde_json::Value> {
    let entry = root
        .entry(key.to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    if !entry.is_object() {
        *entry = serde_json::Value::Object(serde_json::Map::new());
    }
    entry.as_object_mut().expect("刚写入的对象")
}

/// The chat-completions URL derived from a supplier's base URL, appending the
/// suffix once and only once.
pub(crate) fn chat_completions_url(base_url: &str) -> String {
    const SUFFIX: &str = "/chat/completions";
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.ends_with(SUFFIX) {
        trimmed.to_string()
    } else {
        format!("{trimmed}{SUFFIX}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn temp_dir(label: &str) -> PathBuf {
        let unique = format!(
            "ccp-agent-providers-{label}-{}-{}",
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

    fn provider(app_id: &str) -> AgentProvider {
        AgentProvider {
            id: String::new(),
            app_id: app_id.to_string(),
            name: "测试供应商".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: Some("sk-test-placeholder".to_string()),
            has_api_key: false,
            api_format: allowed_formats(app_id)
                .first()
                .copied()
                .unwrap_or(ApiFormat::OpenaiChat),
            models: vec!["model-a".to_string()],
            default_model: String::new(),
            notes: String::new(),
            sort_index: 0,
        }
    }

    fn store(label: &str) -> (AgentProviderStore, PathBuf) {
        let dir = temp_dir(label);
        let store = AgentProviderStore::open(&dir.join("ccp.db")).expect("打开测试数据库");
        (store, dir)
    }

    #[test]
    fn validate_accepts_a_well_formed_provider() {
        let mut candidate = provider("workbuddy");
        normalize(&mut candidate);
        validate(&candidate, true).expect("合法供应商应通过校验");
        // Switch-mode apps fill the default model from the first model.
        let mut switching = provider("gemini");
        switching.api_format = ApiFormat::GeminiNative;
        normalize(&mut switching);
        assert_eq!(switching.default_model, "model-a");
        validate(&switching, true).expect("switch 模式自动填充默认模型");
    }

    #[test]
    fn validate_rejects_each_broken_field() {
        let mut good = provider("workbuddy");
        normalize(&mut good);
        if let Err(error) = validate(&good, true) {
            panic!("基准供应商应合法：{error}");
        }

        let mut unknown_app = good.clone();
        unknown_app.app_id = "not-an-agent".to_string();
        assert!(
            validate(&unknown_app, true).is_err(),
            "未知 app_id 必须失败"
        );

        let mut blank_name = good.clone();
        blank_name.name = "   ".to_string();
        assert!(validate(&blank_name, true).is_err(), "空名称必须失败");

        let mut long_name = good.clone();
        long_name.name = "名".repeat(81);
        assert!(validate(&long_name, true).is_err(), "超长名称必须失败");

        let mut at_limit = good.clone();
        at_limit.name = "名".repeat(80);
        assert!(validate(&at_limit, true).is_ok(), "80 字名称应通过");

        for bad_url in ["", "ftp://api.example.com", "api.example.com", "https://"] {
            let mut candidate = good.clone();
            candidate.base_url = bad_url.to_string();
            assert!(validate(&candidate, true).is_err(), "非法 URL 必须失败");
        }

        let mut no_models = good.clone();
        no_models.models = Vec::new();
        assert!(validate(&no_models, true).is_err(), "空模型列表必须失败");

        let mut blank_model = good.clone();
        blank_model.models = vec!["  ".to_string()];
        assert!(validate(&blank_model, true).is_err(), "空白模型必须失败");

        let mut wrong_format = good.clone();
        wrong_format.api_format = ApiFormat::Anthropic;
        assert!(
            validate(&wrong_format, true).is_err(),
            "workbuddy 不支持 anthropic"
        );

        let mut no_default = provider("gemini");
        no_default.models = vec!["a".to_string(), "b".to_string()];
        no_default.default_model = "c".to_string();
        normalize(&mut no_default);
        assert!(
            validate(&no_default, true).is_err(),
            "默认模型不在列表中必须失败"
        );

        let mut missing_key = good.clone();
        missing_key.api_key = None;
        assert!(validate(&good, false).is_err(), "缺少 API key 必须失败");

        // 错误信息为中文。
        let error = validate(&unknown_app, true).expect_err("未知应用应报错");
        assert!(
            error
                .to_string()
                .chars()
                .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)),
            "错误信息必须是中文"
        );
    }

    #[test]
    fn normalize_trims_dedups_and_fills_default_model() {
        let mut candidate = provider("workbuddy");
        candidate.name = "  供应商  ".to_string();
        candidate.base_url = "https://api.example.com/v1/".to_string();
        candidate.models = vec![
            " a ".to_string(),
            "a".to_string(),
            "b".to_string(),
            "  ".to_string(),
        ];
        normalize(&mut candidate);
        assert_eq!(candidate.name, "供应商");
        assert_eq!(candidate.base_url, "https://api.example.com/v1");
        assert_eq!(candidate.models, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn store_roundtrip_hides_key_and_keeps_it_on_resave() {
        let (mut store, dir) = store("store-roundtrip");

        let mut candidate = provider("opencode");
        let saved = store.save(&mut candidate).expect("保存新供应商");
        assert!(saved.id.starts_with("ap-"), "id 前缀应为 ap-");
        assert_eq!(saved.id.len(), 15, "ap- 加 12 位 uuid");
        assert!(saved.has_api_key, "保存后应标记已存 key");
        assert!(saved.api_key.is_none(), "返回值不得携带 api_key");

        let listed = store.list().expect("列出供应商");
        assert_eq!(listed.len(), 1);
        assert!(listed[0].has_api_key);
        assert!(listed[0].api_key.is_none());

        let serialized = serde_json::to_string(&listed[0]).expect("序列化供应商");
        assert!(
            !serialized.contains("apiKey") && !serialized.contains("sk-test-placeholder"),
            "序列化结果不得包含 api_key"
        );
        assert!(serialized.contains("hasApiKey"));

        let (_, key) = store
            .get_with_key(&saved.id)
            .expect("读取供应商")
            .expect("供应商存在");
        assert_eq!(key, "sk-test-placeholder");

        // 保留旧 key：再次保存时没有传 key。
        let mut without_key = saved.clone();
        without_key.name = "改名后".to_string();
        without_key.api_key = None;
        let updated = store.save(&mut without_key).expect("保存不带 key 的供应商");
        assert_eq!(updated.name, "改名后");
        assert!(updated.has_api_key, "旧 key 必须保留");
        let (_, key) = store
            .get_with_key(&saved.id)
            .expect("再次读取")
            .expect("供应商存在");
        assert_eq!(key, "sk-test-placeholder");

        // New rows go to the end of their app's list.
        let mut second = provider("opencode");
        let second = store.save(&mut second).expect("保存第二个供应商");
        assert!(second.sort_index > updated.sort_index);

        store
            .reorder("opencode", &[second.id.clone(), saved.id.clone()])
            .expect("重排供应商");
        let ordered: Vec<String> = store
            .list()
            .expect("列出供应商")
            .into_iter()
            .map(|provider| provider.id)
            .collect();
        assert_eq!(ordered, vec![second.id.clone(), saved.id.clone()]);

        store.delete(&saved.id).expect("删除供应商");
        let remaining = store.list().expect("列出供应商");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, second.id);

        drop(store);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn store_keeps_provider_state_per_app() {
        let (mut store, dir) = store("store-state");

        let empty = store.state("opencode").expect("读取初始状态");
        assert!(empty.active_id.is_none());
        assert!(empty.applied_ids.is_empty());

        store
            .set_active("opencode", Some("ap-1234567890ab"))
            .expect("写入 active");
        store
            .set_applied(
                "opencode",
                &["ap-1234567890ab".to_string(), "ap-bbbbbbbbbbbb".to_string()],
            )
            .expect("写入 applied");

        let state = store.state("opencode").expect("读取状态");
        assert_eq!(state.active_id.as_deref(), Some("ap-1234567890ab"));
        assert_eq!(state.applied_ids.len(), 2);

        // set_active 不得清掉已应用的列表。
        store
            .set_active("opencode", Some("ap-bbbbbbbbbbbb"))
            .expect("更新 active");
        assert_eq!(
            store.state("opencode").expect("读取状态").applied_ids.len(),
            2
        );

        drop(store);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_modes_and_formats_match_the_contract() {
        assert_eq!(apply_mode("gemini"), Some(ApplyMode::Switch));
        assert_eq!(apply_mode("grok"), Some(ApplyMode::Switch));
        assert_eq!(apply_mode("cursor"), Some(ApplyMode::Manual));
        assert_eq!(apply_mode("opencode"), Some(ApplyMode::Additive));
        assert_eq!(apply_mode("workbuddy"), Some(ApplyMode::Additive));
        assert_eq!(apply_mode("nope"), None);

        assert_eq!(allowed_formats("workbuddy"), &[ApiFormat::OpenaiChat]);
        assert_eq!(allowed_formats("cursor"), &[ApiFormat::OpenaiChat]);
        assert_eq!(allowed_formats("gemini"), &[ApiFormat::GeminiNative]);
        assert_eq!(
            allowed_formats("grok"),
            &[ApiFormat::OpenaiResponses, ApiFormat::OpenaiChat]
        );
        assert!(allowed_formats("nope").is_empty());

        assert_eq!(
            serde_json::to_string(&ApiFormat::OpenaiResponses).expect("序列化协议"),
            "\"openai-responses\""
        );
        assert_eq!(
            serde_json::to_string(&ApiFormat::GeminiNative).expect("序列化协议"),
            "\"gemini-native\""
        );
    }

    #[test]
    fn write_with_backup_keeps_the_first_original_only() {
        let dir = temp_dir("backup");
        let path = dir.join("nested").join("config.json");

        write_with_backup(&path, b"{\"v\":1}").expect("首次写入");
        assert_eq!(
            std::fs::read_to_string(&path).expect("读取文件"),
            "{\"v\":1}"
        );
        assert!(
            !first_write_backup_path(&path).exists(),
            "首次创建没有原文件可备份"
        );

        write_with_backup(&path, b"{\"v\":2}").expect("第二次写入");
        let backup = first_write_backup_path(&path);
        assert!(backup.is_file(), "第二次写入必须生成首次备份");
        assert_eq!(
            std::fs::read_to_string(&backup).expect("读取备份"),
            "{\"v\":1}"
        );

        write_with_backup(&path, b"{\"v\":3}").expect("第三次写入");
        assert_eq!(
            std::fs::read_to_string(&backup).expect("读取备份"),
            "{\"v\":1}",
            "备份只能写一次"
        );
        assert_eq!(
            std::fs::read_to_string(&path).expect("读取文件"),
            "{\"v\":3}"
        );
        assert!(
            !path.with_extension("json.ccp.tmp").exists(),
            "临时文件必须被重命名掉"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_optional_strips_bom_and_tolerates_missing_files() {
        let dir = temp_dir("read-optional");
        let path = dir.join("config.json");
        assert!(read_optional(&path).expect("缺失文件").is_none());

        std::fs::write(&path, "\u{feff}{\"a\":1}").expect("写入带 BOM 的文件");
        assert_eq!(
            read_optional(&path).expect("读取文件").as_deref(),
            Some("{\"a\":1}")
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn chat_completions_url_appends_the_suffix_once() {
        assert_eq!(
            chat_completions_url("https://api.example.com/v1/"),
            "https://api.example.com/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_url("https://api.example.com/v1/chat/completions"),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn default_dir_and_writer_lookup_follow_the_contract() {
        assert!(default_dir("cursor").is_none());
        assert!(writer_for("cursor").is_none());
        assert!(writer_for("gemini").is_none(), "任务 7b 才实现 gemini");
        assert!(writer_for("workbuddy").is_some());
        assert!(writer_for("opencode").is_some());
        assert!(writer_for("openclaw").is_some());
        assert!(writer_for("pi").is_some());
        if let Some(home) = home_dir() {
            assert_eq!(default_dir("gemini"), Some(home.join(".gemini")));
            assert_eq!(
                default_dir("opencode"),
                Some(home.join(".config").join("opencode"))
            );
        }
    }
}
