//! OpenClaw `~/.openclaw/openclaw.json` (JSON5).

use std::path::{Path, PathBuf};

use super::{
    AgentProvider, AgentWriter, api_name, object_or_reset, parse_object, read_optional,
    write_object,
};

const CONFIG_FILE: &str = "openclaw.json";

pub struct OpenClawWriter;

impl AgentWriter for OpenClawWriter {
    fn config_path(&self, dir: &Path) -> PathBuf {
        dir.join(CONFIG_FILE)
    }

    fn apply(&self, dir: &Path, provider: &AgentProvider, api_key: &str) -> anyhow::Result<()> {
        let path = self.config_path(dir);
        let mut root = match read_optional(&path)? {
            Some(text) => parse_object("OpenClaw", &text)?,
            None => serde_json::Map::new(),
        };

        let models = object_or_reset(&mut root, "models");
        models
            .entry("mode".to_string())
            .or_insert_with(|| serde_json::Value::String("merge".to_string()));
        let providers = object_or_reset(models, "providers");

        let entries: Vec<serde_json::Value> = provider
            .models
            .iter()
            .map(|model| serde_json::json!({ "id": model, "name": model }))
            .collect();
        providers.insert(
            provider.id.clone(),
            serde_json::json!({
                "baseUrl": provider.base_url.trim(),
                "apiKey": api_key,
                "api": api_name(provider.api_format),
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
        let mut root = parse_object("OpenClaw", &text)?;
        let removed = root
            .get_mut("models")
            .and_then(|models| models.as_object_mut())
            .and_then(|models| models.get_mut("providers"))
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
            app_id: "openclaw".to_string(),
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
        let dir = temp_dir("openclaw-apply");
        let path = dir.join(CONFIG_FILE);
        std::fs::write(
            &path,
            "{\n  // 用户注释\n  \"gateway\": { \"port\": 1234 },\n  \"models\": { \"providers\": { \"user\": { \"baseUrl\": \"https://user.example\" } } }\n}\n",
        )
        .expect("写入用户文件");

        let writer = OpenClawWriter;
        writer
            .apply(
                &dir,
                &provider("ap-1", ApiFormat::OpenaiResponses),
                "sk-test-placeholder",
            )
            .expect("apply");

        let root: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("读取文件"))
                .expect("解析文件");
        assert_eq!(root["gateway"]["port"], 1234, "用户字段必须保留");
        assert_eq!(root["models"]["mode"], "merge", "mode 缺失时补 merge");
        assert_eq!(
            root["models"]["providers"]["user"]["baseUrl"],
            "https://user.example"
        );
        assert_eq!(
            root["models"]["providers"]["ap-1"]["baseUrl"],
            "https://api.example.com/v1"
        );
        assert_eq!(
            root["models"]["providers"]["ap-1"]["api"],
            "openai-responses"
        );
        assert_eq!(
            root["models"]["providers"]["ap-1"]["apiKey"],
            "sk-test-placeholder"
        );
        assert_eq!(
            root["models"]["providers"]["ap-1"]["models"][0]["id"],
            "model-a"
        );
        assert_eq!(
            root["models"]["providers"]["ap-1"]["models"][1]["name"],
            "model-b"
        );

        writer
            .apply(
                &dir,
                &provider("ap-1", ApiFormat::Anthropic),
                "sk-test-placeholder",
            )
            .expect("再次 apply");
        let root: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("读取文件"))
                .expect("解析文件");
        assert_eq!(
            root["models"]["providers"]
                .as_object()
                .expect("providers")
                .len(),
            2,
            "重复 apply 不得新增条目"
        );
        assert_eq!(
            root["models"]["providers"]["ap-1"]["api"],
            "anthropic-messages"
        );
        assert_eq!(root["gateway"]["port"], 1234);

        writer.unapply(&dir, "ap-1").expect("unapply");
        let root: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("读取文件"))
                .expect("解析文件");
        assert!(root["models"]["providers"].get("ap-1").is_none());
        assert_eq!(
            root["models"]["providers"]["user"]["baseUrl"],
            "https://user.example"
        );
        assert_eq!(root["gateway"]["port"], 1234);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn existing_mode_is_preserved_and_missing_file_is_created() {
        let dir = temp_dir("openclaw-mode");
        let path = dir.join(CONFIG_FILE);
        std::fs::write(&path, "{\"models\":{\"mode\":\"replace\"}}").expect("写入文件");

        let writer = OpenClawWriter;
        writer
            .apply(
                &dir,
                &provider("ap-1", ApiFormat::OpenaiChat),
                "sk-test-placeholder",
            )
            .expect("apply");
        let root: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("读取文件"))
                .expect("解析文件");
        assert_eq!(root["models"]["mode"], "replace", "已有 mode 不得被覆盖");
        assert_eq!(
            root["models"]["providers"]["ap-1"]["api"],
            "openai-completions"
        );

        let missing_dir = temp_dir("openclaw-missing");
        writer
            .apply(
                &missing_dir,
                &provider("ap-2", ApiFormat::OpenaiChat),
                "sk-test-placeholder",
            )
            .expect("在缺失文件上 apply");
        let root: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(missing_dir.join(CONFIG_FILE)).expect("读取文件"),
        )
        .expect("解析文件");
        assert_eq!(root["models"]["mode"], "merge");
        assert_eq!(
            root["models"]["providers"]["ap-2"]["models"]
                .as_array()
                .expect("models")
                .len(),
            2
        );

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&missing_dir);
    }

    #[test]
    fn unparsable_file_is_left_untouched() {
        let dir = temp_dir("openclaw-broken");
        let path = dir.join(CONFIG_FILE);
        let original = "{ \"models\": { \"providers\": ";
        std::fs::write(&path, original).expect("写入损坏文件");

        let error = OpenClawWriter
            .apply(
                &dir,
                &provider("ap-1", ApiFormat::OpenaiChat),
                "sk-test-placeholder",
            )
            .expect_err("损坏文件必须报错");
        assert!(
            error
                .to_string()
                .contains("OpenClaw 配置文件无法解析，未做修改")
        );
        assert!(error.to_string().contains("未做修改"), "错误信息应为中文");
        assert_eq!(std::fs::read_to_string(&path).expect("读取文件"), original);
        assert!(OpenClawWriter.unapply(&dir, "ap-1").is_err());
        assert_eq!(std::fs::read_to_string(&path).expect("读取文件"), original);

        std::fs::write(&path, "\"just a string\"").expect("写入非对象根");
        assert!(
            OpenClawWriter
                .apply(&dir, &provider("ap-1", ApiFormat::OpenaiChat), "k")
                .is_err()
        );
        assert_eq!(
            std::fs::read_to_string(&path).expect("读取文件"),
            "\"just a string\""
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
