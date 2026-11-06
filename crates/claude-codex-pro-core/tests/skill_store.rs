use std::fs;
use std::path::{Path, PathBuf};

use claude_codex_pro_core::skill_store::{
    import_skill_to_store, list_store_skills, skill_store_root, sync_skill_to_app,
};
use tempfile::TempDir;

/// 临时"用户主目录" + 其下的 CCP home（`.claude-codex-pro`），
/// 因此同步目标 `<user home>/.codex`、`<user home>/.claude` 完全落在临时目录内。
struct Fixture {
    temp: TempDir,
    home: PathBuf,
    user_home: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let user_home = temp.path().join("user");
        let home = user_home.join(".claude-codex-pro");
        fs::create_dir_all(&home).unwrap();
        Self {
            temp,
            home,
            user_home,
        }
    }

    fn source_skill(&self, id: &str) -> PathBuf {
        let dir = self.temp.path().join("sources").join(id);
        fs::create_dir_all(dir.join("assets")).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {id}\n---\n# {id}\n"),
        )
        .unwrap();
        fs::write(dir.join("assets/note.txt"), "hello").unwrap();
        dir
    }
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

#[test]
fn import_copies_skill_and_lists_it_with_an_index() {
    let fixture = Fixture::new();
    let source = fixture.source_skill("demo-skill");

    let entry = import_skill_to_store(&fixture.home, &source, "codex").unwrap();

    assert_eq!(entry.id, "demo-skill");
    assert_eq!(entry.name, "demo-skill");
    assert_eq!(entry.source_app, "codex");
    chrono::DateTime::parse_from_rfc3339(&entry.imported_at).unwrap();

    let stored = skill_store_root(&fixture.home).join("demo-skill");
    assert_eq!(
        read(&stored.join("SKILL.md")),
        read(&source.join("SKILL.md"))
    );
    assert_eq!(read(&stored.join("assets/note.txt")), "hello");

    let listed = list_store_skills(&fixture.home).unwrap();
    assert_eq!(listed, vec![entry]);

    let index: serde_json::Value =
        serde_json::from_str(&read(&skill_store_root(&fixture.home).join("index.json"))).unwrap();
    assert_eq!(index["version"], 1);
    assert_eq!(index["entries"][0]["id"], "demo-skill");
}

#[test]
fn import_without_skill_md_is_rejected() {
    let fixture = Fixture::new();
    let source = fixture.temp.path().join("sources/no-skill-md");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("note.txt"), "not a skill").unwrap();

    let error = import_skill_to_store(&fixture.home, &source, "codex").unwrap_err();

    assert!(error.to_string().contains("SKILL.md"), "{error}");
    assert!(!skill_store_root(&fixture.home).join("no-skill-md").exists());
}

#[test]
fn duplicate_import_is_rejected_without_overwriting() {
    let fixture = Fixture::new();
    let source = fixture.source_skill("demo-skill");
    import_skill_to_store(&fixture.home, &source, "codex").unwrap();

    fs::write(source.join("SKILL.md"), "---\nname: changed\n---\n").unwrap();
    let error = import_skill_to_store(&fixture.home, &source, "claude").unwrap_err();

    let destination = skill_store_root(&fixture.home).join("demo-skill");
    assert!(
        error
            .to_string()
            .contains(&destination.to_string_lossy().to_string()),
        "{error}"
    );
    assert!(read(&destination.join("SKILL.md")).contains("name: demo-skill"));
    assert_eq!(list_store_skills(&fixture.home).unwrap().len(), 1);
}

#[test]
fn sync_copies_skill_to_codex() {
    let fixture = Fixture::new();
    let source = fixture.source_skill("demo-skill");
    import_skill_to_store(&fixture.home, &source, "codex").unwrap();

    let target = sync_skill_to_app(&fixture.home, "demo-skill", "codex").unwrap();

    assert_eq!(
        target,
        fixture
            .user_home
            .join(".codex")
            .join("skills")
            .join("demo-skill")
    );
    assert_eq!(
        read(&target.join("SKILL.md")),
        read(&source.join("SKILL.md"))
    );
    assert_eq!(read(&target.join("assets/note.txt")), "hello");
}

#[test]
fn sync_copies_skill_to_claude_and_rejects_unknown_app() {
    let fixture = Fixture::new();
    let source = fixture.source_skill("demo-skill");
    import_skill_to_store(&fixture.home, &source, "codex").unwrap();

    let target = sync_skill_to_app(&fixture.home, "demo-skill", "claude").unwrap();
    assert_eq!(
        target,
        fixture
            .user_home
            .join(".claude")
            .join("skills")
            .join("demo-skill")
    );
    assert!(target.join("SKILL.md").is_file());

    let error = sync_skill_to_app(&fixture.home, "demo-skill", "workbuddy").unwrap_err();
    assert!(error.to_string().contains("workbuddy"), "{error}");
}

#[test]
fn sync_rejects_existing_target() {
    let fixture = Fixture::new();
    let source = fixture.source_skill("demo-skill");
    import_skill_to_store(&fixture.home, &source, "codex").unwrap();

    let occupied = fixture
        .user_home
        .join(".codex")
        .join("skills")
        .join("demo-skill");
    fs::create_dir_all(&occupied).unwrap();
    fs::write(occupied.join("SKILL.md"), "occupied").unwrap();

    let error = sync_skill_to_app(&fixture.home, "demo-skill", "codex").unwrap_err();

    assert!(
        error
            .to_string()
            .contains(&occupied.to_string_lossy().to_string()),
        "{error}"
    );
    assert_eq!(read(&occupied.join("SKILL.md")), "occupied");
}

#[test]
fn sync_rejects_ids_that_escape_the_store() {
    let fixture = Fixture::new();
    let source = fixture.source_skill("demo-skill");
    import_skill_to_store(&fixture.home, &source, "codex").unwrap();

    for id in ["../escape", "a/b", "a\\b", "..", "", "  "] {
        let error = sync_skill_to_app(&fixture.home, id, "codex").unwrap_err();
        assert!(error.to_string().contains("Skill ID"), "{id:?}: {error}");
    }
    assert!(!fixture.user_home.join(".codex").exists());

    let escape = fixture.temp.path().join("escape");
    assert!(!escape.exists());
}

#[test]
fn corrupt_index_is_rebuilt_from_the_disk_directories() {
    let fixture = Fixture::new();
    let source = fixture.source_skill("demo-skill");
    import_skill_to_store(&fixture.home, &source, "codex").unwrap();
    let other = fixture.source_skill("other-skill");
    import_skill_to_store(&fixture.home, &other, "claude").unwrap();

    let index_path = skill_store_root(&fixture.home).join("index.json");
    fs::write(&index_path, "{ this is not json").unwrap();

    let listed = list_store_skills(&fixture.home).unwrap();
    let ids = listed
        .iter()
        .map(|entry| entry.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["demo-skill", "other-skill"]);

    let rebuilt: serde_json::Value = serde_json::from_str(&read(&index_path)).unwrap();
    assert_eq!(rebuilt["version"], 1);
    assert_eq!(rebuilt["entries"][0]["id"], "demo-skill");

    fs::remove_dir_all(skill_store_root(&fixture.home).join("other-skill")).unwrap();
    let listed = list_store_skills(&fixture.home).unwrap();
    assert_eq!(
        listed
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        vec!["demo-skill"]
    );
}

#[test]
fn sync_rejects_a_skill_missing_from_the_store() {
    let fixture = Fixture::new();

    let error = sync_skill_to_app(&fixture.home, "ghost-skill", "codex").unwrap_err();

    assert!(error.to_string().contains("ghost-skill"), "{error}");
    assert!(!fixture.user_home.join(".codex").exists());
}

#[cfg(unix)]
#[test]
fn import_skips_symbolic_links() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    let source = fixture.source_skill("demo-skill");
    symlink(source.join("SKILL.md"), source.join("linked.md")).unwrap();

    import_skill_to_store(&fixture.home, &source, "codex").unwrap();

    let stored = skill_store_root(&fixture.home).join("demo-skill");
    assert!(stored.join("SKILL.md").is_file());
    assert!(!stored.join("linked.md").exists());
}
