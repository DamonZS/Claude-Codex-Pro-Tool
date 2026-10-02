//! Grok Build `~/.grok/config.toml`, patched through `toml_edit` so the user's
//! own formatting, comments and tables survive a round trip.
//!
//! Switch mode: CCP keeps exactly one `[model."<id>"]` table, marked with
//! `_ccp_provider = true`, and points `[models].default` at it.

use std::path::{Path, PathBuf};

use super::{AgentProvider, AgentWriter, ApiFormat, read_optional, write_with_backup};

const CONFIG_FILE: &str = "config.toml";

/// Marks the `[model."<x>"]` tables CCP wrote earlier.
const OWNER_KEY: &str = "_ccp_provider";

pub struct GrokWriter;

impl AgentWriter for GrokWriter {
    fn config_path(&self, dir: &Path) -> PathBuf {
        dir.join(CONFIG_FILE)
    }

    fn apply(&self, dir: &Path, provider: &AgentProvider, api_key: &str) -> anyhow::Result<()> {
        let path = self.config_path(dir);
        let mut document: toml_edit::DocumentMut = match read_optional(&path)? {
            Some(text) => text
                .parse()
                .map_err(|_| anyhow::anyhow!("Grok 配置文件无法解析，未做修改"))?,
            None => toml_edit::DocumentMut::new(),
        };

        // 只删除 CCP 自己写过的表，用户手写的表一律保留。
        let mut owned: Vec<String> = Vec::new();
        if let Some(models) = document.get("model").and_then(|item| item.as_table_like()) {
            for (key, value) in models.iter() {
                if value
                    .get(OWNER_KEY)
                    .and_then(|flag| flag.as_bool())
                    .unwrap_or(false)
                {
                    owned.push(key.to_string());
                }
            }
        }
        for key in owned {
            let emptied = {
                let models = document
                    .get_mut("model")
                    .and_then(|item| item.as_table_like_mut())
                    .expect("刚刚遍历过的表");
                models.remove(&key);
                models.is_empty()
            };
            if emptied {
                document.remove("model");
            }
        }

        let model_name = provider.default_model.trim();
        let model_table = document
            .entry("model")
            .or_insert(toml_edit::Item::Table(toml_edit::Table::new()))
            .as_table_like_mut()
            .ok_or_else(|| anyhow::anyhow!("Grok 配置文件无法解析，未做修改"))?;
        let mut entry = toml_edit::Table::new();
        entry.insert("model", toml_edit::value(model_name));
        entry.insert("name", toml_edit::value(provider.name.trim()));
        entry.insert(
            "base_url",
            toml_edit::value(provider.base_url.trim().to_string()),
        );
        entry.insert("api_key", toml_edit::value(api_key));
        entry.insert(
            "api_backend",
            toml_edit::value(match provider.api_format {
                ApiFormat::OpenaiChat => "chat_completions",
                _ => "responses",
            }),
        );
        entry.insert("context_window", toml_edit::value(500_000i64));
        entry.insert(OWNER_KEY, toml_edit::value(true));
        model_table.insert(model_name, toml_edit::Item::Table(entry));

        let models_table = document
            .entry("models")
            .or_insert(toml_edit::Item::Table(toml_edit::Table::new()))
            .as_table_like_mut()
            .ok_or_else(|| anyhow::anyhow!("Grok 配置文件无法解析，未做修改"))?;
        models_table.insert("default", toml_edit::value(model_name));

        write_with_backup(&path, document.to_string().as_bytes())
    }

    /// Switch mode: a no-op. Switching to another supplier replaces the table.
    fn unapply(&self, _dir: &Path, _provider_id: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_providers::tests::temp_dir;

    fn provider(model: &str, format: ApiFormat) -> AgentProvider {
        AgentProvider {
            id: "ap-1".to_string(),
            app_id: "grok".to_string(),
            name: "供应商".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: None,
            has_api_key: true,
            api_format: format,
            models: vec![model.to_string()],
            default_model: model.to_string(),
            notes: String::new(),
            sort_index: 0,
        }
    }

    fn parse(path: &Path) -> toml_edit::DocumentMut {
        std::fs::read_to_string(path)
            .expect("读取 config.toml")
            .parse()
            .expect("解析 config.toml")
    }

    #[test]
    fn apply_keeps_user_tables_and_replaces_the_ccp_table() {
        let dir = temp_dir("grok-apply");
        let path = dir.join(CONFIG_FILE);
        std::fs::write(
            &path,
            "# 用户注释\ntheme = \"dark\"\n\n[models]\ndefault = \"user-model\"\nother = 1\n\n[model.\"user-model\"]\nmodel = \"user-model\"\napi_backend = \"chat_completions\"\n\n[model.\"ap-old\"]\nmodel = \"old\"\n_ccp_provider = true\n",
        )
        .expect("写入用户 config.toml");

        let writer = GrokWriter;
        writer
            .apply(
                &dir,
                &provider("grok-4", ApiFormat::OpenaiResponses),
                "sk-test-placeholder",
            )
            .expect("apply");

        let document = parse(&path);
        assert_eq!(document["theme"].as_str(), Some("dark"), "用户表必须保留");
        assert_eq!(
            document["model"]["user-model"]["api_backend"].as_str(),
            Some("chat_completions"),
            "用户手写的 model 表必须保留"
        );
        assert!(
            document
                .get("model")
                .and_then(|m| m.get("ap-old"))
                .is_none(),
            "CCP 旧表必须被替换"
        );
        assert_eq!(document["models"]["default"].as_str(), Some("grok-4"));
        assert_eq!(document["models"]["other"].as_integer(), Some(1));
        let ccp = &document["model"]["grok-4"];
        assert_eq!(ccp["model"].as_str(), Some("grok-4"));
        assert_eq!(ccp["name"].as_str(), Some("供应商"));
        assert_eq!(ccp["base_url"].as_str(), Some("https://api.example.com/v1"));
        assert_eq!(ccp["api_key"].as_str(), Some("sk-test-placeholder"));
        assert_eq!(ccp["api_backend"].as_str(), Some("responses"));
        assert_eq!(ccp["context_window"].as_integer(), Some(500_000));
        assert_eq!(ccp["_ccp_provider"].as_bool(), Some(true));

        // 再次 apply 不产生重复表，并平滑切换默认模型。
        writer
            .apply(
                &dir,
                &provider("grok-4-fast", ApiFormat::OpenaiChat),
                "sk-test-placeholder",
            )
            .expect("再次 apply");
        let document = parse(&path);
        let ccp_tables = document["model"]
            .as_table_like()
            .expect("model 表")
            .iter()
            .filter(|(_, value)| {
                value
                    .get("_ccp_provider")
                    .and_then(|flag| flag.as_bool())
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(ccp_tables, 1, "只允许存在一张 CCP 表");
        assert_eq!(
            document["model"]["user-model"]["model"].as_str(),
            Some("user-model")
        );
        assert_eq!(document["models"]["default"].as_str(), Some("grok-4-fast"));
        assert_eq!(
            document["model"]["grok-4-fast"]["api_backend"].as_str(),
            Some("chat_completions")
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_on_missing_file_creates_the_tables() {
        let dir = temp_dir("grok-missing");
        GrokWriter
            .apply(
                &dir,
                &provider("grok-4", ApiFormat::OpenaiResponses),
                "sk-test-placeholder",
            )
            .expect("在缺失文件上 apply");
        let document = parse(&dir.join(CONFIG_FILE));
        assert_eq!(document["models"]["default"].as_str(), Some("grok-4"));
        assert_eq!(
            document["model"]["grok-4"]["context_window"].as_integer(),
            Some(500_000)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unparsable_file_is_left_untouched() {
        let dir = temp_dir("grok-broken");
        let path = dir.join(CONFIG_FILE);
        let original = "[models\ndefault = ";
        std::fs::write(&path, original).expect("写入损坏文件");

        let error = GrokWriter
            .apply(
                &dir,
                &provider("grok-4", ApiFormat::OpenaiResponses),
                "sk-test-placeholder",
            )
            .expect_err("损坏文件必须报错");
        assert!(
            error
                .to_string()
                .contains("Grok 配置文件无法解析，未做修改")
        );
        assert_eq!(std::fs::read_to_string(&path).expect("读取文件"), original);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
