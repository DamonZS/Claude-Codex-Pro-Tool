//! Hermes `config.yaml`: an additive list of `custom_providers` entries.
//!
//! Every key the user wrote stays in place; CCP only appends and removes the
//! entries whose `name` is the supplier id it owns.

use std::path::{Path, PathBuf};

use super::{AgentProvider, AgentWriter, read_optional, write_with_backup};

const CONFIG_FILE: &str = "config.yaml";

pub struct HermesWriter;

impl HermesWriter {
    /// Read `config.yaml` into a mapping, or an empty one when it is missing.
    fn read_root(path: &Path) -> anyhow::Result<serde_yaml::Mapping> {
        let Some(text) = read_optional(path)? else {
            return Ok(serde_yaml::Mapping::new());
        };
        let value: serde_yaml::Value = serde_yaml::from_str(&text)
            .map_err(|_| anyhow::anyhow!("Hermes 配置文件无法解析，未做修改"))?;
        match value {
            serde_yaml::Value::Mapping(root) => Ok(root),
            serde_yaml::Value::Null => Ok(serde_yaml::Mapping::new()),
            _ => anyhow::bail!("Hermes 配置文件无法解析，未做修改"),
        }
    }

    /// `custom_providers` as a sequence, creating it when missing.
    fn providers_mut(
        root: &mut serde_yaml::Mapping,
    ) -> anyhow::Result<&mut Vec<serde_yaml::Value>> {
        let key = serde_yaml::Value::String("custom_providers".to_string());
        if !root.contains_key(&key) {
            root.insert(key.clone(), serde_yaml::Value::Sequence(Vec::new()));
        }
        root.get_mut(&key)
            .and_then(|value| value.as_sequence_mut())
            .ok_or_else(|| anyhow::anyhow!("Hermes 配置文件的 custom_providers 不是数组，未做修改"))
    }
}

fn yaml(text: &str) -> serde_yaml::Value {
    serde_yaml::Value::String(text.to_string())
}

impl AgentWriter for HermesWriter {
    fn config_path(&self, dir: &Path) -> PathBuf {
        dir.join(CONFIG_FILE)
    }

    fn apply(&self, dir: &Path, provider: &AgentProvider, api_key: &str) -> anyhow::Result<()> {
        let path = self.config_path(dir);
        let mut root = Self::read_root(&path)?;

        let mut models = serde_yaml::Mapping::new();
        for model in &provider.models {
            models.insert(
                yaml(model),
                serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
            );
        }

        let mut entry = serde_yaml::Mapping::new();
        entry.insert(yaml("name"), yaml(&provider.id));
        entry.insert(yaml("base_url"), yaml(provider.base_url.trim()));
        entry.insert(yaml("api_key"), yaml(api_key));
        entry.insert(
            yaml("api_mode"),
            yaml(match provider.api_format {
                super::ApiFormat::Anthropic => "anthropic_messages",
                _ => "chat_completions",
            }),
        );
        entry.insert(
            yaml("model"),
            yaml(provider.models.first().map(String::as_str).unwrap_or("")),
        );
        entry.insert(yaml("models"), serde_yaml::Value::Mapping(models));

        let id = provider.id.clone();
        let providers = Self::providers_mut(&mut root)?;
        providers.retain(|existing| {
            existing
                .as_mapping()
                .and_then(|entry| entry.get(yaml("name")))
                .and_then(|name| name.as_str())
                != Some(id.as_str())
        });
        providers.push(serde_yaml::Value::Mapping(entry));

        let text = serde_yaml::to_string(&serde_yaml::Value::Mapping(root))?;
        write_with_backup(&path, text.as_bytes())
    }

    fn unapply(&self, dir: &Path, provider_id: &str) -> anyhow::Result<()> {
        let path = self.config_path(dir);
        let Some(_) = read_optional(&path)? else {
            return Ok(());
        };
        let mut root = Self::read_root(&path)?;

        let key = yaml("custom_providers");
        let mut changed = false;
        if let Some(providers) = root.get_mut(&key).and_then(|value| value.as_sequence_mut()) {
            let before = providers.len();
            providers.retain(|existing| {
                existing
                    .as_mapping()
                    .and_then(|entry| entry.get(yaml("name")))
                    .and_then(|name| name.as_str())
                    != Some(provider_id)
            });
            changed = providers.len() != before;
        }
        // 用户自己的默认模型选择绝不改动，即使它正指向被移除的供应商。
        if !changed {
            return Ok(());
        }

        let text = serde_yaml::to_string(&serde_yaml::Value::Mapping(root))?;
        write_with_backup(&path, text.as_bytes())
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
            app_id: "hermes".to_string(),
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

    fn root_of(path: &Path) -> serde_yaml::Value {
        serde_yaml::from_str(&std::fs::read_to_string(path).expect("读取 config.yaml"))
            .expect("解析 config.yaml")
    }

    const USER_FILE: &str = "theme: dark\nmodel:\n  provider: user-choice\ncustom_providers:\n  - name: user-provider\n    base_url: https://user.example\n";

    #[test]
    fn apply_appends_keeps_user_content_and_is_idempotent() {
        let dir = temp_dir("hermes-apply");
        let path = dir.join(CONFIG_FILE);
        std::fs::write(&path, USER_FILE).expect("写入用户文件");

        let writer = HermesWriter;
        writer
            .apply(
                &dir,
                &provider("ap-1", ApiFormat::Anthropic),
                "sk-test-placeholder",
            )
            .expect("apply");

        let root = root_of(&path);
        assert_eq!(root["theme"].as_str(), Some("dark"), "用户字段必须保留");
        assert_eq!(root["model"]["provider"].as_str(), Some("user-choice"));
        let providers = root["custom_providers"].as_sequence().expect("数组");
        assert_eq!(providers.len(), 2, "用户条目加一个新条目");
        assert_eq!(providers[0]["name"].as_str(), Some("user-provider"));
        let ccp = &providers[1];
        assert_eq!(ccp["name"].as_str(), Some("ap-1"));
        assert_eq!(ccp["base_url"].as_str(), Some("https://api.example.com/v1"));
        assert_eq!(ccp["api_key"].as_str(), Some("sk-test-placeholder"));
        assert_eq!(ccp["api_mode"].as_str(), Some("anthropic_messages"));
        assert_eq!(ccp["model"].as_str(), Some("model-a"));
        assert!(ccp["models"]["model-a"].is_mapping());
        assert!(ccp["models"]["model-b"].is_mapping());

        // 再次 apply 不得追加重复条目。
        writer
            .apply(
                &dir,
                &provider("ap-1", ApiFormat::OpenaiChat),
                "sk-test-placeholder",
            )
            .expect("再次 apply");
        let root = root_of(&path);
        let providers = root["custom_providers"].as_sequence().expect("数组");
        assert_eq!(providers.len(), 2, "重复 apply 不得复制条目");
        assert_eq!(providers[1]["api_mode"].as_str(), Some("chat_completions"));

        // unapply 只删除自己，用户条目与默认模型选择保持原样。
        writer.unapply(&dir, "ap-1").expect("unapply");
        let root = root_of(&path);
        let providers = root["custom_providers"].as_sequence().expect("数组");
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0]["name"].as_str(), Some("user-provider"));
        assert_eq!(root["model"]["provider"].as_str(), Some("user-choice"));
        assert_eq!(root["theme"].as_str(), Some("dark"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn custom_providers_that_is_not_an_array_is_rejected() {
        let dir = temp_dir("hermes-not-array");
        let path = dir.join(CONFIG_FILE);
        let original = "theme: dark\ncustom_providers: oops\n";
        std::fs::write(&path, original).expect("写入用户文件");

        let error = HermesWriter
            .apply(
                &dir,
                &provider("ap-1", ApiFormat::OpenaiChat),
                "sk-test-placeholder",
            )
            .expect_err("非数组必须报错");
        assert!(error.to_string().contains("不是数组"), "错误信息应说明原因");
        assert_eq!(std::fs::read_to_string(&path).expect("读取文件"), original);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unparsable_file_is_left_untouched() {
        let dir = temp_dir("hermes-broken");
        let path = dir.join(CONFIG_FILE);
        let original = "custom_providers: [ {name: ap-1\n";
        std::fs::write(&path, original).expect("写入损坏文件");

        let error = HermesWriter
            .apply(
                &dir,
                &provider("ap-1", ApiFormat::OpenaiChat),
                "sk-test-placeholder",
            )
            .expect_err("损坏文件必须报错");
        assert!(
            error
                .to_string()
                .contains("Hermes 配置文件无法解析，未做修改")
        );
        assert_eq!(std::fs::read_to_string(&path).expect("读取文件"), original);
        assert!(HermesWriter.unapply(&dir, "ap-1").is_err());
        assert_eq!(std::fs::read_to_string(&path).expect("读取文件"), original);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_on_missing_file_creates_custom_providers() {
        let dir = temp_dir("hermes-missing");
        HermesWriter
            .apply(
                &dir,
                &provider("ap-1", ApiFormat::OpenaiChat),
                "sk-test-placeholder",
            )
            .expect("在缺失文件上 apply");
        let root = root_of(&dir.join(CONFIG_FILE));
        assert_eq!(
            root["custom_providers"].as_sequence().expect("数组").len(),
            1
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
