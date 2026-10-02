//! WorkBuddy `~/.workbuddy/models.json`: a plain JSON array of model entries.

use std::path::{Path, PathBuf};

use anyhow::bail;

use super::{AgentProvider, AgentWriter, chat_completions_url, read_optional, write_with_backup};

/// Owner marker written into every element CCP creates.
const OWNER_KEY: &str = "_ccpProvider";

const CONFIG_FILE: &str = "models.json";

pub struct WorkBuddyWriter;

impl AgentWriter for WorkBuddyWriter {
    fn config_path(&self, dir: &Path) -> PathBuf {
        dir.join(CONFIG_FILE)
    }

    fn apply(&self, dir: &Path, provider: &AgentProvider, api_key: &str) -> anyhow::Result<()> {
        let path = self.config_path(dir);
        let mut entries = match read_optional(&path)? {
            Some(text) => parse_array(&text)?,
            // 文件不存在时按空列表处理。
            None => Vec::new(),
        };

        // 只清理自己写过的元素，用户手写的元素保持原样。
        entries.retain(|entry| entry.get(OWNER_KEY).and_then(|v| v.as_str()) != Some(&provider.id));
        let url = chat_completions_url(&provider.base_url);
        for model in &provider.models {
            entries.push(serde_json::json!({
                "id": model,
                "name": model,
                "vendor": "Custom",
                "url": url,
                "apiKey": api_key,
                "supportsToolCall": true,
                "supportsImages": false,
                "supportsReasoning": false,
                "useCustomProtocol": false,
                "onlyReasoning": false,
                OWNER_KEY: provider.id,
            }));
        }

        let mut text = serde_json::to_string_pretty(&serde_json::Value::Array(entries))?;
        text.push('\n');
        write_with_backup(&path, text.as_bytes())
    }

    fn unapply(&self, dir: &Path, provider_id: &str) -> anyhow::Result<()> {
        let path = self.config_path(dir);
        let Some(text) = read_optional(&path)? else {
            return Ok(());
        };
        let mut entries = parse_array(&text)?;
        let before = entries.len();
        entries.retain(|entry| entry.get(OWNER_KEY).and_then(|v| v.as_str()) != Some(provider_id));
        if entries.len() == before {
            return Ok(());
        }
        let mut text = serde_json::to_string_pretty(&serde_json::Value::Array(entries))?;
        text.push('\n');
        write_with_backup(&path, text.as_bytes())
    }
}

fn parse_array(text: &str) -> anyhow::Result<Vec<serde_json::Value>> {
    let value: serde_json::Value = json5::from_str(text)
        .map_err(|_| anyhow::anyhow!("WorkBuddy 配置文件无法解析，未做修改"))?;
    match value {
        serde_json::Value::Array(entries) => Ok(entries),
        _ => bail!("WorkBuddy 配置文件无法解析，未做修改"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{agent_providers::ApiFormat, agent_providers::tests::temp_dir};

    fn provider(id: &str, models: &[&str], base_url: &str) -> AgentProvider {
        AgentProvider {
            id: id.to_string(),
            app_id: "workbuddy".to_string(),
            name: "测试供应商".to_string(),
            base_url: base_url.to_string(),
            api_key: None,
            has_api_key: true,
            api_format: ApiFormat::OpenaiChat,
            models: models.iter().map(|m| m.to_string()).collect(),
            default_model: String::new(),
            notes: String::new(),
            sort_index: 0,
        }
    }

    fn read_entries(path: &Path) -> Vec<serde_json::Value> {
        serde_json::from_str(&std::fs::read_to_string(path).expect("读取 models.json"))
            .expect("解析 models.json")
    }

    #[test]
    fn apply_keeps_user_entries_and_does_not_duplicate() {
        let dir = temp_dir("workbuddy-apply");
        let path = dir.join(CONFIG_FILE);
        std::fs::write(
            &path,
            r#"[{"id":"user-model","name":"user-model","vendor":"Custom"}]"#,
        )
        .expect("写入用户文件");

        let writer = WorkBuddyWriter;
        writer
            .apply(
                &dir,
                &provider("ap-1", &["a", "b"], "https://api.example.com/v1/"),
                "sk-test-placeholder",
            )
            .expect("第一次 apply");

        let entries = read_entries(&path);
        assert_eq!(entries.len(), 3, "用户元素 + 两个新模型");
        assert_eq!(entries[0]["id"], "user-model", "用户元素必须保留");
        assert_eq!(
            entries[1]["url"],
            "https://api.example.com/v1/chat/completions"
        );
        assert_eq!(entries[1]["vendor"], "Custom");
        assert_eq!(entries[1]["supportsToolCall"], true);
        assert_eq!(entries[1]["supportsReasoning"], false);
        assert_eq!(entries[1][OWNER_KEY], "ap-1");
        assert_eq!(entries[1]["apiKey"], "sk-test-placeholder");

        // 再 apply 一次不得产生重复。
        writer
            .apply(
                &dir,
                &provider("ap-1", &["a", "b"], "https://api.example.com/v1"),
                "sk-test-placeholder",
            )
            .expect("第二次 apply");
        let entries = read_entries(&path);
        assert_eq!(entries.len(), 3, "重复 apply 不得复制元素");
        assert_eq!(
            entries[1]["url"],
            "https://api.example.com/v1/chat/completions"
        );

        // 已经带 /chat/completions 的地址不得再拼一次。
        writer
            .apply(
                &dir,
                &provider(
                    "ap-1",
                    &["a"],
                    "https://api.example.com/v1/chat/completions",
                ),
                "sk-test-placeholder",
            )
            .expect("带后缀的 apply");
        let entries = read_entries(&path);
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[1]["url"],
            "https://api.example.com/v1/chat/completions"
        );

        writer.unapply(&dir, "ap-1").expect("unapply");
        let entries = read_entries(&path);
        assert_eq!(entries.len(), 1, "只删除 CCP 自己的元素");
        assert_eq!(entries[0]["id"], "user-model");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_on_missing_file_starts_from_an_empty_array() {
        let dir = temp_dir("workbuddy-missing");
        WorkBuddyWriter
            .apply(
                &dir,
                &provider("ap-1", &["a"], "https://api.example.com"),
                "sk-test-placeholder",
            )
            .expect("在缺失文件上 apply");
        let entries = read_entries(&dir.join(CONFIG_FILE));
        assert_eq!(entries.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unparsable_file_is_left_untouched() {
        let dir = temp_dir("workbuddy-broken");
        let path = dir.join(CONFIG_FILE);
        let original = "{ this is not json";
        std::fs::write(&path, original).expect("写入损坏文件");

        let error = WorkBuddyWriter
            .apply(
                &dir,
                &provider("ap-1", &["a"], "https://api.example.com"),
                "sk-test-placeholder",
            )
            .expect_err("损坏文件必须报错");
        assert!(
            error
                .to_string()
                .contains("WorkBuddy 配置文件无法解析，未做修改")
        );
        assert_eq!(std::fs::read_to_string(&path).expect("读取文件"), original);

        // 根不是数组同样拒绝。
        std::fs::write(&path, r#"{"models":[]}"#).expect("写入对象根");
        assert!(
            WorkBuddyWriter
                .apply(
                    &dir,
                    &provider("ap-1", &["a"], "https://api.example.com"),
                    "k"
                )
                .is_err()
        );
        assert_eq!(
            std::fs::read_to_string(&path).expect("读取文件"),
            r#"{"models":[]}"#
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
