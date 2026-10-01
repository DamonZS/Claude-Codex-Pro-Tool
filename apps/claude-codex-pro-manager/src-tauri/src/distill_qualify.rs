//! Post-generation quality checker, ported from AITracker
//! `src/modules/distillation/qualify.ts` (Copyright (C) 2026 AITracker
//! contributors, used with permission).
//!
//! `error` checks are hard failures; `warn` checks are advisory only.

use regex::Regex;
use serde::Serialize;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityCheck {
    pub id: String,
    pub label: String,
    /// `"error"` or `"warn"`.
    pub level: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Qualification {
    pub pass: bool,
    pub checks: Vec<QualityCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistilledFile {
    pub path: String,
    pub content: String,
}

/// Execution/operation description keywords (hits >= 2 -> scripts/ required).
const EXECUTION_KEYWORDS: &[&str] = &[
    "自动化",
    "脚本",
    "执行",
    "运行",
    "部署",
    "扫描",
    "生成",
    "创建",
    "api",
    "浏览器",
    "上传",
    "下载",
    "转换",
    "构建",
    "安装",
    "提取",
    "编译",
    "获取",
    "发送",
    "automat",
    "script",
    "execute",
    "run",
    "deploy",
    "scan",
    "generate",
    "create",
    "upload",
    "download",
    "install",
    "fetch",
    "post",
    "send",
    "convert",
    "extract",
    "build",
    "compile",
];

const FORBIDDEN_GENERIC: &[&str] = &[
    "所有",
    "任何",
    "任意",
    "everything",
    "anything",
    "all kinds",
    "whatever",
];

const UNFINISHED_MARKERS: &[&str] = &["TODO", "FIXME", "HACK", "XXX"];

const MAX_SKILL_BYTES: usize = 8 * 1024;

fn frontmatter_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?s)\A---\s*\n(.*?)\n---").expect("frontmatter regex"))
}

fn file_tag_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?is)<file\s+path=["']([^"']+)["']\s*>(.*?)</file>"#).expect("file regex")
    })
}

/// JS `String.prototype.length` (UTF-16 code units).
fn js_len(value: &str) -> usize {
    value.encode_utf16().count()
}

fn hanzi_count(text: &str) -> usize {
    text.chars()
        .filter(|ch| ('\u{4e00}'..='\u{9fff}').contains(ch))
        .count()
}

fn english_letter_count(text: &str) -> usize {
    text.chars().filter(char::is_ascii_alphabetic).count()
}

pub fn count_execution_keywords(description: &str) -> usize {
    let lower = description.to_lowercase();
    EXECUTION_KEYWORDS
        .iter()
        .filter(|keyword| lower.contains(&keyword.to_lowercase()))
        .count()
}

fn parse_frontmatter(skill_md: &str) -> Vec<(String, String)> {
    let Some(captures) = frontmatter_regex().captures(skill_md) else {
        return Vec::new();
    };
    let mut result: Vec<(String, String)> = Vec::new();
    for line in captures[1].split('\n') {
        let Some(sep) = line.find(':') else {
            continue;
        };
        if sep == 0 {
            continue;
        }
        let key = line[..sep].trim().to_lowercase();
        let value = line[sep + 1..].trim().to_string();
        if let Some(existing) = result.iter_mut().find(|(name, _)| *name == key) {
            existing.1 = value;
        } else {
            result.push((key, value));
        }
    }
    result
}

/// Port of `buildFilesForQualification`.
pub fn build_files_for_qualification(
    summary: &str,
    kind: &str,
    description_hint: &str,
) -> Vec<DistilledFile> {
    if kind == "skill" {
        let parsed = parse_file_tags(summary);
        if parsed
            .iter()
            .any(|file| file.path.to_lowercase() == "skill.md")
        {
            return parsed;
        }
        return vec![DistilledFile {
            path: "SKILL.md".into(),
            content: summary.to_string(),
        }];
    }
    let doc_path = if kind == "brief" {
        "WORKFLOW.md"
    } else {
        "PROMPT.md"
    };
    vec![
        DistilledFile {
            path: "SKILL.md".into(),
            content: format!(
                "---\nname: Distilled\ndescription: {description_hint}\n---\n# Distilled\n"
            ),
        },
        DistilledFile {
            path: doc_path.into(),
            content: summary.to_string(),
        },
    ]
}

/// Extract `<file path="...">...</file>` blocks (trimmed, non-empty only).
pub fn parse_file_tags(summary: &str) -> Vec<DistilledFile> {
    file_tag_regex()
        .captures_iter(summary)
        .map(|captures| DistilledFile {
            path: captures[1].trim().to_string(),
            content: captures[2].trim().to_string(),
        })
        .filter(|file| !file.path.is_empty() && !file.content.is_empty())
        .collect()
}

/// Port of `qualifySkillFiles`. `kind` other than skill/prompt/brief is
/// treated as skill, matching AITracker's save path.
pub fn qualify_skill_files(files: &[DistilledFile], kind: &str) -> Qualification {
    let kind = if matches!(kind, "skill" | "prompt" | "brief") {
        kind
    } else {
        "skill"
    };
    let total_bytes: usize = files.iter().map(|file| file.content.len()).sum();
    if files.len() > 200 || total_bytes > 10 * 1024 * 1024 {
        return Qualification {
            pass: false,
            checks: vec![QualityCheck {
                id: "size-cap".into(),
                label: "产物规模在合理范围内".into(),
                level: "error".into(),
                ok: false,
                detail: Some(format!(
                    "{} 个文件 / {}MB",
                    files.len(),
                    (total_bytes as f64 / 1024.0 / 1024.0).round() as u64
                )),
            }],
        };
    }

    let skill_file = files
        .iter()
        .find(|file| file.path.to_lowercase() == "skill.md");
    let script_files = files
        .iter()
        .filter(|file| file.path.to_lowercase().starts_with("scripts/"))
        .collect::<Vec<_>>();
    let mut checks = Vec::new();
    let mut push = |id: &str, label: &str, ok: bool, level: &str, detail: Option<String>| {
        checks.push(QualityCheck {
            id: id.into(),
            label: label.into(),
            level: level.into(),
            ok,
            detail,
        });
    };

    let Some(skill_file) = skill_file else {
        push(
            "frontmatter",
            "包含 SKILL.md 与 YAML frontmatter",
            false,
            "error",
            Some("缺少 SKILL.md 文件".into()),
        );
        return Qualification {
            pass: false,
            checks,
        };
    };

    let frontmatter = parse_frontmatter(&skill_file.content);
    let get = |key: &str| {
        frontmatter
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    };
    let has_frontmatter = !frontmatter.is_empty()
        && get("name").is_some_and(|value| !value.is_empty())
        && get("description").is_some_and(|value| !value.is_empty());
    push(
        "frontmatter",
        "SKILL.md 含 name / description frontmatter",
        has_frontmatter,
        "error",
        (!has_frontmatter).then(|| "frontmatter 缺少 name 或 description".into()),
    );

    let description = get("description").unwrap_or("").to_string();
    let desc_len = js_len(description.trim());
    let mut needs_scripts = false;
    if kind == "skill" {
        push(
            "desc-length",
            "description 长度 50–300 字符（建议）",
            (50..=300).contains(&desc_len),
            "warn",
            Some(format!("{desc_len} 字符")),
        );
    } else {
        push(
            "desc-present",
            "description 已提供",
            desc_len > 0,
            "error",
            (desc_len == 0).then(|| "description 为空".into()),
        );
    }

    if kind == "skill" {
        let hanzi = hanzi_count(&description);
        let letters = english_letter_count(&description);
        push(
            "desc-keywords",
            "description 含中英文关键词（建议）",
            hanzi >= 2 && letters >= 3,
            "warn",
            Some(format!("中文 {hanzi} 字 / 英文 {letters} 字母")),
        );

        let lower = description.to_lowercase();
        let generic = FORBIDDEN_GENERIC
            .iter()
            .filter(|word| lower.contains(*word))
            .copied()
            .collect::<Vec<_>>();
        push(
            "desc-generic",
            "description 避免泛化词（建议）",
            generic.is_empty(),
            "warn",
            (!generic.is_empty()).then(|| format!("命中泛化词：{}", generic.join("、"))),
        );

        let execution_hits = count_execution_keywords(&description);
        needs_scripts = execution_hits >= 2;
        let script_ok = if needs_scripts {
            script_files
                .iter()
                .any(|file| js_len(file.content.trim()) > 10)
        } else {
            script_files.is_empty()
        };
        push(
            "scripts-decision",
            if needs_scripts {
                "执行类任务提供非空 scripts/（建议）"
            } else {
                "知识/指导类不含 scripts/（建议）"
            },
            script_ok,
            "warn",
            (!script_ok).then(|| {
                if needs_scripts {
                    format!(
                        "description 命中 {execution_hits} 个执行关键词，但 scripts/ 为空或缺失"
                    )
                } else {
                    format!("description 命中 {execution_hits} 个执行关键词，不应创建 scripts/")
                }
            }),
        );
    }

    let size = skill_file.content.len();
    push(
        "size",
        "SKILL.md 体积 ≤ 8KB",
        size <= MAX_SKILL_BYTES,
        "error",
        Some(format!("{}KB", (size as f64 / 1024.0).round() as u64)),
    );

    let skill_unfinished = UNFINISHED_MARKERS
        .iter()
        .filter(|marker| skill_file.content.contains(*marker))
        .take(3)
        .copied()
        .collect::<Vec<_>>();
    push(
        "no-todo",
        "SKILL.md 无 TODO/FIXME/HACK/XXX",
        skill_unfinished.is_empty(),
        "error",
        (!skill_unfinished.is_empty()).then(|| skill_unfinished.join("、")),
    );
    let other_unfinished = files
        .iter()
        .filter(|file| file.path.to_lowercase() != "skill.md")
        .flat_map(|file| {
            UNFINISHED_MARKERS
                .iter()
                .filter(|marker| file.content.contains(*marker))
                .map(|marker| format!("{marker}@{}", file.path))
                .collect::<Vec<_>>()
        })
        .take(3)
        .collect::<Vec<_>>();
    push(
        "no-todo-other",
        "其他文件无未完成标记（建议）",
        other_unfinished.is_empty(),
        "warn",
        (!other_unfinished.is_empty()).then(|| other_unfinished.join("、")),
    );

    if kind == "skill" && needs_scripts {
        let lower_path = |file: &&&DistilledFile| file.path.to_lowercase();
        let python_ok = script_files
            .iter()
            .filter(|file| lower_path(file).ends_with(".py"))
            .all(|file| python_try_regex().is_match(&file.content));
        let shell_ok = script_files
            .iter()
            .filter(|file| {
                let path = lower_path(file);
                path.ends_with(".sh") || path.ends_with(".bash") || path.ends_with(".zsh")
            })
            .all(|file| shell_strict_regex().is_match(&file.content));
        push(
            "scripts-robust",
            "脚本容错（Python try/except、Shell set -euo pipefail，建议）",
            python_ok && shell_ok,
            "warn",
            if !python_ok {
                Some("存在缺少 try/except 的 Python 脚本".into())
            } else if !shell_ok {
                Some("存在缺少 set -euo pipefail 的 Shell 脚本".into())
            } else {
                None
            },
        );
    }

    let empty_files = files
        .iter()
        .filter(|file| file.content.trim().is_empty())
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    push(
        "files-nonempty",
        "产物文件均非空",
        empty_files.is_empty(),
        "error",
        (!empty_files.is_empty()).then(|| format!("空文件：{}", empty_files.join("、"))),
    );

    let pass = checks.iter().all(|check| check.ok || check.level == "warn");
    Qualification { pass, checks }
}

fn python_try_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"try\s*:").expect("python regex"))
}

fn shell_strict_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"set\s+-euo\s+pipefail").expect("shell regex"))
}

/// Failure list in AITracker's feedback format: `label（detail）` joined by `；`.
pub fn failure_summary(qualification: &Qualification) -> String {
    qualification
        .checks
        .iter()
        .filter(|check| !check.ok)
        .map(|check| match &check.detail {
            Some(detail) if !detail.is_empty() => format!("{}（{detail}）", check.label),
            _ => check.label.clone(),
        })
        .collect::<Vec<_>>()
        .join("；")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, content: &str) -> DistilledFile {
        DistilledFile {
            path: path.into(),
            content: content.into(),
        }
    }

    const GOOD_DESC: &str = "为团队提供中文周报写作指导与结构建议，覆盖 weekly report outline 与语气调整，帮助用户快速产出清晰周报内容摘要";

    #[test]
    fn distill_qualify_missing_skill_md_fails() {
        let result = qualify_skill_files(&[file("README.md", "hello")], "skill");
        assert!(!result.pass);
        assert_eq!(result.checks[0].id, "frontmatter");
        assert_eq!(
            result.checks[0].detail.as_deref(),
            Some("缺少 SKILL.md 文件")
        );
    }

    #[test]
    fn distill_qualify_todo_in_skill_md_fails_but_other_files_only_warn() {
        let skill =
            format!("---\nname: Weekly\ndescription: {GOOD_DESC}\n---\n# Weekly\nTODO later");
        let result = qualify_skill_files(&[file("SKILL.md", &skill)], "skill");
        assert!(!result.pass);
        assert!(
            result
                .checks
                .iter()
                .any(|check| check.id == "no-todo" && !check.ok)
        );

        let clean = format!("---\nname: Weekly\ndescription: {GOOD_DESC}\n---\n# Weekly\n");
        let result = qualify_skill_files(
            &[file("SKILL.md", &clean), file("references/a.md", "FIXME x")],
            "skill",
        );
        assert!(result.pass);
        assert!(
            result
                .checks
                .iter()
                .any(|check| check.id == "no-todo-other" && !check.ok && check.level == "warn")
        );
    }

    #[test]
    fn distill_qualify_execution_keywords_require_scripts_as_warning() {
        assert_eq!(count_execution_keywords("自动化部署 deploy 脚本"), 4);
        let skill = "---\nname: Deployer\ndescription: 自动化部署脚本 deploy\n---\n# Deployer\n";
        let result = qualify_skill_files(&[file("SKILL.md", skill)], "skill");
        assert!(result.pass, "warnings must not fail");
        let scripts = result
            .checks
            .iter()
            .find(|check| check.id == "scripts-decision")
            .unwrap();
        assert!(!scripts.ok);
        assert_eq!(scripts.level, "warn");
        assert!(
            scripts
                .detail
                .as_deref()
                .unwrap()
                .contains("scripts/ 为空或缺失")
        );

        let result = qualify_skill_files(
            &[
                file("SKILL.md", skill),
                file("scripts/run.sh", "echo deploying now"),
            ],
            "skill",
        );
        let robust = result
            .checks
            .iter()
            .find(|check| check.id == "scripts-robust")
            .unwrap();
        assert!(!robust.ok);
        assert_eq!(
            robust.detail.as_deref(),
            Some("存在缺少 set -euo pipefail 的 Shell 脚本")
        );
    }

    #[test]
    fn distill_qualify_builds_files_from_xml_tags_or_wraps_document() {
        let summary = "<folder name=\"X\">\n<file path=\"SKILL.md\">\n---\nname: X\n</file>\n<file path='scripts/a.py'>try:\n  pass</file></folder>";
        let files = build_files_for_qualification(summary, "skill", "hint");
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].content, "---\nname: X");
        assert_eq!(files[1].path, "scripts/a.py");

        let files = build_files_for_qualification("plain", "skill", "hint");
        assert_eq!(files, vec![file("SKILL.md", "plain")]);
        let result = qualify_skill_files(&files, "skill");
        assert!(!result.pass);

        let files = build_files_for_qualification("# Workflow", "brief", "2 场会话蒸馏产物");
        assert_eq!(files[1].path, "WORKFLOW.md");
        assert!(files[0].content.contains("description: 2 场会话蒸馏产物"));
        assert!(qualify_skill_files(&files, "brief").pass);
    }

    #[test]
    fn distill_qualify_failure_summary_uses_aitracker_format() {
        let result = qualify_skill_files(&[file("README.md", "x")], "skill");
        assert_eq!(
            failure_summary(&result),
            "包含 SKILL.md 与 YAML frontmatter（缺少 SKILL.md 文件）"
        );
    }
}
