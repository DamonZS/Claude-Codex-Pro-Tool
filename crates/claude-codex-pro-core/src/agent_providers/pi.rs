//! Pi `~/.pi/agent/models.json` (JSON5).

use std::path::{Path, PathBuf};

use super::{
    AgentProvider, AgentWriter, api_name, object_or_reset, parse_object, read_optional,
    write_object,
};

const CONFIG_FILE: &str = "models.json";

/// Pi refuses to touch config files bigger than this; so do we.
const MAX_CONFIG_BYTES: usize = 1024 * 1024;

pub struct PiWriter;

impl AgentWriter for PiWriter {
    fn config_path(&self, dir: &Path) -> PathBuf {
        dir.join(CONFIG_FILE)
    }

    fn apply(&self, dir: &Path, provider: &AgentProvider, api_key: &str) -> anyhow::Result<()> {
        let path = self.config_path(dir);
        let text = read_optional(&path)?;
        if text
            .as_ref()
            .is_some_and(|text| text.len() > MAX_CONFIG_BYTES)
        {
            anyhow::bail!("Pi 配置文件过大（超过 1 MiB），未做修改");
        }
        let mut root = match text {
            Some(text) => parse_object("Pi", &text)?,
            None => serde_json::Map::new(),
        };

        let providers = object_or_reset(&mut root, "providers");
        let entries: Vec<serde_json::Value> = provider
            .models
            .iter()
            .map(|model| serde_json::json!({ "id": model }))
            .collect();
        providers.insert(
            provider.id.clone(),
            serde_json::json!({
                "name": provider.name,
                "baseUrl": provider.base_url.trim(),
                "api": api_name(provider.api_format),
                "apiKey": api_key,
                "models": entries,
            }),
        );

        write_object(&path, &root)
    }

    fn unapply(&self, dir: &Path, provider_id: &str) -> anyhow::Result<()> {
        let path = self.config_path(dir);
        let Some(text) = read_optional(&path)? else {
            return Ok(());
        };
        if text.len() > MAX_CONFIG_BYTES {
            anyhow::bail!("Pi 配置文件过大（超过 1 MiB），未做修改");
        }
        let mut root = parse_object("Pi", &text)?;
        let removed = root
            .get_mut("providers")
            .and_then(|providers| providers.as_object_mut())
            .map(|providers| providers.remove(provider_id).is_some())
            .unwrap_or(false);
        if !removed {
            return Ok(());
        }
        write_object(&path, &root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_providers::ApiFormat;
    use crate::agent_providers::tests::temp_dir;

    fn provider(id: &str, format: ApiFormat) -> AgentProvider {
        AgentProvider {
            id: id.to_string(),
            app_id: "pi".to_string(),
            name: "供应商".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: None,
            has_api_key: true,
            api_format: format,
            models: vec!["model-a".to_string(), "model-b".to_string()],
            default_model: String::new(),
            notes: String::new(),
            sort_index: 0,
        }
    }

    #[test]
    fn apply_keeps_user_config_and_is_idempotent() {
        let dir = temp_dir("pi-apply");
        let path = dir.join(CONFIG_FILE);
        std::fs::write(
            &path,
            "{\n  // 用户注释\n  \"theme\": \"dark\",\n  \"providers\": { \"user\": { \"name\": \"用户自己的\" } }\n}\n",
        )
        .expect("写入用户文件");

        let writer = PiWriter;
        writer
            .apply(
                &dir,
                &provider("ap-1", ApiFormat::Anthropic),
                "sk-test-placeholder",
            )
            .expect("apply");

        let root: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("读取文件"))
                .expect("解析文件");
        assert_eq!(root["theme"], "dark", "用户字段必须保留");
        assert_eq!(root["providers"]["user"]["name"], "用户自己的");
        assert_eq!(root["providers"]["ap-1"]["name"], "供应商");
        assert_eq!(
            root["providers"]["ap-1"]["baseUrl"],
            "https://api.example.com/v1"
        );
        assert_eq!(root["providers"]["ap-1"]["api"], "anthropic-messages");
        assert_eq!(root["providers"]["ap-1"]["apiKey"], "sk-test-placeholder");
        assert_eq!(root["providers"]["ap-1"]["models"][0]["id"], "model-a");
        assert_eq!(root["providers"]["ap-1"]["models"][1]["id"], "model-b");

        writer
            .apply(
                &dir,
                &provider("ap-1", ApiFormat::OpenaiChat),
                "sk-test-placeholder",
            )
            .expect("再次 apply");
        let root: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("读取文件"))
                .expect("解析文件");
        assert_eq!(
            root["providers"].as_object().expect("providers").len(),
            2,
            "重复 apply 不得新增条目"
        );
        assert_eq!(root["providers"]["ap-1"]["api"], "openai-completions");
        assert_eq!(
            root["providers"]["ap-1"]["models"]
                .as_array()
                .expect("models")
                .len(),
            2
        );

        writer.unapply(&dir, "ap-1").expect("unapply");
        let root: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("读取文件"))
                .expect("解析文件");
        assert!(root["providers"].get("ap-1").is_none());
        assert_eq!(root["providers"]["user"]["name"], "用户自己的");
        assert_eq!(root["theme"], "dark");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn oversized_file_is_rejected() {
        let dir = temp_dir("pi-oversized");
        let path = dir.join(CONFIG_FILE);
        let huge = format!("{{\"pad\":\"{}\"}}", "x".repeat(MAX_CONFIG_BYTES));
        std::fs::write(&path, &huge).expect("写入超大文件");

        let error = PiWriter
            .apply(
                &dir,
                &provider("ap-1", ApiFormat::OpenaiChat),
                "sk-test-placeholder",
            )
            .expect_err("超大文件必须拒绝");
        assert!(error.to_string().contains("1 MiB"), "错误信息应说明上限");
        assert_eq!(std::fs::read_to_string(&path).expect("读取文件"), huge);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unparsable_file_is_left_untouched() {
        let dir = temp_dir("pi-broken");
        let path = dir.join(CONFIG_FILE);
        let original = "{ \"providers\": [ oops";
        std::fs::write(&path, original).expect("写入损坏文件");

        let error = PiWriter
            .apply(
                &dir,
                &provider("ap-1", ApiFormat::OpenaiChat),
                "sk-test-placeholder",
            )
            .expect_err("损坏文件必须报错");
        assert!(error.to_string().contains("Pi 配置文件无法解析，未做修改"));
        assert_eq!(std::fs::read_to_string(&path).expect("读取文件"), original);

        std::fs::write(&path, "42").expect("写入非对象根");
        assert!(
            PiWriter
                .apply(&dir, &provider("ap-1", ApiFormat::OpenaiChat), "k")
                .is_err()
        );
        assert_eq!(std::fs::read_to_string(&path).expect("读取文件"), "42");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
