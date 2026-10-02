//! MiniMax Code `config.yaml`: an additive `custom_provider.<id>` node.
//!
//! The node always carries `kind: custom`. A node with any other `kind` belongs
//! to the user's own MiniMax account and is never overwritten.

use std::path::{Path, PathBuf};

use super::{AgentProvider, AgentWriter, ApiFormat, read_optional, write_with_backup};

const CONFIG_FILE: &str = "config.yaml";
const KEY: &str = "custom_provider";

pub struct MiniMaxWriter;

impl MiniMaxWriter {
    fn read_root(path: &Path) -> anyhow::Result<serde_yaml::Mapping> {
        let Some(text) = read_optional(path)? else {
            return Ok(serde_yaml::Mapping::new());
        };
        let value: serde_yaml::Value = serde_yaml::from_str(&text)
            .map_err(|_| anyhow::anyhow!("MiniMax Code 配置文件无法解析，未做修改"))?;
        match value {
            serde_yaml::Value::Mapping(root) => Ok(root),
            serde_yaml::Value::Null => Ok(serde_yaml::Mapping::new()),
            _ => anyhow::bail!("MiniMax Code 配置文件无法解析，未做修改"),
        }
    }

    fn providers_mut(root: &mut serde_yaml::Mapping) -> anyhow::Result<&mut serde_yaml::Mapping> {
        let key = yaml(KEY);
        if !root.contains_key(&key) {
            root.insert(
                key.clone(),
                serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
            );
        }
        root.get_mut(&key)
            .and_then(|value| value.as_mapping_mut())
            .ok_or_else(|| {
                anyhow::anyhow!("MiniMax Code 配置文件的 custom_provider 不是映射，未做修改")
            })
    }
}

fn yaml(text: &str) -> serde_yaml::Value {
    serde_yaml::Value::String(text.to_string())
}

/// The id CCP is allowed to write: `ap-xxxx` and any other plain token.
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// The wire API name MiniMax Code expects.
fn api_name(format: ApiFormat) -> &'static str {
    match format {
        ApiFormat::Anthropic => "anthropic-messages",
        ApiFormat::OpenaiChat => "openai-completions",
        ApiFormat::OpenaiResponses => "openai-responses",
        ApiFormat::GeminiNative => "openai-completions",
    }
}

/// Whether `defaultModel` / `defaultLightModel` still points at `id`.
fn default_model_uses(root: &serde_yaml::Mapping, id: &str) -> bool {
    let prefix = format!("{KEY}:{id}/");
    ["defaultModel", "defaultLightModel"].iter().any(|key| {
        root.get(yaml(key))
            .and_then(|value| value.as_str())
            .is_some_and(|value| value.starts_with(&prefix))
    })
}

impl AgentWriter for MiniMaxWriter {
    fn config_path(&self, dir: &Path) -> PathBuf {
        dir.join(CONFIG_FILE)
    }

    fn apply(&self, dir: &Path, provider: &AgentProvider, api_key: &str) -> anyhow::Result<()> {
        if !valid_id(&provider.id) {
            anyhow::bail!("MiniMax Code 供应商 id 不合法，未做修改");
        }
        let path = self.config_path(dir);
        let mut root = Self::read_root(&path)?;

        // 已存在的节点如果不是 CCP 建的，就属于用户自己的 MiniMax 账号。
        let key = yaml(KEY);
        if let Some(existing) = root
            .get(&key)
            .and_then(|value| value.as_mapping())
            .and_then(|providers| providers.get(yaml(&provider.id)))
            .and_then(|value| value.as_mapping())
            && existing.get(yaml("kind")).and_then(|kind| kind.as_str()) != Some("custom")
        {
            anyhow::bail!("该供应商已由 MiniMax Code 自身管理，请勿覆盖，未做修改");
        }

        let mut models = serde_yaml::Mapping::new();
        for model in &provider.models {
            let mut entry = serde_yaml::Mapping::new();
            entry.insert(yaml("name"), yaml(model));
            models.insert(yaml(model), serde_yaml::Value::Mapping(entry));
        }

        let mut options = serde_yaml::Mapping::new();
        options.insert(yaml("baseURL"), yaml(provider.base_url.trim()));
        options.insert(yaml("apiKey"), yaml(api_key));

        let mut entry = serde_yaml::Mapping::new();
        entry.insert(yaml("name"), yaml(provider.name.trim()));
        entry.insert(yaml("kind"), yaml("custom"));
        entry.insert(yaml("enabled"), serde_yaml::Value::Bool(true));
        entry.insert(yaml("api"), yaml(api_name(provider.api_format)));
        entry.insert(yaml("options"), serde_yaml::Value::Mapping(options));
        entry.insert(yaml("models"), serde_yaml::Value::Mapping(models));

        let id = provider.id.clone();
        let providers = Self::providers_mut(&mut root)?;
        providers.insert(yaml(&id), serde_yaml::Value::Mapping(entry));

        let text = serde_yaml::to_string(&serde_yaml::Value::Mapping(root))?;
        write_with_backup(&path, text.as_bytes())
    }

    fn unapply(&self, dir: &Path, provider_id: &str) -> anyhow::Result<()> {
        let path = self.config_path(dir);
        let Some(_) = read_optional(&path)? else {
            return Ok(());
        };
        let mut root = Self::read_root(&path)?;

        if default_model_uses(&root, provider_id) {
            anyhow::bail!(
                "MiniMax Code 的默认模型仍在使用该供应商，请先在 MiniMax Code 中改用其他模型"
            );
        }

        let key = yaml(KEY);
        let mut changed = false;
        if let Some(providers) = root.get_mut(&key).and_then(|value| value.as_mapping_mut()) {
            changed = providers.remove(yaml(provider_id)).is_some();
        }
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
    use crate::agent_providers::tests::temp_dir;

    fn provider(id: &str, format: ApiFormat) -> AgentProvider {
        AgentProvider {
            id: id.to_string(),
            app_id: "mcode".to_string(),
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

    #[test]
    fn apply_writes_the_node_and_keeps_user_content() {
        let dir = temp_dir("mcode-apply");
        let path = dir.join(CONFIG_FILE);
        std::fs::write(
            &path,
            "theme: dark\ncustom_provider:\n  mine:\n    kind: minimax\n    name: 我的账号\n",
        )
        .expect("写入用户文件");

        let writer = MiniMaxWriter;
        writer
            .apply(
                &dir,
                &provider("ap-1234567890ab", ApiFormat::Anthropic),
                "sk-test-placeholder",
            )
            .expect("apply");

        let root = root_of(&path);
        assert_eq!(root["theme"].as_str(), Some("dark"), "用户字段必须保留");
        assert_eq!(
            root["custom_provider"]["mine"]["kind"].as_str(),
            Some("minimax"),
            "用户自己的节点必须保留"
        );
        let node = &root["custom_provider"]["ap-1234567890ab"];
        assert_eq!(node["name"].as_str(), Some("供应商"));
        assert_eq!(node["kind"].as_str(), Some("custom"));
        assert_eq!(node["enabled"].as_bool(), Some(true));
        assert_eq!(node["api"].as_str(), Some("anthropic-messages"));
        assert_eq!(
            node["options"]["baseURL"].as_str(),
            Some("https://api.example.com/v1")
        );
        assert_eq!(
            node["options"]["apiKey"].as_str(),
            Some("sk-test-placeholder")
        );
        assert_eq!(node["models"]["model-a"]["name"].as_str(), Some("model-a"));
        assert_eq!(node["models"]["model-b"]["name"].as_str(), Some("model-b"));

        // 再次 apply 覆盖同一个节点，不产生副本。
        writer
            .apply(
                &dir,
                &provider("ap-1234567890ab", ApiFormat::OpenaiResponses),
                "sk-test-placeholder",
            )
            .expect("再次 apply");
        let root = root_of(&path);
        assert_eq!(
            root["custom_provider"].as_mapping().expect("映射").len(),
            2,
            "重复 apply 不得新增节点"
        );
        assert_eq!(
            root["custom_provider"]["ap-1234567890ab"]["api"].as_str(),
            Some("openai-responses")
        );

        writer.unapply(&dir, "ap-1234567890ab").expect("unapply");
        let root = root_of(&path);
        assert!(root["custom_provider"].get("ap-1234567890ab").is_none());
        assert_eq!(
            root["custom_provider"]["mine"]["name"].as_str(),
            Some("我的账号")
        );
        assert_eq!(root["theme"].as_str(), Some("dark"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_non_custom_node_is_rejected() {
        let dir = temp_dir("mcode-not-custom");
        let path = dir.join(CONFIG_FILE);
        let original = "custom_provider:\n  ap-1234567890ab:\n    kind: minimax\n    name: 账号\n";
        std::fs::write(&path, original).expect("写入用户文件");

        let error = MiniMaxWriter
            .apply(
                &dir,
                &provider("ap-1234567890ab", ApiFormat::OpenaiChat),
                "sk-test-placeholder",
            )
            .expect_err("非 custom 节点必须被拒绝");
        assert!(
            error.to_string().contains("MiniMax Code"),
            "错误信息应说明应用"
        );
        assert_eq!(std::fs::read_to_string(&path).expect("读取文件"), original);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unapply_is_rejected_while_the_default_model_uses_the_provider() {
        let dir = temp_dir("mcode-default-model");
        let path = dir.join(CONFIG_FILE);
        std::fs::write(
            &path,
            "defaultModel: \"custom_provider:ap-1234567890ab/model-a\"\n",
        )
        .expect("写入用户文件");

        let writer = MiniMaxWriter;
        writer
            .apply(
                &dir,
                &provider("ap-1234567890ab", ApiFormat::OpenaiChat),
                "sk-test-placeholder",
            )
            .expect("apply");

        let error = writer
            .unapply(&dir, "ap-1234567890ab")
            .expect_err("默认模型仍指向时必须拒绝");
        assert_eq!(
            error.to_string(),
            "MiniMax Code 的默认模型仍在使用该供应商，请先在 MiniMax Code 中改用其他模型"
        );
        let root = root_of(&path);
        assert!(
            root["custom_provider"].get("ap-1234567890ab").is_some(),
            "拒绝时不得删除节点"
        );

        // 换掉默认模型后即可移除。
        let mut document = root.as_mapping().expect("映射").clone();
        document.insert(yaml("defaultModel"), yaml("custom_provider:other/model-a"));
        std::fs::write(
            &path,
            serde_yaml::to_string(&serde_yaml::Value::Mapping(document)).expect("序列化"),
        )
        .expect("写回文件");
        writer.unapply(&dir, "ap-1234567890ab").expect("unapply");
        assert!(
            root_of(&path)["custom_provider"]
                .get("ap-1234567890ab")
                .is_none()
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unparsable_file_is_left_untouched() {
        let dir = temp_dir("mcode-broken");
        let path = dir.join(CONFIG_FILE);
        let original = "custom_provider: { oops\n";
        std::fs::write(&path, original).expect("写入损坏文件");

        let error = MiniMaxWriter
            .apply(
                &dir,
                &provider("ap-1234567890ab", ApiFormat::OpenaiChat),
                "sk-test-placeholder",
            )
            .expect_err("损坏文件必须报错");
        assert!(
            error
                .to_string()
                .contains("MiniMax Code 配置文件无法解析，未做修改")
        );
        assert_eq!(std::fs::read_to_string(&path).expect("读取文件"), original);
        assert!(MiniMaxWriter.unapply(&dir, "ap-1234567890ab").is_err());
        assert_eq!(std::fs::read_to_string(&path).expect("读取文件"), original);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
