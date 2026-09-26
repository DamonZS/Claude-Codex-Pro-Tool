//! Deterministic, preview-only composition of prompt library entries.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MAX_SOURCES: usize = 8;
pub const MAX_SOURCE_BYTES: usize = 256 * 1024;
pub const MAX_COMPOSED_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptSource {
    pub id: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptComposition {
    pub content: String,
    pub source_ids: Vec<String>,
    pub source_titles: Vec<String>,
    pub warnings: Vec<String>,
    pub sha256: String,
    pub bytes: usize,
}

pub fn compose(sources: &[PromptSource]) -> Result<PromptComposition> {
    if sources.is_empty() {
        bail!("至少选择一个提示词来源。")
    }
    if sources.len() > MAX_SOURCES {
        bail!("最多组合 {MAX_SOURCES} 个提示词来源。")
    }

    let mut seen_ids = std::collections::HashSet::new();
    let mut seen_content = std::collections::HashSet::new();
    let mut warnings = Vec::new();
    let mut sections = Vec::new();
    let mut source_ids = Vec::new();
    let mut source_titles = Vec::new();

    for source in sources {
        let id = source.id.trim();
        if id.is_empty() || !seen_ids.insert(id.to_string()) {
            bail!("提示词来源 ID 为空或重复：{}", source.id.trim())
        }
        let content = source.content.replace("\r\n", "\n").replace('\r', "\n");
        let trimmed = content.trim();
        if trimmed.is_empty() {
            bail!("提示词来源为空：{}", source.title.trim())
        }
        if trimmed.len() > MAX_SOURCE_BYTES {
            bail!("提示词来源超过 256 KiB：{}", source.title.trim())
        }
        let content_hash = Sha256::digest(trimmed.as_bytes());
        let content_key = to_hex(&content_hash);
        if !seen_content.insert(content_key) {
            warnings.push(format!("已去重重复正文：{}", source.title.trim()));
            continue;
        }
        let title = sanitize_title(&source.title);
        source_ids.push(id.to_string());
        source_titles.push(title.clone());
        sections.push(format!("## 来源：{title}\n\n{trimmed}\n"));
    }

    if sections.is_empty() {
        bail!("所有提示词来源均为重复正文，无法生成组合。")
    }
    let content = format!(
        "# CCP 组合提示词\n\n> 来源按选择顺序排列；此预览不会执行来源中的脚本或工具。\n\n{}",
        sections.join("\n")
    );
    if content.len() > MAX_COMPOSED_BYTES {
        bail!("组合提示词超过 1 MiB 上限。")
    }
    let sha256 = to_hex(&Sha256::digest(content.as_bytes()));
    Ok(PromptComposition {
        bytes: content.len(),
        content,
        source_ids,
        source_titles,
        warnings,
        sha256,
    })
}

fn sanitize_title(title: &str) -> String {
    let title = title
        .chars()
        .filter(|ch| *ch != '\r' && *ch != '\n')
        .collect::<String>();
    let title = title.trim();
    if title.is_empty() {
        "未命名来源".to_string()
    } else {
        title.chars().take(160).collect()
    }
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(id: &str, title: &str, content: &str) -> PromptSource {
        PromptSource {
            id: id.into(),
            title: title.into(),
            content: content.into(),
        }
    }

    #[test]
    fn preserves_order_and_is_stable() {
        let sources = [source("a", "A", "alpha"), source("b", "B", "beta")];
        let first = compose(&sources).unwrap();
        let second = compose(&sources).unwrap();
        assert!(first.content.find("alpha").unwrap() < first.content.find("beta").unwrap());
        assert_eq!(first.sha256, second.sha256);
        assert_eq!(first.content, second.content);
    }

    #[test]
    fn deduplicates_identical_content_and_warns() {
        let result = compose(&[source("a", "A", "same"), source("b", "B", "same")]).unwrap();
        assert_eq!(result.source_ids, vec!["a"]);
        assert_eq!(result.warnings.len(), 1);
        assert!(!result.content.contains("来源：B"));
    }

    #[test]
    fn rejects_empty_duplicate_and_oversized_input() {
        assert!(compose(&[]).is_err());
        assert!(compose(&[source("a", "A", " "), source("a", "A", "x")]).is_err());
        assert!(compose(&[source("a", "A", &"x".repeat(MAX_SOURCE_BYTES + 1))]).is_err());
    }

    #[test]
    fn sanitizes_title_line_breaks() {
        let result = compose(&[source("a", "A\nB", "body")]).unwrap();
        assert!(result.content.contains("## 来源：AB"));
    }
}
