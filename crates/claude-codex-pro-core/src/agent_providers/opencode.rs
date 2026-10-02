//! OpenCode config: `opencode.jsonc` when present, otherwise `opencode.json`.

use std::path::{Path, PathBuf};

use super::{
    AgentProvider, AgentWriter, ApiFormat, read_optional, write_object, write_with_backup,
};

const JSONC_FILE: &str = "opencode.jsonc";
const JSON_FILE: &str = "opencode.json";
const SCHEMA: &str = "https://opencode.ai/config.json";

pub struct OpenCodeWriter;

impl OpenCodeWriter {
    /// The file OpenCode actually reads: `.jsonc` wins when it exists.
    fn existing_path(&self, dir: &Path) -> PathBuf {
        let jsonc = dir.join(JSONC_FILE);
        if jsonc.is_file() {
            jsonc
        } else {
            dir.join(JSON_FILE)
        }
    }
}

/// The npm package that backs a given wire format.
fn package_for(format: ApiFormat) -> &'static str {
    match format {
        ApiFormat::OpenaiChat => "@ai-sdk/openai-compatible",
        ApiFormat::Anthropic => "@ai-sdk/anthropic",
        ApiFormat::OpenaiResponses => "@ai-sdk/openai",
        // OpenCode 不支持原生 Gemini 协议，这里只是兜底。
        ApiFormat::GeminiNative => "@ai-sdk/openai-compatible",
    }
}

impl AgentWriter for OpenCodeWriter {
    fn config_path(&self, dir: &Path) -> PathBuf {
        self.existing_path(dir)
    }

    fn apply(&self, dir: &Path, provider: &AgentProvider, api_key: &str) -> anyhow::Result<()> {
        let path = self.existing_path(dir);

        // 用 json5 解析，因此带注释的 .jsonc 可以读入。写回使用 serde_json
        // 美化输出，注释会丢失——所以第一次写入前的字节级备份
        // (`<path>.ccp-first-write.bak`) 是注释的唯一保护。
        let mut root = match read_optional(&path)? {
            Some(text) => super::parse_object("OpenCode", &text)?,
            None => {
                if !dir.exists() {
                    std::fs::create_dir_all(dir)?;
                }
                write_with_backup(&path, format!("{{\"$schema\":\"{SCHEMA}\"}}\n").as_bytes())?;
                super::parse_object("OpenCode", &format!("{{\"$schema\":\"{SCHEMA}\"}}"))?
            }
        };

        let providers = super::object_or_reset(&mut root, "provider");
        let mut models = serde_json::Map::new();
        for model in &provider.models {
            models.insert(model.clone(), serde_json::json!({ "name": model }));
        }
        providers.insert(
            provider.id.clone(),
            serde_json::json!({
                "npm": package_for(provider.api_format),
                "name": provider.name,
                "options": { "baseURL": provider.base_url.trim(), "apiKey": api_key },
                "models": serde_json::Value::Object(models),
            }),
        );

        write_object(&path, &root)
    }

    fn unapply(&self, dir: &Path, provider_id: &str) -> anyhow::Result<()> {
        let path = self.existing_path(dir);
        let Some(text) = read_optional(&path)? else {
            return Ok(());
        };
        let mut root = super::parse_object("OpenCode", &text)?;
        let removed = root
            .get_mut("provider")
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
    use crate::agent_providers::tests::temp_dir;

    fn provider(id: &str, name: &str, format: ApiFormat) -> AgentProvider {
        AgentProvider {
            id: id.to_string(),
            app_id: "opencode".to_string(),
            name: name.to_string(),
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
    fn jsonc_is_preferred_and_comments_do_not_break_parsing() {
        let dir = temp_dir("opencode-jsonc");
        let jsonc = dir.join(JSONC_FILE);
        std::fs::write(
            &jsonc,
            "{\n  // 用户自己的注释\n  \"$schema\": \"https://opencode.ai/config.json\",\n  \"theme\": \"dark\"\n}\n",
        )
        .expect("写入 jsonc");
        std::fs::write(dir.join(JSON_FILE), "{\"theme\":\"light\"}").expect("写入 json");

        let writer = OpenCodeWriter;
        assert_eq!(writer.config_path(&dir), jsonc);
        writer
            .apply(
                &dir,
                &provider("ap-1", "供应商一", ApiFormat::OpenaiChat),
                "sk-test-placeholder",
            )
            .expect("apply 到 jsonc");

        let root: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&jsonc).expect("读取 jsonc"))
                .expect("解析写回内容");
        assert_eq!(root["theme"], "dark", "用户字段必须保留");
        assert_eq!(root["provider"]["ap-1"]["name"], "供应商一");
        assert_eq!(root["provider"]["ap-1"]["npm"], "@ai-sdk/openai-compatible");
        assert_eq!(
            root["provider"]["ap-1"]["options"]["baseURL"],
            "https://api.example.com/v1"
        );
        assert_eq!(
            root["provider"]["ap-1"]["options"]["apiKey"],
            "sk-test-placeholder"
        );
        assert_eq!(
            root["provider"]["ap-1"]["models"]["model-a"]["name"],
            "model-a"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join(JSON_FILE)).expect("读取 json"),
            "{\"theme\":\"light\"}",
            "json 文件不得被改动"
        );

        // 再次 apply 不产生重复键，只替换自己的节点。
        writer
            .apply(
                &dir,
                &provider("ap-1", "供应商一", ApiFormat::OpenaiChat),
                "sk-test-placeholder",
            )
            .expect("再次 apply");
        let root: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&jsonc).expect("读取 jsonc"))
                .expect("解析写回内容");
        assert_eq!(
            root["provider"].as_object().expect("provider 对象").len(),
            1
        );
        assert_eq!(root["theme"], "dark");

        writer.unapply(&dir, "ap-1").expect("unapply");
        let root: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&jsonc).expect("读取 jsonc"))
                .expect("解析写回内容");
        assert!(
            root["provider"]
                .as_object()
                .expect("provider 对象")
                .is_empty()
        );
        assert_eq!(root["theme"], "dark", "unapply 后用户字段仍在");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn anthropic_and_responses_use_their_own_package() {
        let dir = temp_dir("opencode-packages");
        let writer = OpenCodeWriter;
        writer
            .apply(
                &dir,
                &provider("ap-a", "A", ApiFormat::Anthropic),
                "sk-test-placeholder",
            )
            .expect("apply anthropic");
        writer
            .apply(
                &dir,
                &provider("ap-r", "R", ApiFormat::OpenaiResponses),
                "sk-test-placeholder",
            )
            .expect("apply responses");

        let root: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join(JSON_FILE)).expect("读取文件"))
                .expect("解析文件");
        assert_eq!(root["$schema"], SCHEMA, "新建文件必须带上 schema");
        assert_eq!(root["provider"]["ap-a"]["npm"], "@ai-sdk/anthropic");
        assert_eq!(root["provider"]["ap-r"]["npm"], "@ai-sdk/openai");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn non_object_provider_node_is_reset() {
        let dir = temp_dir("opencode-reset");
        let path = dir.join(JSON_FILE);
        std::fs::write(&path, "{\"provider\":\"oops\",\"theme\":\"dark\"}").expect("写入文件");

        OpenCodeWriter
            .apply(
                &dir,
                &provider("ap-1", "供应商", ApiFormat::OpenaiChat),
                "sk-test-placeholder",
            )
            .expect("apply");
        let root: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("读取文件"))
                .expect("解析文件");
        assert_eq!(root["provider"]["ap-1"]["name"], "供应商");
        assert_eq!(root["theme"], "dark");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unparsable_file_is_left_untouched() {
        let dir = temp_dir("opencode-broken");
        let path = dir.join(JSON_FILE);
        let original = "{ not json at all ";
        std::fs::write(&path, original).expect("写入损坏文件");

        let error = OpenCodeWriter
            .apply(
                &dir,
                &provider("ap-1", "供应商", ApiFormat::OpenaiChat),
                "sk-test-placeholder",
            )
            .expect_err("损坏文件必须报错");
        assert!(
            error
                .to_string()
                .contains("OpenCode 配置文件无法解析，未做修改")
        );
        assert_eq!(std::fs::read_to_string(&path).expect("读取文件"), original);
        assert!(
            !super::super::first_write_backup_path(&path).exists(),
            "解析失败不得写入备份或新内容"
        );

        std::fs::write(&path, "[1,2,3]").expect("写入数组根");
        assert!(
            OpenCodeWriter
                .apply(
                    &dir,
                    &provider("ap-1", "供应商", ApiFormat::OpenaiChat),
                    "k"
                )
                .is_err()
        );
        assert_eq!(std::fs::read_to_string(&path).expect("读取文件"), "[1,2,3]");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
