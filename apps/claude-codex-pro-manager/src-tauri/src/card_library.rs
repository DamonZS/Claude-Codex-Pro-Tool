//! Knowledge card library: model-judged cards (experience / pitfall / guidance /
//! architecture) with tags, optional editing, filtering and export.
//!
//! Storage is a single JSON file under the CCP app state directory so the whole
//! library can be inspected, exported and backed up as one unit.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::{
    CardLibraryPayload, CardPayload, CardQueryRequest, CardSaveRequest, CommandResult,
    DistillationFileInput,
};

pub const CARD_LIBRARY_FILE_NAME: &str = "card-library-v1.json";
pub const MAX_CARDS: usize = 2_000;
pub const MAX_CARD_TAGS: usize = 8;
pub const MAX_TAG_CHARS: usize = 32;
pub const MAX_CARD_TITLE_CHARS: usize = 120;
pub const MAX_CARD_BODY_CHARS: usize = 8_000;
pub const DEFAULT_PAGE_SIZE: usize = 50;
pub const MAX_PAGE_SIZE: usize = 200;
pub const MAX_DELETE_CARDS: usize = 200;

/// Card kinds the model may pick. Anything else degrades to `experience`.
pub const CARD_KINDS: [&str; 4] = ["experience", "pitfall", "guidance", "architecture"];
pub const DEFAULT_CARD_KIND: &str = "experience";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeCard {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn normalize_card_kind(kind: &str) -> &'static str {
    let trimmed = kind.trim().to_ascii_lowercase();
    for candidate in CARD_KINDS {
        if candidate == trimmed {
            return candidate;
        }
    }
    DEFAULT_CARD_KIND
}

/// Trim, dedupe (case-insensitively) and cap tags so a noisy model answer can
/// never grow the store without bound.
pub fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for tag in tags {
        let tag = tag.trim().trim_start_matches('#').trim();
        if tag.is_empty() {
            continue;
        }
        let tag = tag.chars().take(MAX_TAG_CHARS).collect::<String>();
        let key = tag.to_ascii_lowercase();
        if !seen.insert(key) {
            continue;
        }
        normalized.push(tag);
        if normalized.len() >= MAX_CARD_TAGS {
            break;
        }
    }
    normalized
}

fn card_title(body: &str) -> String {
    let first_line = body
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    let title = first_line.trim_start_matches('#').trim();
    let title = title.chars().take(MAX_CARD_TITLE_CHARS).collect::<String>();
    if title.is_empty() {
        "未命名卡片".into()
    } else {
        title
    }
}

pub fn card_store_path() -> PathBuf {
    claude_codex_pro_core::paths::default_app_state_dir()
        .join("aitracker")
        .join(CARD_LIBRARY_FILE_NAME)
}

/// Read the whole library; a missing or unreadable file yields an empty store
/// instead of an error so the UI can always render.
pub fn load_cards() -> Vec<KnowledgeCard> {
    load_cards_from(&card_store_path())
}

pub fn load_cards_from(path: &Path) -> Vec<KnowledgeCard> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<KnowledgeCard>>(&raw).unwrap_or_default()
}

pub fn write_cards_to(path: &Path, cards: &[KnowledgeCard]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(cards)?;
    fs::write(path, raw)?;
    Ok(())
}

fn write_cards(cards: &[KnowledgeCard]) -> anyhow::Result<()> {
    write_cards_to(&card_store_path(), cards)
}

/// What the model said about one card. Invalid kinds degrade to `experience`;
/// invalid tags are dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardAnnotation {
    pub kind: String,
    pub tags: Vec<String>,
    pub title: String,
}

/// Parse the model's annotation payload. Never fails: a diagnostic string is
/// returned alongside the degraded annotation so the caller can surface why.
pub fn parse_card_annotation(raw: &str) -> (CardAnnotation, Option<String>) {
    let mut diagnostic = None;
    let value = serde_json::from_str::<serde_json::Value>(raw.trim()).ok();
    let object = value.as_ref().and_then(|value| value.as_object());
    let mut kind = String::new();
    let mut tags = Vec::new();
    let mut title = String::new();
    if let Some(object) = object {
        if let Some(value) = object.get("kind").and_then(|value| value.as_str()) {
            kind = value.to_string();
        }
        if let Some(value) = object.get("title").and_then(|value| value.as_str()) {
            title = value.trim().to_string();
        }
        match object.get("tags") {
            Some(serde_json::Value::Array(items)) => {
                tags = items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect();
            }
            Some(_) => diagnostic = Some("模型返回的 tags 不是数组，已忽略。".into()),
            None => {}
        }
    } else if !raw.trim().is_empty() {
        diagnostic = Some("模型未返回可解析的卡片标注，已按经验卡处理。".into());
    }
    let normalized_kind = normalize_card_kind(&kind);
    if !kind.trim().is_empty() && normalized_kind != kind.trim().to_ascii_lowercase() {
        diagnostic = Some(format!(
            "模型返回的类型「{}」不受支持，已按 {normalized_kind} 处理。",
            kind.trim()
        ));
    }
    (
        CardAnnotation {
            kind: normalized_kind.to_string(),
            tags: normalize_tags(&tags),
            title: title.chars().take(MAX_CARD_TITLE_CHARS).collect(),
        },
        diagnostic,
    )
}

/// Build a card, letting the model annotation win over the caller's defaults
/// while keeping the store consistent.
pub fn build_card(request: &CardSaveRequest, annotation: Option<&CardAnnotation>) -> KnowledgeCard {
    let timestamp = now_ms().to_string();
    let body = request
        .body
        .trim()
        .chars()
        .take(MAX_CARD_BODY_CHARS)
        .collect::<String>();
    let mut tags = request.tags.clone().unwrap_or_default();
    if let Some(annotation) = annotation {
        tags.extend(annotation.tags.iter().cloned());
    }
    let model_title = annotation
        .map(|annotation| annotation.title.clone())
        .unwrap_or_default();
    let explicit_title = request
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(|title| title.chars().take(MAX_CARD_TITLE_CHARS).collect::<String>());
    let title = explicit_title.unwrap_or_else(|| {
        if model_title.is_empty() {
            card_title(&body)
        } else {
            model_title
        }
    });
    let kind = annotation
        .map(|annotation| annotation.kind.clone())
        .unwrap_or_else(|| normalize_card_kind(&request.kind).to_string());
    KnowledgeCard {
        id: timestamp.clone(),
        kind,
        title,
        body,
        tags: normalize_tags(&tags),
        source: request.source.trim().to_string(),
        created_at: timestamp.clone(),
        updated_at: timestamp,
    }
}

fn counts(cards: &[KnowledgeCard]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for card in cards {
        *counts.entry(card.kind.clone()).or_insert(0) += 1;
    }
    counts
}

pub fn matches_filter(card: &KnowledgeCard, request: &CardQueryRequest) -> bool {
    let kind = request.kind.trim();
    if !kind.is_empty() && card.kind != normalize_card_kind(kind) {
        return false;
    }
    let tag = request.tag.trim();
    if !tag.is_empty()
        && !card
            .tags
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(tag))
    {
        return false;
    }
    let keyword = request.keyword.trim().to_ascii_lowercase();
    if !keyword.is_empty() {
        let haystack = format!(
            "{}\n{}\n{}\n{}",
            card.title,
            card.body,
            card.tags.join(" "),
            card.kind
        )
        .to_ascii_lowercase();
        if !haystack.contains(&keyword) {
            return false;
        }
    }
    true
}

/// Case-insensitive keyword/tag/kind filter with paging over the full store.
pub fn query_cards(request: &CardQueryRequest) -> CommandResult<CardLibraryPayload> {
    let all = load_cards();
    let mut filtered = all
        .iter()
        .filter(|card| matches_filter(card, request))
        .cloned()
        .collect::<Vec<_>>();
    filtered.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    let offset = request.offset.unwrap_or(0);
    let limit = request
        .limit
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let page = filtered
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    super::ok(
        &format!("已加载 {} 张卡片。", page.len()),
        CardLibraryPayload {
            total: all.len(),
            kinds: counts(&all),
            cards: page,
        },
    )
}

pub fn upsert_card(request: CardSaveRequest) -> CommandResult<CardPayload> {
    let body = request.body.trim();
    if body.is_empty() {
        return super::failed(
            "卡片内容为空，无法保存（errors.cardLibrary.emptyBody）。",
            CardPayload {
                cards: load_cards(),
                card: None,
            },
        );
    }
    let mut cards = load_cards();
    let id = request.id.trim().to_string();
    let index = cards.iter().position(|card| card.id == id);
    if index.is_none() && cards.len() >= MAX_CARDS {
        return super::failed(
            &format!("卡片数量已达上限（{MAX_CARDS} 张）。"),
            CardPayload { cards, card: None },
        );
    }
    let (annotation, annotation_diagnostic) = match request.annotation.as_deref() {
        Some(raw) => {
            let (annotation, diagnostic) = parse_card_annotation(raw);
            (Some(annotation), diagnostic)
        }
        None => (None, None),
    };
    let mut card = build_card(&request, annotation.as_ref());
    match index {
        Some(index) => {
            card.id = cards[index].id.clone();
            card.created_at = cards[index].created_at.clone();
            cards[index] = card.clone();
        }
        None => cards.push(card.clone()),
    }
    match write_cards(&cards) {
        Ok(()) => {
            let message = match annotation_diagnostic {
                Some(diagnostic) => format!("卡片已保存（{diagnostic}）"),
                None => "卡片已保存。".to_string(),
            };
            super::ok(
                &message,
                CardPayload {
                    cards,
                    card: Some(card),
                },
            )
        }
        Err(error) => super::failed(
            &format!("卡片保存失败：{error}"),
            CardPayload {
                cards: load_cards(),
                card: None,
            },
        ),
    }
}

pub fn delete_cards(ids: Vec<String>) -> CommandResult<CardPayload> {
    let ids = ids
        .into_iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect::<BTreeSet<_>>();
    if ids.is_empty() || ids.len() > MAX_DELETE_CARDS {
        return super::failed(
            &format!("一次可删除 1–{MAX_DELETE_CARDS} 张卡片。"),
            CardPayload {
                cards: load_cards(),
                card: None,
            },
        );
    }
    let mut cards = load_cards();
    cards.retain(|card| !ids.contains(&card.id));
    match write_cards(&cards) {
        Ok(()) => super::ok(
            &format!("已删除 {} 张卡片。", ids.len()),
            CardPayload { cards, card: None },
        ),
        Err(error) => super::failed(
            &format!("卡片删除失败：{error}"),
            CardPayload {
                cards: load_cards(),
                card: None,
            },
        ),
    }
}

pub fn export_card_markdown(card: &KnowledgeCard) -> String {
    let mut lines = vec![
        format!("# {}", card.title),
        String::new(),
        format!("- 类型：{}", card.kind),
    ];
    if !card.tags.is_empty() {
        lines.push(format!("- 标签：{}", card.tags.join("、")));
    }
    if !card.source.is_empty() {
        lines.push(format!("- 来源：{}", card.source));
    }
    lines.extend([String::new(), card.body.clone(), String::new()]);
    lines.join("\n")
}

/// Card package as `SKILL.md`, so a card can be installed as a Skill later.
pub fn card_skill_files(card: &KnowledgeCard) -> Vec<DistillationFileInput> {
    let description = format!(
        "{}（{}）",
        card.title,
        if card.tags.is_empty() {
            card.kind.clone()
        } else {
            card.tags.join("、")
        }
    );
    let content = format!(
        "---\nname: {}\ndescription: {}\n---\n\n{}\n",
        sanitize_skill_name(&card.title),
        description.replace('\n', " "),
        card.body.trim()
    );
    vec![
        DistillationFileInput {
            path: "SKILL.md".into(),
            content,
        },
        DistillationFileInput {
            path: "references/card.md".into(),
            content: export_card_markdown(card),
        },
    ]
}

/// Skill folder name derived from the card title.
pub fn card_skill_name(card: &KnowledgeCard) -> String {
    sanitize_skill_name(&card.title)
}

fn sanitize_skill_name(title: &str) -> String {
    let mut name = String::new();
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            name.push(ch.to_ascii_lowercase());
        } else if (ch == ' ' || ch == '-' || ch == '_') && !name.ends_with('-') {
            name.push('-');
        } else if !ch.is_ascii() {
            name.push(ch);
        }
    }
    let name = name.trim_matches('-').to_string();
    if name.is_empty() {
        "knowledge-card".into()
    } else {
        name.chars().take(48).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ccp-card-test-{name}-{}.json", now_ms()))
    }

    fn sample_card(body: &str, kind: &str, tags: &[&str]) -> KnowledgeCard {
        KnowledgeCard {
            id: "1".into(),
            kind: normalize_card_kind(kind).into(),
            title: card_title(body),
            body: body.into(),
            tags: normalize_tags(&tags.iter().map(|tag| tag.to_string()).collect::<Vec<_>>()),
            source: "session:demo".into(),
            created_at: "1".into(),
            updated_at: "1".into(),
        }
    }

    #[test]
    fn invalid_kind_degrades_to_experience_with_diagnostic() {
        let (annotation, diagnostic) = parse_card_annotation(r#"{"kind":"nonsense","tags":["a"]}"#);
        assert_eq!(annotation.kind, "experience");
        assert_eq!(annotation.tags, vec!["a".to_string()]);
        assert!(diagnostic.unwrap().contains("nonsense"));
    }

    #[test]
    fn invalid_annotation_degrades_without_panicking() {
        let (annotation, diagnostic) = parse_card_annotation("not json at all");
        assert_eq!(annotation.kind, "experience");
        assert!(annotation.tags.is_empty());
        assert!(diagnostic.is_some());
    }

    #[test]
    fn model_annotation_wins_over_caller_defaults() {
        let (annotation, diagnostic) = parse_card_annotation(
            r#"{"kind":"pitfall","tags":["  #CSS  ","css"],"title":"注入脚本读 CSS 变量"}"#,
        );
        assert_eq!(diagnostic, None);
        assert_eq!(annotation.kind, "pitfall");
        assert_eq!(annotation.tags, vec!["CSS".to_string()]);

        let request = CardSaveRequest {
            kind: "experience".into(),
            body: "把 CSS 变量同步到注入脚本时先读计算样式".into(),
            tags: Some(vec!["inject".into()]),
            annotation: Some(r#"{"kind":"pitfall"}"#.into()),
            ..Default::default()
        };
        let card = build_card(&request, Some(&annotation));
        assert_eq!(card.kind, "pitfall");
        assert_eq!(card.title, "注入脚本读 CSS 变量");
        assert_eq!(card.tags, vec!["inject".to_string(), "CSS".to_string()]);
    }

    #[test]
    fn tags_are_deduped_capped_and_trimmed() {
        let tags = (0..12)
            .map(|index| format!("  #Tag{index}  "))
            .collect::<Vec<_>>();
        let normalized = normalize_tags(&tags);
        assert_eq!(normalized.len(), MAX_CARD_TAGS);
        assert_eq!(normalized[0], "Tag0");
        let repeated = normalize_tags(&["same".into(), "SAME".into(), "same ".into()]);
        assert_eq!(repeated, vec!["same".to_string()]);
        let long = normalize_tags(&["x".repeat(200)]);
        assert_eq!(long[0].chars().count(), MAX_TAG_CHARS);
    }

    #[test]
    fn library_roundtrip_and_filters() {
        let path = temp_store("roundtrip");
        let cards = vec![
            sample_card("# 踩坑：代理截胡\n内容", "pitfall", &["网络"]),
            sample_card("# 架构：采集链路\n内容", "architecture", &["采集"]),
        ];
        write_cards_to(&path, &cards).unwrap();
        let loaded = load_cards_from(&path);
        assert_eq!(loaded, cards);
        let _ = fs::remove_file(&path);

        let pitfall = CardQueryRequest {
            kind: "pitfall".into(),
            tag: "网络".into(),
            keyword: "代理".into(),
            ..CardQueryRequest::default()
        };
        assert!(matches_filter(&cards[0], &pitfall));
        assert!(!matches_filter(&cards[1], &pitfall));
        let mismatched_tag = CardQueryRequest {
            tag: "采集".into(),
            ..CardQueryRequest::default()
        };
        assert!(!matches_filter(&cards[0], &mismatched_tag));
    }

    #[test]
    fn card_export_and_skill_files_are_usable() {
        let card = sample_card("# 读取需求文档\n先读 spec 再动手", "guidance", &["流程"]);
        let exported = export_card_markdown(&card);
        assert!(exported.contains("# 读取需求文档"));
        let files = card_skill_files(&card);
        let skill = files.iter().find(|file| file.path == "SKILL.md").unwrap();
        assert!(skill.content.contains("name: "));
        assert!(skill.content.contains("description: "));
        assert!(!skill.content.contains("TODO"));
        assert!(files.iter().any(|file| file.path == "references/card.md"));
    }
}
