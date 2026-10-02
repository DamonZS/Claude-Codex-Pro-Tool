//! Gemini CLI `~/.gemini`: the `.env` key file plus `settings.json`.
//!
//! Switch mode: only one supplier is live at a time. CCP owns the `GOOGLE_*`
//! lines and the four Gemini-specific keys in `.env`, and the
//! `security.auth.selectedType` / `model.name` nodes in `settings.json`.

use std::path::{Path, PathBuf};

use super::{AgentProvider, AgentWriter, object_or_reset, read_optional, write_with_backup};

const ENV_FILE: &str = ".env";
const SETTINGS_FILE: &str = "settings.json";

/// Environment keys Gemini CLI reads, beyond the `GOOGLE_` prefix.
const MANAGED_KEYS: [&str; 4] = [
    "GEMINI_API_KEY",
    "GEMINI_MODEL",
    "GEMINI_API_KEY_AUTH_MECHANISM",
    "GEMINI_CLI_CUSTOM_HEADERS",
];

/// The key a `.env` line assigns, or `None` for comments and blank lines.
fn line_key(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
        return None;
    }
    let (key, _) = trimmed.split_once('=')?;
    let key = key.trim();
    // `export FOO=bar` 也按同一把 key 处理。
    let key = key.strip_prefix("export ").map(str::trim).unwrap_or(key);
    if key.is_empty() { None } else { Some(key) }
}

/// Whether CCP owns this environment key.
fn is_managed(key: &str) -> bool {
    key.starts_with("GOOGLE_") || MANAGED_KEYS.contains(&key)
}

/// Reject values that would break the `.env` line layout.
fn check_value(label: &str, value: &str) -> anyhow::Result<()> {
    if value.contains('\n') || value.contains('\r') {
        anyhow::bail!("{label} 不能包含换行符");
    }
    Ok(())
}

/// Rewrite `.env`: drop the managed lines, keep everything else verbatim.
fn rewrite_env(path: &Path, base_url: &str, api_key: &str, model: &str) -> anyhow::Result<()> {
    check_value("Base URL", base_url)?;
    check_value("API Key", api_key)?;
    check_value("默认模型", model)?;

    let existing = read_optional(path)?.unwrap_or_default();
    let mut lines: Vec<String> = Vec::new();
    for line in existing.lines() {
        if line_key(line).is_some_and(is_managed) {
            continue;
        }
        lines.push(line.to_string());
    }
    // 文件末尾原有的空行不值得保留，统一换成一段干净的新内容。
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }

    lines.push(format!("GOOGLE_GEMINI_BASE_URL={base_url}"));
    lines.push(format!("GEMINI_API_KEY={api_key}"));
    lines.push(format!("GEMINI_MODEL={model}"));

    let mut text = lines.join("\n");
    text.push('\n');
    write_with_backup(path, text.as_bytes())
}

pub struct GeminiWriter;

impl AgentWriter for GeminiWriter {
    fn config_path(&self, dir: &Path) -> PathBuf {
        dir.join(SETTINGS_FILE)
    }

    fn apply(&self, dir: &Path, provider: &AgentProvider, api_key: &str) -> anyhow::Result<()> {
        let model = provider.default_model.trim();
        rewrite_env(
            &dir.join(ENV_FILE),
            provider.base_url.trim(),
            api_key,
            model,
        )?;

        let path = self.config_path(dir);
        let mut root = match read_optional(&path)? {
            Some(text) => super::parse_object("Gemini", &text)?,
            None => serde_json::Map::new(),
        };

        let security = object_or_reset(&mut root, "security");
        let auth = object_or_reset(security, "auth");
        auth.insert(
            "selectedType".to_string(),
            serde_json::Value::String("gemini-api-key".to_string()),
        );

        let model_node = object_or_reset(&mut root, "model");
        model_node.insert(
            "name".to_string(),
            serde_json::Value::String(model.to_string()),
        );

        super::write_object(&path, &root)
    }

    /// Switch mode: switching to another supplier overwrites this one, so there
    /// is nothing to undo in the live file.
    fn unapply(&self, _dir: &Path, _provider_id: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_providers::ApiFormat;
    use crate::agent_providers::tests::temp_dir;

    fn provider(model: &str) -> AgentProvider {
        AgentProvider {
            id: "ap-1".to_string(),
            app_id: "gemini".to_string(),
            name: "供应商".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: None,
            has_api_key: true,
            api_format: ApiFormat::GeminiNative,
            models: vec![model.to_string()],
            default_model: model.to_string(),
            notes: String::new(),
            sort_index: 0,
        }
    }

    fn env_text(dir: &Path) -> String {
        std::fs::read_to_string(dir.join(ENV_FILE)).expect("读取 .env")
    }

    fn managed_line_count(text: &str, key: &str) -> usize {
        text.lines()
            .filter(|line| line_key(line) == Some(key))
            .count()
    }

    #[test]
    fn apply_keeps_user_env_lines_and_replaces_managed_ones() {
        let dir = temp_dir("gemini-env");
        let path = dir.join(ENV_FILE);
        std::fs::write(
            &path,
            "# 用户注释\nHTTP_PROXY=http://127.0.0.1:7890\nGEMINI_API_KEY=old-placeholder\nGOOGLE_CLOUD_PROJECT=my-project\nGEMINI_MODEL=old-model\nHTTPS_PROXY=http://127.0.0.1:7890\n",
        )
        .expect("写入用户 .env");

        let writer = GeminiWriter;
        writer
            .apply(&dir, &provider("gemini-2.5-pro"), "sk-test-placeholder")
            .expect("apply");

        let text = env_text(&dir);
        assert!(text.contains("# 用户注释"), "注释必须保留");
        assert!(
            text.contains("HTTP_PROXY=http://127.0.0.1:7890"),
            "用户代理设置必须保留"
        );
        assert!(text.contains("HTTPS_PROXY=http://127.0.0.1:7890"));
        assert!(
            !text.contains("GOOGLE_CLOUD_PROJECT"),
            "GOOGLE_ 前缀的旧行必须删除"
        );
        assert_eq!(managed_line_count(&text, "GEMINI_API_KEY"), 1);
        assert_eq!(managed_line_count(&text, "GEMINI_MODEL"), 1);
        assert!(
            text.contains("GEMINI_API_KEY=sk-test-placeholder"),
            "新的 key 必须写入"
        );
        assert!(text.contains("GEMINI_MODEL=gemini-2.5-pro"));
        assert!(text.contains("GOOGLE_GEMINI_BASE_URL=https://api.example.com/v1"));

        // 再次 apply 不得重复托管行。
        writer
            .apply(&dir, &provider("gemini-2.5-flash"), "sk-test-placeholder")
            .expect("再次 apply");
        let text = env_text(&dir);
        assert_eq!(managed_line_count(&text, "GEMINI_API_KEY"), 1);
        assert_eq!(managed_line_count(&text, "GEMINI_MODEL"), 1);
        assert_eq!(managed_line_count(&text, "GOOGLE_GEMINI_BASE_URL"), 1);
        assert!(text.contains("GEMINI_MODEL=gemini-2.5-flash"));
        assert!(text.contains("HTTP_PROXY=http://127.0.0.1:7890"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_writes_settings_json_and_keeps_user_nodes() {
        let dir = temp_dir("gemini-settings");
        let path = dir.join(SETTINGS_FILE);
        std::fs::write(
            &path,
            "{\n  // 用户注释\n  \"theme\": \"dark\",\n  \"security\": { \"auth\": { \"selectedType\": \"oauth-personal\", \"extra\": 1 } }\n}\n",
        )
        .expect("写入用户 settings.json");

        let writer = GeminiWriter;
        writer
            .apply(&dir, &provider("gemini-2.5-pro"), "sk-test-placeholder")
            .expect("apply");

        let root: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("读取 settings.json"))
                .expect("解析 settings.json");
        assert_eq!(root["theme"], "dark", "用户字段必须保留");
        assert_eq!(
            root["security"]["auth"]["selectedType"], "gemini-api-key",
            "auth 类型必须被改写"
        );
        assert_eq!(root["security"]["auth"]["extra"], 1, "同级用户字段必须保留");
        assert_eq!(root["model"]["name"], "gemini-2.5-pro");

        // 缺少中间对象时必须自动创建。
        std::fs::write(&path, "{\"theme\":\"light\"}").expect("写入最小文件");
        writer
            .apply(&dir, &provider("gemini-2.5-pro"), "sk-test-placeholder")
            .expect("在最小文件上 apply");
        let root: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("读取 settings.json"))
                .expect("解析 settings.json");
        assert_eq!(root["security"]["auth"]["selectedType"], "gemini-api-key");
        assert_eq!(root["model"]["name"], "gemini-2.5-pro");
        assert_eq!(root["theme"], "light");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unparsable_settings_file_is_left_untouched() {
        let dir = temp_dir("gemini-broken");
        let path = dir.join(SETTINGS_FILE);
        let original = "{ this is not json";
        std::fs::write(&path, original).expect("写入损坏文件");

        let error = GeminiWriter
            .apply(&dir, &provider("gemini-2.5-pro"), "sk-test-placeholder")
            .expect_err("损坏文件必须报错");
        assert!(
            error
                .to_string()
                .contains("Gemini 配置文件无法解析，未做修改")
        );
        assert_eq!(std::fs::read_to_string(&path).expect("读取文件"), original);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn newline_in_values_is_rejected() {
        let dir = temp_dir("gemini-newline");
        assert!(
            GeminiWriter
                .apply(&dir, &provider("gemini-2.5-pro"), "sk-bad\nvalue")
                .is_err(),
            "带换行的 key 必须被拒绝"
        );
        assert!(!dir.join(ENV_FILE).exists(), "被拒绝时不得写入 .env");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unapply_is_a_no_op() {
        let dir = temp_dir("gemini-unapply");
        GeminiWriter
            .apply(&dir, &provider("gemini-2.5-pro"), "sk-test-placeholder")
            .expect("apply");
        let before = env_text(&dir);
        GeminiWriter.unapply(&dir, "ap-1").expect("unapply");
        assert_eq!(env_text(&dir), before, "unapply 不得改动文件");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
