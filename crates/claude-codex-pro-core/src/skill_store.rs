//! Skills 单一真源（SSOT）存储。
//!
//! 真源目录为 `<CCP home>/skills/`，每个 Skill 一个目录 `<root>/<id>/SKILL.md`，
//! 索引文件为 `<root>/index.json`。导入把外部 Skill 目录复制进真源库，同步再把
//! 真源目录复制到 Codex / Claude 的 `skills` 根目录。
//!
//! 参考 `unified_tool_inventory.rs` 的既有实现（`copy_skill_dir` 与
//! `claude_home_path`），但本模块独立实现，不修改那个文件。

use std::io::Read;
use std::path::{Component, Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

/// 真源目录版本号，随目录结构变化递增。
pub const STORE_VERSION: u32 = 1;

const SKILLS_DIR: &str = "skills";
const INDEX_FILE: &str = "index.json";
const SKILL_FILE: &str = "SKILL.md";
const UNKNOWN_SOURCE_APP: &str = "unknown";
const MAX_METADATA_BYTES: u64 = 64 * 1024;

/// 真源库中的一个 Skill。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillStoreEntry {
    pub id: String,
    pub name: String,
    pub source_app: String,
    pub imported_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillStoreIndex {
    version: u32,
    #[serde(default)]
    entries: Vec<SkillStoreEntry>,
}

/// 真源根目录：`<CCP home>/skills`。
///
/// `home` 由调用方用 [`crate::paths::default_app_state_dir`] 解析，模块内不硬编码。
pub fn skill_store_root(home: &Path) -> PathBuf {
    home.join(SKILLS_DIR)
}

/// 列出真源库中的 Skill，并顺手校正索引。
///
/// 以磁盘目录为准：只有"真实目录（非符号链接）+ 含 `SKILL.md`"的条目算 Skill。
/// 索引缺失、无法读取或 JSON 损坏时按目录重建并写回。
pub fn list_store_skills(home: &Path) -> anyhow::Result<Vec<SkillStoreEntry>> {
    let root = skill_store_root(home);
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let previous = read_index(home).ok().flatten();
    let entries = collect_entries(&root, previous.as_ref())?;

    let index_path = root.join(INDEX_FILE);
    let bytes = index_bytes(&entries)?;
    let existing = std::fs::read(&index_path).ok();
    if existing.as_deref() != Some(bytes.as_slice()) {
        crate::settings::atomic_write(&index_path, &bytes)
            .with_context(|| format!("写入 Skill 真源索引失败：{}", display_path(&index_path)))?;
    }
    Ok(entries)
}

/// 把 `source_dir` 复制进真源库，返回新条目。
///
/// - 源目录必须含 `SKILL.md`；
/// - `id` 取源目录最后一段目录名并做逃逸校验；
/// - 同 id 已存在时拒绝，不覆盖；
/// - 递归复制普通文件与目录，跳过符号链接。
pub fn import_skill_to_store(
    home: &Path,
    source_dir: &Path,
    source_app: &str,
) -> anyhow::Result<SkillStoreEntry> {
    if !source_dir.is_dir() {
        anyhow::bail!("Skill 源目录不存在：{}", display_path(source_dir));
    }
    if !source_dir.join(SKILL_FILE).is_file() {
        anyhow::bail!("Skill 源目录缺少 SKILL.md：{}", display_path(source_dir));
    }
    let id = source_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("Skill 源目录名无效：{}", display_path(source_dir)))?
        .to_string();
    validate_skill_id(&id)
        .with_context(|| format!("Skill 源目录名不能作为 ID：{}", display_path(source_dir)))?;
    let source_app = source_app.trim();
    if source_app.is_empty() {
        anyhow::bail!("Skill 来源应用不能为空");
    }

    let root = skill_store_root(home);
    let destination = root.join(&id);
    if destination.exists() {
        anyhow::bail!("Skill 已存在于真源库：{}", display_path(&destination));
    }
    crate::settings::create_private_dir_all(&root)
        .with_context(|| format!("创建 Skill 真源目录失败：{}", display_path(&root)))?;

    if let Err(error) = copy_skill_dir(source_dir, &destination) {
        let _ = std::fs::remove_dir_all(&destination);
        return Err(error).with_context(|| {
            format!(
                "复制 Skill 到真源库失败：{} → {}",
                display_path(source_dir),
                display_path(&destination)
            )
        });
    }

    let entry = SkillStoreEntry {
        id: id.clone(),
        name: skill_display_name(&destination, &id),
        source_app: source_app.to_string(),
        imported_at: chrono::Utc::now().to_rfc3339(),
    };

    let mut entries = read_index(home)
        .ok()
        .flatten()
        .map(|index| index.entries)
        .unwrap_or_default();
    entries.retain(|existing| existing.id != id);
    entries.push(entry.clone());
    entries.sort_by(|left, right| left.id.cmp(&right.id));

    let index_path = root.join(INDEX_FILE);
    let bytes = index_bytes(&entries)?;
    if let Err(error) = crate::settings::atomic_write(&index_path, &bytes) {
        // 索引写不进去时不留"库存里有、索引里没有"的半成品。
        let _ = std::fs::remove_dir_all(&destination);
        return Err(error)
            .with_context(|| format!("写入 Skill 真源索引失败：{}", display_path(&index_path)));
    }
    Ok(entry)
}

/// 把真源库中的 Skill 同步到应用目录。
///
/// `app` 只支持 `"codex"` 与 `"claude"`，目标分别是
/// `<user home>/.codex/skills/<id>` 与 `<user home>/.claude/skills/<id>`。
/// 目标已存在时拒绝。返回目标 Skill 目录。
pub fn sync_skill_to_app(home: &Path, id: &str, app: &str) -> anyhow::Result<PathBuf> {
    validate_skill_id(id)?;

    let user_home = user_home_from_ccp_home(home)?;
    let target_root = match app {
        "codex" => user_home.join(".codex"),
        "claude" => claude_home_path(&user_home),
        other => anyhow::bail!("不支持的 Skill 目标应用：{other}"),
    }
    .join(SKILLS_DIR);

    let source = skill_store_root(home).join(id);
    if !source.is_dir() || !source.join(SKILL_FILE).is_file() {
        anyhow::bail!("真源库中未找到 Skill：{id}");
    }

    let destination = target_root.join(id);
    if destination.exists() {
        anyhow::bail!("目标应用已存在同名 Skill：{}", display_path(&destination));
    }
    std::fs::create_dir_all(&target_root)
        .with_context(|| format!("创建 Skill 目录失败：{}", display_path(&target_root)))?;

    if let Err(error) = copy_skill_dir(&source, &destination) {
        let _ = std::fs::remove_dir_all(&destination);
        return Err(error).with_context(|| {
            format!(
                "同步 Skill 失败：{} → {}",
                display_path(&source),
                display_path(&destination)
            )
        });
    }
    Ok(destination)
}

/// 校验 `id`：必须是单段普通目录名。
///
/// 拒绝空串、`.`、`..`、含 `/` 或 `\`、含 `..`、带盘符前缀等写法，防止路径逃逸。
fn validate_skill_id(id: &str) -> anyhow::Result<()> {
    if id.trim().is_empty() {
        anyhow::bail!("Skill ID 无效：{id}（不能为空，必须是单段目录名）");
    }
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        anyhow::bail!("Skill ID 无效：{id}（不能包含路径分隔符或 `..`，必须是单段目录名）");
    }
    let mut components = Path::new(id).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(()),
        _ => anyhow::bail!("Skill ID 无效：{id}（必须是单段目录名）"),
    }
}

/// 从 CCP home 派生用户主目录。
///
/// 默认 CCP home 为 `~/.claude-codex-pro`，其父目录即 `~`。测试通过把 CCP home
/// 放在临时"用户主目录"下来隔离真实环境。
fn user_home_from_ccp_home(home: &Path) -> anyhow::Result<PathBuf> {
    home.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow::anyhow!("无法从 CCP 目录派生用户主目录：{}", display_path(home)))
}

/// Claude 配置目录，与 `unified_tool_inventory.rs` 中 `claude_home_path` 的约定一致。
fn claude_home_path(user_home: &Path) -> PathBuf {
    user_home.join(".claude")
}

fn read_index(home: &Path) -> anyhow::Result<Option<SkillStoreIndex>> {
    let path = skill_store_root(home).join(INDEX_FILE);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("读取 Skill 真源索引失败：{}", display_path(&path)));
        }
    };
    let index = serde_json::from_slice::<SkillStoreIndex>(&bytes)
        .with_context(|| format!("解析 Skill 真源索引失败：{}", display_path(&path)))?;
    Ok(Some(index))
}

fn index_bytes(entries: &[SkillStoreEntry]) -> anyhow::Result<Vec<u8>> {
    let index = SkillStoreIndex {
        version: STORE_VERSION,
        entries: entries.to_vec(),
    };
    let mut bytes = serde_json::to_vec_pretty(&index).context("序列化 Skill 真源索引失败")?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// 按磁盘目录重建条目，并沿用索引中已有的 `source_app` / `imported_at`。
fn collect_entries(
    root: &Path,
    previous: Option<&SkillStoreIndex>,
) -> anyhow::Result<Vec<SkillStoreEntry>> {
    let mut known = std::collections::HashMap::<String, SkillStoreEntry>::new();
    if let Some(index) = previous {
        for entry in &index.entries {
            known.insert(entry.id.clone(), entry.clone());
        }
    }

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(root)
        .with_context(|| format!("读取 Skill 真源目录失败：{}", display_path(root)))?
    {
        let entry =
            entry.with_context(|| format!("读取 Skill 真源条目失败：{}", display_path(root)))?;
        let file_type = entry.file_type().with_context(|| {
            format!(
                "读取 Skill 真源条目类型失败：{}",
                display_path(&entry.path())
            )
        })?;
        if file_type.is_symlink() || !file_type.is_dir() {
            continue;
        }
        let Some(id) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if validate_skill_id(&id).is_err() {
            continue;
        }
        let dir = entry.path();
        if !dir.join(SKILL_FILE).is_file() {
            continue;
        }
        let prior = known.get(&id);
        entries.push(SkillStoreEntry {
            name: skill_display_name(&dir, &id),
            source_app: prior
                .map(|entry| entry.source_app.clone())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| UNKNOWN_SOURCE_APP.to_string()),
            imported_at: prior
                .map(|entry| entry.imported_at.clone())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| modified_rfc3339(&dir)),
            id,
        });
    }
    entries.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(entries)
}

/// 读取 `SKILL.md` frontmatter 的 `name:`，缺失或为空时回退为目录名。
fn skill_display_name(skill_dir: &Path, fallback: &str) -> String {
    let path = skill_dir.join(SKILL_FILE);
    let Ok(metadata) = read_limited_text(&path, MAX_METADATA_BYTES) else {
        return fallback.to_string();
    };
    frontmatter_name(&metadata).unwrap_or_else(|| fallback.to_string())
}

fn read_limited_text(path: &Path, limit: u64) -> anyhow::Result<String> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("读取文件失败：{}", display_path(path)))?;
    let mut buffer = Vec::new();
    file.take(limit)
        .read_to_end(&mut buffer)
        .with_context(|| format!("读取文件失败：{}", display_path(path)))?;
    Ok(String::from_utf8_lossy(&buffer).to_string())
}

/// 单行 frontmatter 取值：只认 `---` 围栏内的 `name:` 行。
fn frontmatter_name(body: &str) -> Option<String> {
    let body = body.strip_prefix('\u{feff}').unwrap_or(body);
    let body = body.trim_start_matches(|character| character == '\r' || character == '\n');
    let rest = body.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    for line in rest[..end].lines() {
        let Some(value) = line.trim().strip_prefix("name:") else {
            continue;
        };
        let value = value
            .trim()
            .trim_matches(|character| character == '"' || character == '\'')
            .trim();
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

fn modified_rfc3339(path: &Path) -> String {
    let Ok(modified) = std::fs::metadata(path).and_then(|metadata| metadata.modified()) else {
        return chrono::Utc::now().to_rfc3339();
    };
    let datetime: chrono::DateTime<chrono::Utc> = modified.into();
    datetime.to_rfc3339()
}

/// 递归复制普通文件与目录；符号链接直接跳过（不跟随、不复制链接本身）。
fn copy_skill_dir(source: &Path, destination: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(destination)
        .with_context(|| format!("创建目录失败：{}", display_path(destination)))?;
    for entry in std::fs::read_dir(source)
        .with_context(|| format!("读取目录失败：{}", display_path(source)))?
    {
        let entry = entry.with_context(|| format!("读取目录条目失败：{}", display_path(source)))?;
        let file_type = entry
            .file_type()
            .with_context(|| format!("读取条目类型失败：{}", display_path(&entry.path())))?;
        if file_type.is_symlink() {
            continue;
        }
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_skill_dir(&entry.path(), &target)?;
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), &target).with_context(|| {
                format!(
                    "复制文件失败：{} → {}",
                    display_path(&entry.path()),
                    display_path(&target)
                )
            })?;
        }
    }
    Ok(())
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_name_reads_only_the_fenced_block() {
        assert_eq!(
            frontmatter_name("---\nname: demo\n---\n# body\n"),
            Some("demo".to_string())
        );
        assert_eq!(
            frontmatter_name("---\nname: \"quoted\"\n---\n"),
            Some("quoted".to_string())
        );
        assert_eq!(frontmatter_name("name: no-fence\n"), None);
    }

    #[test]
    fn validate_skill_id_rejects_escape_attempts() {
        for bad in ["", "  ", ".", "..", "../escape", "a/b", "a\\b", "a..b"] {
            assert!(validate_skill_id(bad).is_err(), "should reject {bad:?}");
        }
        assert!(validate_skill_id("review-helper").is_ok());
    }
}
