use claude_codex_pro_core::memory_assist::{
    MemoryAssistStore, MemoryCandidateRequest, MemoryImportRequest, MemoryItemRequest,
    MemoryQueryRequest, MemorySelfCheckRequest, MemorySessionRequest,
};
use rusqlite::{Connection, params};
use std::sync::{Mutex, OnceLock};

static CODEX_HOME_LOCK: Mutex<()> = Mutex::new(());
static TEST_CODEX_HOME: OnceLock<tempfile::TempDir> = OnceLock::new();

fn init_empty_codex_home_locked() -> &'static tempfile::TempDir {
    TEST_CODEX_HOME.get_or_init(|| {
        let dir = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("CODEX_HOME", dir.path());
        }
        dir
    })
}

fn ensure_empty_codex_home() {
    let _guard = CODEX_HOME_LOCK.lock().unwrap();
    init_empty_codex_home_locked();
}

fn with_codex_home_env<T>(f: impl FnOnce() -> T) -> T {
    let _guard = CODEX_HOME_LOCK.lock().unwrap();
    init_empty_codex_home_locked();
    f()
}

fn with_temporary_codex_home<T>(codex_home: &std::path::Path, f: impl FnOnce() -> T) -> T {
    let _guard = CODEX_HOME_LOCK.lock().unwrap();
    let empty_codex_home = init_empty_codex_home_locked().path().to_path_buf();
    unsafe {
        std::env::set_var("CODEX_HOME", codex_home);
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    unsafe {
        std::env::set_var("CODEX_HOME", empty_codex_home);
    }
    match result {
        Ok(value) => value,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

fn store_at(path: &std::path::Path) -> MemoryAssistStore {
    ensure_empty_codex_home();
    MemoryAssistStore::new(path.to_path_buf())
}

#[test]
fn learn_query_and_workspace_filter_use_project_plus_global_scope() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_at(&temp.path().join("memory.sqlite"));

    store
        .learn_item(MemoryItemRequest {
            text: "项目约定：构建管理工具使用 npm --prefix apps/claude-codex-pro-manager run vite:build".into(),
            workspace: "repo-a".into(),
            category: "build".into(),
            tags: vec!["codex".into()],
            source: "test".into(),
            source_session_id: "s1".into(),
        })
        .unwrap();
    store
        .learn_item(MemoryItemRequest {
            text: "全局偏好：大量改动前先备份源码".into(),
            workspace: "global".into(),
            category: "preference".into(),
            tags: vec!["backup".into()],
            source: "test".into(),
            source_session_id: "s2".into(),
        })
        .unwrap();
    store
        .learn_item(MemoryItemRequest {
            text: "另一个项目的供应商配置说明".into(),
            workspace: "repo-b".into(),
            category: "relay".into(),
            tags: vec![],
            source: "test".into(),
            source_session_id: "s3".into(),
        })
        .unwrap();

    let result = store
        .query(MemoryQueryRequest {
            query: "构建前需要备份吗".into(),
            workspace: "repo-a".into(),
            include_global: true,
            include_archived: false,
            limit: 8,
        })
        .unwrap();

    let texts = result
        .results
        .iter()
        .map(|item| item.item.text.as_str())
        .collect::<Vec<_>>();
    assert!(texts.iter().any(|text| text.contains("vite:build")));
    assert!(texts.iter().any(|text| text.contains("先备份源码")));
    assert!(!texts.iter().any(|text| text.contains("另一个项目")));
    assert_eq!(
        with_codex_home_env(|| store.status()).unwrap().total_items,
        3
    );
}

#[test]
fn all_workspaces_scope_lists_and_searches_every_workspace() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_at(&temp.path().join("memory.sqlite"));

    for (workspace, text) in [
        ("codex:repo:a", "A 项目使用 vite:build 构建管理工具"),
        ("codex:repo:b", "B 项目需要修复插件安装 diff"),
        ("global", "全局偏好：大量改动前先备份"),
    ] {
        store
            .learn_item(MemoryItemRequest {
                text: text.into(),
                workspace: workspace.into(),
                category: "test".into(),
                tags: vec![],
                source: "test".into(),
                source_session_id: "s".into(),
            })
            .unwrap();
        store
            .create_candidate(MemoryCandidateRequest {
                text: format!("{workspace} 待确认记忆"),
                workspace: workspace.into(),
                category: "candidate".into(),
                tags: vec![],
                source: "test".into(),
                reason: "test".into(),
                source_session_id: "s".into(),
            })
            .unwrap();
    }

    let items = store
        .list_items(MemoryQueryRequest {
            query: String::new(),
            workspace: "__all__".into(),
            include_global: true,
            include_archived: false,
            limit: 20,
        })
        .unwrap();
    let workspaces = items
        .iter()
        .map(|item| item.workspace.as_str())
        .collect::<Vec<_>>();
    assert!(workspaces.contains(&"codex:repo:a"));
    assert!(workspaces.contains(&"codex:repo:b"));
    assert!(workspaces.contains(&"global"));

    let search = store
        .query(MemoryQueryRequest {
            query: "插件 diff".into(),
            workspace: "__all__".into(),
            include_global: true,
            include_archived: false,
            limit: 20,
        })
        .unwrap();
    assert!(
        search
            .results
            .iter()
            .any(|item| item.item.workspace == "codex:repo:b")
    );

    let candidates = store.list_candidates("__all__", true).unwrap();
    assert_eq!(candidates.len(), 3);
}

#[test]
fn status_includes_capture_and_codex_session_workspaces_without_auto_approving() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_at(&temp.path().join("memory.sqlite"));
    let codex_home = temp.path().join("codex-home");
    let sqlite_dir = codex_home.join("sqlite");
    std::fs::create_dir_all(&sqlite_dir).unwrap();
    let rollout_path = codex_home.join("rollout.jsonl");
    std::fs::write(
        &rollout_path,
        format!(
            "{}\n{}\n",
            serde_json::json!({
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": "user",
                    "content": [{
                        "type": "input_text",
                        "text": "这个项目必须保留 Harness Engineering 工作流。"
                    }]
                }
            }),
            serde_json::json!({
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": "收到"}]
                }
            })
        ),
    )
    .unwrap();
    let db_path = sqlite_dir.join("codex-dev.db");
    let db = Connection::open(&db_path).unwrap();
    db.execute(
        "CREATE TABLE threads (
            id TEXT PRIMARY KEY,
            title TEXT,
            cwd TEXT,
            rollout_path TEXT,
            updated_at_ms INTEGER
        )",
        [],
    )
    .unwrap();
    db.execute(
        "CREATE TABLE local_thread_catalog (
            id TEXT PRIMARY KEY,
            path TEXT,
            updated_at_ms INTEGER
        )",
        [],
    )
    .unwrap();
    db.execute(
        "INSERT INTO threads (id, title, cwd, rollout_path, updated_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        (
            "thread-1",
            "history",
            "D:\\Project\\Claude-Codex-Pro-Tool",
            rollout_path.to_string_lossy().to_string(),
            1000_i64,
        ),
    )
    .unwrap();
    db.execute(
        "INSERT INTO local_thread_catalog (id, path, updated_at_ms)
         VALUES (?1, ?2, ?3)",
        ("catalog-1", "D:\\Project\\toporeduce", 999_i64),
    )
    .unwrap();
    drop(db);

    let status = store.status_from_codex_home(&codex_home).unwrap();

    assert_eq!(status.total_items, 0);
    assert_eq!(
        status.pending_candidates, 0,
        "status refresh may backfill captures but must not generate candidates"
    );
    assert_eq!(status.total_captures, 2);
    let repo = status
        .workspaces
        .iter()
        .find(|workspace| workspace.workspace == "D:\\Project\\Claude-Codex-Pro-Tool")
        .expect("thread cwd workspace should be visible");
    assert_eq!(repo.capture_count, 2);
    assert_eq!(repo.session_count, 1);
    assert!(repo.latest_capture_at > 0);
    let catalog = status
        .workspaces
        .iter()
        .find(|workspace| workspace.workspace == "D:\\Project\\toporeduce")
        .expect("catalog workspace should be visible");
    assert_eq!(catalog.capture_count, 0);
    assert_eq!(catalog.session_count, 1);
}

#[test]
fn codex_history_backfill_ignores_internal_context_and_is_idempotent() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_at(&temp.path().join("memory.sqlite"));
    let codex_home = temp.path().join("codex-home");
    let sqlite_dir = codex_home.join("sqlite");
    std::fs::create_dir_all(&sqlite_dir).unwrap();
    let rollout_path = codex_home.join("rollout.jsonl");
    std::fs::write(
        &rollout_path,
        format!(
            "{}\n{}\n{}\n{}\n",
            serde_json::json!({
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": "user",
                    "content": [{
                        "type": "input_text",
                        "text": "<environment_context><cwd>D:\\Project\\Claude-Codex-Pro-Tool</cwd></environment_context>"
                    }]
                }
            }),
            serde_json::json!({
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": "user",
                    "content": [{
                        "type": "input_text",
                        "text": "<codex_internal_context source=\"goal\">Continue working toward the active thread goal.</codex_internal_context>"
                    }]
                }
            }),
            serde_json::json!({
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": "user",
                    "content": [{
                        "type": "input_text",
                        "text": "# Files mentioned by the user:\n\n## screenshot.png: C:/tmp/screenshot.png\n\n## My request for Codex:\nThis project must keep Pangu memory capture evidence even when no candidate is generated."
                    }, {
                        "type": "input_text",
                        "text": "<image name=[Image #1] path=\"C:\\tmp\\screenshot.png\">"
                    }]
                }
            }),
            serde_json::json!({
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": "done"}]
                }
            })
        ),
    )
    .unwrap();
    let db_path = sqlite_dir.join("codex-dev.db");
    let db = Connection::open(&db_path).unwrap();
    db.execute(
        "CREATE TABLE threads (
            id TEXT PRIMARY KEY,
            title TEXT,
            cwd TEXT,
            rollout_path TEXT,
            updated_at_ms INTEGER
        )",
        [],
    )
    .unwrap();
    db.execute(
        "INSERT INTO threads (id, title, cwd, rollout_path, updated_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        (
            "thread-filter",
            "history",
            "repo-filter",
            rollout_path.to_string_lossy().to_string(),
            1000_i64,
        ),
    )
    .unwrap();
    drop(db);

    let first = store.status_from_codex_home(&codex_home).unwrap();
    assert_eq!(first.total_captures, 2);

    let conn = Connection::open(store.db_path()).unwrap();
    conn.execute(
        "INSERT INTO memory_captures
         (id, workspace, source, source_session_id, text_length, text_hash, summary,
          candidate_triggered, candidate_reason, skip_reason, captured_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, '', 'history_not_learnable', ?8, ?8)",
        params![
            "cap_internal_old",
            "repo-filter",
            "codex-history-rollout",
            "thread-filter",
            80_i64,
            "internal-old-hash",
            "<codex_internal_context source=\"goal\">Continue working toward the active thread goal.</codex_internal_context>",
            9_999_999_i64,
        ],
    )
    .unwrap();
    let (summary, first_updated_at): (String, i64) = conn
        .query_row(
            "SELECT summary, updated_at FROM memory_captures
             WHERE workspace = 'repo-filter'
               AND source = 'codex-history-rollout-user'
               AND summary LIKE '%Pangu memory capture evidence%'
             ORDER BY updated_at DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert!(summary.contains("Pangu memory capture evidence"));
    assert!(!summary.contains("environment_context"));
    assert!(!summary.contains("codex_internal_context"));
    assert!(!summary.contains("<image"));
    drop(conn);

    std::thread::sleep(std::time::Duration::from_secs(1));
    let second = store.status_from_codex_home(&codex_home).unwrap();
    assert_eq!(second.total_captures, 2);
    let workspace = second
        .workspaces
        .iter()
        .find(|workspace| workspace.workspace == "repo-filter")
        .expect("valid capture workspace");
    assert_eq!(workspace.capture_count, 2);
    let conn = Connection::open(store.db_path()).unwrap();
    let second_updated_at: i64 = conn
        .query_row(
            "SELECT updated_at FROM memory_captures
             WHERE workspace = 'repo-filter'
               AND source = 'codex-history-rollout-user'
               AND summary LIKE '%Pangu memory capture evidence%'
             ORDER BY updated_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(first_updated_at, second_updated_at);
    let session = with_codex_home_env(|| {
        store.session_summary(MemorySessionRequest {
            workspace: "repo-filter".into(),
            query: "Pangu memory capture evidence".into(),
            max_items: 5,
        })
    })
    .unwrap();
    assert_eq!(session.recent_captures.len(), 2);
    assert!(!session.capture_summary.contains("codex_internal_context"));
}

#[test]
fn learn_item_redacts_secret_values_before_storage() {
    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("memory.sqlite");
    let store = store_at(&db_path);

    let item = store
        .learn_item(MemoryItemRequest {
            text: "API key 是 sk-secret123，Authorization: Bearer abc.def.ghi".into(),
            workspace: "repo-a".into(),
            category: "secret-test".into(),
            tags: vec![],
            source: "test".into(),
            source_session_id: "session sk-source-session".into(),
        })
        .unwrap();
    store
        .create_candidate(MemoryCandidateRequest {
            text: "候选 API key 是 sk-candidate-text".into(),
            workspace: "repo-a".into(),
            category: "secret-test".into(),
            tags: vec![],
            source: "test".into(),
            reason: "because Bearer candidate.reason.secret".into(),
            source_session_id: "session sk-candidate-session".into(),
        })
        .unwrap();

    assert!(!item.text.contains("sk-secret123"));
    assert!(!item.text.contains("abc.def.ghi"));
    assert!(item.text.contains("sk-***"));
    assert!(item.text.contains("Bearer ***"));

    let conn = Connection::open(db_path).unwrap();
    let raw_items: String = conn
        .query_row(
            "SELECT text || ' ' || source_session_id || ' ' || keywords FROM memory_items LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let raw_candidates: String = conn
        .query_row(
            "SELECT text || ' ' || reason || ' ' || source_session_id || ' ' || keywords FROM memory_candidates LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let raw = format!("{raw_items}\n{raw_candidates}");
    for leaked in [
        "sk-source-session",
        "sk-candidate-text",
        "candidate.reason.secret",
        "sk-candidate-session",
    ] {
        assert!(
            !raw.contains(leaked),
            "raw SQLite contents must not contain secret fragment: {leaked}"
        );
    }
}

#[test]
fn redaction_handles_bearer_case_whitespace_and_source_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("memory.sqlite");
    let store = store_at(&db_path);

    let item = store
        .learn_item(MemoryItemRequest {
            text: "authorization: bearer lower.secret and AUTHORIZATION: BEARER upper.secret and Bearer\twith-tab.secret".into(),
            workspace: "repo-a".into(),
            category: "secret-test".into(),
            tags: vec![],
            source: "Bearer source.secret".into(),
            source_session_id: "session bearer session.secret".into(),
        })
        .unwrap();
    store
        .create_candidate(MemoryCandidateRequest {
            text: "candidate authorization: bearer candidate.secret".into(),
            workspace: "repo-a".into(),
            category: "secret-test".into(),
            tags: vec![],
            source: "BEARER candidate.source.secret".into(),
            reason: "because bearer candidate.reason.secret".into(),
            source_session_id: "session BEARER candidate.session.secret".into(),
        })
        .unwrap();

    assert!(!item.text.contains("lower.secret"));
    assert!(!item.text.contains("upper.secret"));
    assert!(!item.text.contains("with-tab.secret"));
    assert!(item.text.matches("Bearer ***").count() >= 3);

    let conn = Connection::open(db_path).unwrap();
    let raw_items: String = conn
        .query_row(
            "SELECT text || ' ' || source || ' ' || source_session_id || ' ' || keywords FROM memory_items LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let raw_candidates: String = conn
        .query_row(
            "SELECT text || ' ' || source || ' ' || reason || ' ' || source_session_id || ' ' || keywords FROM memory_candidates LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let raw = format!("{raw_items}\n{raw_candidates}");
    for leaked in [
        "lower.secret",
        "upper.secret",
        "with-tab.secret",
        "source.secret",
        "session.secret",
        "candidate.secret",
        "candidate.source.secret",
        "candidate.reason.secret",
        "candidate.session.secret",
    ] {
        assert!(
            !raw.contains(leaked),
            "raw SQLite contents must not contain secret fragment: {leaked}"
        );
    }
}

#[test]
fn similar_memory_updates_existing_item_instead_of_creating_duplicate() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_at(&temp.path().join("memory.sqlite"));

    let first = store
        .learn_item(MemoryItemRequest {
            text: "README 需要记录 Windows 和 macOS 构建命令".into(),
            workspace: "repo-a".into(),
            category: "docs".into(),
            tags: vec!["readme".into()],
            source: "test".into(),
            source_session_id: "s1".into(),
        })
        .unwrap();
    let second = store
        .learn_item(MemoryItemRequest {
            text: "README 需要记录 Windows、macOS 构建命令和 CI 排查命令".into(),
            workspace: "repo-a".into(),
            category: "docs".into(),
            tags: vec!["ci".into()],
            source: "test".into(),
            source_session_id: "s2".into(),
        })
        .unwrap();

    assert_eq!(first.id, second.id);
    assert!(second.text.contains("CI"));
    let listed = store
        .list_items(MemoryQueryRequest {
            query: String::new(),
            workspace: "repo-a".into(),
            include_global: true,
            include_archived: false,
            limit: 20,
        })
        .unwrap();
    assert_eq!(listed.len(), 1);
}

#[test]
fn related_but_distinct_memory_does_not_overwrite_existing_fact() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_at(&temp.path().join("memory.sqlite"));

    let first = store
        .learn_item(MemoryItemRequest {
            text: "project alpha uses npm build and cargo tests".into(),
            workspace: "repo-a".into(),
            category: "build".into(),
            tags: vec![],
            source: "test".into(),
            source_session_id: "s1".into(),
        })
        .unwrap();
    let second = store
        .learn_item(MemoryItemRequest {
            text: "project alpha uses npm build and playwright checks".into(),
            workspace: "repo-a".into(),
            category: "build".into(),
            tags: vec![],
            source: "test".into(),
            source_session_id: "s2".into(),
        })
        .unwrap();

    assert_ne!(first.id, second.id);
    let listed = store
        .list_items(MemoryQueryRequest {
            query: String::new(),
            workspace: "repo-a".into(),
            include_global: true,
            include_archived: false,
            limit: 20,
        })
        .unwrap();
    let joined = listed
        .iter()
        .map(|item| item.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("cargo tests"));
    assert!(joined.contains("playwright checks"));
}

#[test]
fn list_items_does_not_increment_access_count() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_at(&temp.path().join("memory.sqlite"));

    let item = store
        .learn_item(MemoryItemRequest {
            text: "列表刷新不应该改变访问计数".into(),
            workspace: "repo-a".into(),
            category: "quality".into(),
            tags: vec![],
            source: "test".into(),
            source_session_id: "s1".into(),
        })
        .unwrap();
    assert_eq!(item.access_count, 0);

    let listed = store
        .list_items(MemoryQueryRequest {
            query: String::new(),
            workspace: "repo-a".into(),
            include_global: true,
            include_archived: false,
            limit: 20,
        })
        .unwrap();
    assert_eq!(listed[0].access_count, 0);

    let queried = store
        .query(MemoryQueryRequest {
            query: "列表刷新".into(),
            workspace: "repo-a".into(),
            include_global: true,
            include_archived: false,
            limit: 20,
        })
        .unwrap();
    assert_eq!(queried.results.len(), 1);

    let listed_after_query = store
        .list_items(MemoryQueryRequest {
            query: String::new(),
            workspace: "repo-a".into(),
            include_global: true,
            include_archived: false,
            limit: 20,
        })
        .unwrap();
    assert_eq!(listed_after_query[0].access_count, 1);
}

#[test]
fn similar_pending_candidate_updates_existing_candidate() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_at(&temp.path().join("memory.sqlite"));

    let first = store
        .create_candidate(MemoryCandidateRequest {
            text: "以后大量改动前先备份源码".into(),
            workspace: "repo-a".into(),
            category: "preference".into(),
            tags: vec!["backup".into()],
            source: "codex-dom-auto".into(),
            reason: "future rule phrase".into(),
            source_session_id: "s1".into(),
        })
        .unwrap();
    let second = store
        .create_candidate(MemoryCandidateRequest {
            text: "以后大量改动前先备份源码到 F 盘".into(),
            workspace: "repo-a".into(),
            category: "preference".into(),
            tags: vec!["backup".into(), "project".into()],
            source: "codex-dom-auto".into(),
            reason: "future rule phrase".into(),
            source_session_id: "s2".into(),
        })
        .unwrap();

    assert_eq!(first.id, second.id);
    assert!(second.text.contains("F 盘"));
    assert_eq!(store.list_candidates("repo-a", true).unwrap().len(), 1);
    assert!(second.tags.contains(&"project".to_string()));
}

#[test]
fn pending_candidate_can_be_approved_or_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_at(&temp.path().join("memory.sqlite"));

    let candidate = store
        .create_candidate(MemoryCandidateRequest {
            text: "以后插件安装必须先展示 diff".into(),
            workspace: "repo-a".into(),
            category: "safety".into(),
            tags: vec!["plugin".into()],
            source: "codex-dom".into(),
            reason: "explicit remember phrase".into(),
            source_session_id: "s1".into(),
        })
        .unwrap();
    assert_eq!(candidate.status, "pending");
    assert_eq!(store.list_candidates("repo-a", true).unwrap().len(), 1);

    let item = store.approve_candidate(&candidate.id).unwrap();
    assert!(item.text.contains("展示 diff"));
    assert!(store.list_candidates("repo-a", true).unwrap().is_empty());

    let rejected = store
        .create_candidate(MemoryCandidateRequest {
            text: "临时草稿，不应该进入长期记忆".into(),
            workspace: "repo-a".into(),
            category: "draft".into(),
            tags: vec![],
            source: "codex-dom".into(),
            reason: "test".into(),
            source_session_id: "s2".into(),
        })
        .unwrap();
    let rejected = store.reject_candidate(&rejected.id).unwrap();
    assert_eq!(rejected.status, "rejected");
}

#[test]
fn approved_candidate_cannot_be_rejected_later() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_at(&temp.path().join("memory.sqlite"));

    let candidate = store
        .create_candidate(MemoryCandidateRequest {
            text: "以后插件安装必须先展示 diff".into(),
            workspace: "repo-a".into(),
            category: "safety".into(),
            tags: vec![],
            source: "codex-dom".into(),
            reason: "explicit remember phrase".into(),
            source_session_id: "s1".into(),
        })
        .unwrap();

    store.approve_candidate(&candidate.id).unwrap();
    let error = store.reject_candidate(&candidate.id).unwrap_err();

    assert!(error.to_string().contains("candidate is not pending"));
}

#[test]
fn session_summary_limits_injected_items_and_reports_workspace_counts() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_at(&temp.path().join("memory.sqlite"));
    let texts = [
        "Codex 插件中心需要展示官方插件目录",
        "Codex 会话修复入口放在工具页面",
        "供应商配置切换后要同步历史会话",
        "README 需要记录构建和 CI 排查步骤",
        "Claude 中文包装窗口不修改官方 MSIX",
        "提示词优化器使用系统浏览器打开",
        "管理工具设置页开关使用滑块",
        "大量改动前先备份源码到 F 盘",
    ];
    for (idx, text) in texts.iter().enumerate() {
        store
            .learn_item(MemoryItemRequest {
                text: (*text).into(),
                workspace: "repo-a".into(),
                category: "codex".into(),
                tags: vec!["codex".into()],
                source: "test".into(),
                source_session_id: format!("s{idx}"),
            })
            .unwrap();
    }

    let summary = with_codex_home_env(|| {
        store.session_summary(MemorySessionRequest {
            workspace: "repo-a".into(),
            query: "插件中心会话修复".into(),
            max_items: 3,
        })
    })
    .unwrap();

    assert_eq!(summary.injected_items.len(), 3);
    assert_eq!(summary.workspace, "repo-a");
    assert_eq!(summary.total_items, 8);
    assert!(summary.summary.contains("repo-a"));
    let inject_cache = store.inject_summary_cache_path();
    assert!(inject_cache.exists());
    let inject_cache_content = std::fs::read_to_string(inject_cache).unwrap();
    assert!(inject_cache_content.contains("盘古记忆会话启动摘要"));
    assert!(inject_cache_content.contains("Codex 插件中心"));
}

#[test]
fn codex_history_backfill_records_captures_and_pending_candidates() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_at(&temp.path().join("memory.sqlite"));
    let codex_home = temp.path().join("codex-home");
    let sqlite_dir = codex_home.join("sqlite");
    std::fs::create_dir_all(&sqlite_dir).unwrap();
    let rollout_path = codex_home.join("rollout.jsonl");
    std::fs::write(
        &rollout_path,
        format!(
            "{}\n{}\n",
            serde_json::json!({
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": "user",
                    "content": [{
                        "type": "input_text",
                        "text": "这个项目必须先写规格再开发，并且验证后再交付。"
                    }]
                }
            }),
            serde_json::json!({
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": "收到"}]
                }
            })
        ),
    )
    .unwrap();
    let db_path = sqlite_dir.join("codex-dev.db");
    let db = Connection::open(&db_path).unwrap();
    db.execute(
        "CREATE TABLE threads (
            id TEXT PRIMARY KEY,
            title TEXT,
            cwd TEXT,
            rollout_path TEXT,
            updated_at_ms INTEGER
        )",
        [],
    )
    .unwrap();
    db.execute(
        "INSERT INTO threads (id, title, cwd, rollout_path, updated_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        (
            "thread-1",
            "history",
            "repo-history",
            rollout_path.to_string_lossy().to_string(),
            1000_i64,
        ),
    )
    .unwrap();
    drop(db);

    let report = store.backfill_codex_history_from_home(&codex_home, "", 5, true);
    assert_eq!(report.db_paths_checked, 1);
    assert_eq!(report.rollout_files_checked, 1);
    assert_eq!(report.user_messages_seen, 2);
    assert_eq!(report.captures_recorded, 2);
    assert_eq!(report.items_learned, 1);
    assert_eq!(report.candidates_created, 0);
    assert!(report.errors.is_empty(), "{:?}", report.errors);

    let status = with_codex_home_env(|| store.status()).unwrap();
    assert_eq!(status.total_items, 1);
    assert_eq!(status.pending_candidates, 0);
    assert!(std::path::Path::new(&status.inject_summary_cache_path).exists());
    let summary = with_codex_home_env(|| {
        store.session_summary(MemorySessionRequest {
            workspace: "repo-history".into(),
            query: "规格".into(),
            max_items: 5,
        })
    })
    .unwrap();
    assert_eq!(summary.recent_captures.len(), 2);
    assert_eq!(summary.injected_items.len(), 1);
    assert!(summary.recent_captures.iter().any(|capture| {
        capture
            .candidate_reason
            .contains("auto_learned: history workflow rule")
    }));
}

#[test]
fn codex_history_backfill_compacts_lessons_into_single_manual() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_at(&temp.path().join("memory.sqlite"));
    let codex_home = temp.path().join("codex-home");
    let sqlite_dir = codex_home.join("sqlite");
    std::fs::create_dir_all(&sqlite_dir).unwrap();
    let rollout_path = codex_home.join("rollout.jsonl");
    std::fs::write(
        &rollout_path,
        format!(
            "{}\n{}\n",
            serde_json::json!({
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": "user",
                    "content": [{
                        "type": "input_text",
                        "text": "经验教训：盘古记忆提炼后必须合成一条精简手册，不要生成很多条散乱卡片。"
                    }]
                }
            }),
            serde_json::json!({
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": "user",
                    "content": [{
                        "type": "input_text",
                        "text": "以后前端 UI 改动后必须重新构建 debug manager 并验证，否则用户看到的还是旧版。"
                    }]
                }
            })
        ),
    )
    .unwrap();
    let db_path = sqlite_dir.join("codex-dev.db");
    let db = Connection::open(&db_path).unwrap();
    db.execute(
        "CREATE TABLE threads (
            id TEXT PRIMARY KEY,
            title TEXT,
            cwd TEXT,
            rollout_path TEXT,
            updated_at_ms INTEGER
        )",
        [],
    )
    .unwrap();
    db.execute(
        "INSERT INTO threads (id, title, cwd, rollout_path, updated_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        (
            "thread-manual",
            "manual",
            "repo-history",
            rollout_path.to_string_lossy().to_string(),
            1000_i64,
        ),
    )
    .unwrap();
    drop(db);

    let report = store.backfill_codex_history_from_home(&codex_home, "", 20, true);
    assert_eq!(report.user_messages_seen, 2);
    assert_eq!(report.items_learned, 1);
    assert!(report.errors.is_empty(), "{:?}", report.errors);

    let items = store
        .list_items(MemoryQueryRequest {
            query: String::new(),
            workspace: "__all__".into(),
            include_global: true,
            include_archived: false,
            limit: 10,
        })
        .unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].category, "lesson-manual");
    assert!(items[0].text.starts_with("经验教训手册："));
    assert!(items[0].text.contains("合成一条精简手册"));
    assert!(items[0].text.contains("重新构建 debug manager"));
}

#[test]
fn session_summary_auto_learns_high_confidence_history_from_codex_home() {
    let temp = tempfile::tempdir().unwrap();
    let store = store_at(&temp.path().join("memory.sqlite"));
    let codex_home = temp.path().join("codex-home");
    let sqlite_dir = codex_home.join("sqlite");
    std::fs::create_dir_all(&sqlite_dir).unwrap();
    let rollout_path = codex_home.join("rollout.jsonl");
    std::fs::write(
        &rollout_path,
        format!(
            "{}\n",
            serde_json::json!({
                "type": "response_item",
                "payload": {
                    "type": "message",
                    "role": "user",
                    "content": [{
                        "type": "input_text",
                        "text": "盘古记忆必须从真实 Codex 历史会话读取，并自动写入高置信长期记忆。"
                    }]
                }
            })
        ),
    )
    .unwrap();
    let db_path = sqlite_dir.join("codex-dev.db");
    let db = Connection::open(&db_path).unwrap();
    db.execute(
        "CREATE TABLE threads (
            id TEXT PRIMARY KEY,
            title TEXT,
            cwd TEXT,
            rollout_path TEXT,
            updated_at_ms INTEGER
        )",
        [],
    )
    .unwrap();
    db.execute(
        "INSERT INTO threads (id, title, cwd, rollout_path, updated_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        (
            "thread-session",
            "history",
            "repo-session",
            rollout_path.to_string_lossy().to_string(),
            1000_i64,
        ),
    )
    .unwrap();
    drop(db);

    let summary = with_temporary_codex_home(&codex_home, || {
        store.session_summary(MemorySessionRequest {
            workspace: "repo-session".into(),
            query: "盘古记忆 历史会话".into(),
            max_items: 5,
        })
    })
    .unwrap();

    assert_eq!(summary.total_items, 1);
    assert_eq!(summary.injected_items.len(), 1);
    assert!(summary.summary.contains("本次启动从历史会话自动学习 1 条"));
    assert!(std::path::Path::new(&summary.inject_summary_cache_path).exists());
}

#[test]
fn export_import_and_selfcheck_create_recoverable_state() {
    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("memory.sqlite");
    let store = store_at(&db_path);
    store
        .learn_item(MemoryItemRequest {
            text: "导出导入需要保留工作区".into(),
            workspace: "repo-a".into(),
            category: "backup".into(),
            tags: vec!["export".into()],
            source: "test".into(),
            source_session_id: "s1".into(),
        })
        .unwrap();

    let export = store.export_json().unwrap();
    assert_eq!(export.items.len(), 1);

    let imported_store = store_at(&temp.path().join("imported.sqlite"));
    imported_store
        .import_json(MemoryImportRequest {
            data: export,
            replace_existing: true,
        })
        .unwrap();
    assert_eq!(
        with_codex_home_env(|| imported_store.status())
            .unwrap()
            .total_items,
        1
    );

    let report = imported_store
        .run_selfcheck(MemorySelfCheckRequest { repair: true })
        .unwrap();
    assert_eq!(report.status, "ok");
    assert!(report.backup_path.is_some());
    assert!(report.checks.iter().any(|check| check.name == "schema"));
}

#[test]
fn selfcheck_repair_backups_do_not_overwrite_within_same_second() {
    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("memory.sqlite");
    let store = store_at(&db_path);
    store
        .learn_item(MemoryItemRequest {
            text: "备份测试第一条".into(),
            workspace: "repo-a".into(),
            category: "backup".into(),
            tags: vec![],
            source: "test".into(),
            source_session_id: "s1".into(),
        })
        .unwrap();
    let first = store
        .run_selfcheck(MemorySelfCheckRequest { repair: true })
        .unwrap()
        .backup_path
        .unwrap();
    store
        .learn_item(MemoryItemRequest {
            text: "备份测试第二条".into(),
            workspace: "repo-a".into(),
            category: "backup".into(),
            tags: vec![],
            source: "test".into(),
            source_session_id: "s2".into(),
        })
        .unwrap();
    let second = store
        .run_selfcheck(MemorySelfCheckRequest { repair: true })
        .unwrap()
        .backup_path
        .unwrap();

    assert_ne!(first, second);
    assert!(std::path::Path::new(&first).exists());
    assert!(std::path::Path::new(&second).exists());
}

#[test]
fn import_redacts_secret_values_before_storage_and_search() {
    let temp = tempfile::tempdir().unwrap();
    let source = store_at(&temp.path().join("source.sqlite"));
    source
        .learn_item(MemoryItemRequest {
            text: "API key 是 sk-secret123，Authorization: Bearer abc.def.ghi".into(),
            workspace: "repo-a".into(),
            category: "secret-test".into(),
            tags: vec![],
            source: "test".into(),
            source_session_id: "s1".into(),
        })
        .unwrap();

    let mut export = source.export_json().unwrap();
    export.items[0].text =
        "导入包里包含 sk-import-secret 和 Authorization: Bearer imported.token".into();
    export.items[0].source_session_id = "https://example.test/?token=sk-session-secret".into();
    export.candidates.push(
        source
            .create_candidate(MemoryCandidateRequest {
                text: "候选记忆里包含 sk-candidate-secret".into(),
                workspace: "repo-a".into(),
                category: "secret-test".into(),
                tags: vec![],
                source: "test".into(),
                reason: "because Bearer candidate.reason.token".into(),
                source_session_id: "session sk-candidate-session".into(),
            })
            .unwrap(),
    );
    export.candidates[0].text = "导入候选包含 Authorization: Bearer imported.candidate".into();

    let imported_path = temp.path().join("imported.sqlite");
    let imported = store_at(&imported_path);
    imported
        .import_json(MemoryImportRequest {
            data: export,
            replace_existing: true,
        })
        .unwrap();

    let leaked = imported
        .query(MemoryQueryRequest {
            query: "import-secret imported.token".into(),
            workspace: "repo-a".into(),
            include_global: true,
            include_archived: false,
            limit: 5,
        })
        .unwrap();
    assert!(
        leaked.results.is_empty(),
        "secret tokens must not remain searchable after import"
    );

    let redacted = imported
        .query(MemoryQueryRequest {
            query: "导入包".into(),
            workspace: "repo-a".into(),
            include_global: true,
            include_archived: false,
            limit: 5,
        })
        .unwrap();
    assert_eq!(redacted.results.len(), 1);
    assert!(redacted.results[0].item.text.contains("sk-***"));
    assert!(redacted.results[0].item.text.contains("Bearer ***"));

    let conn = Connection::open(imported_path).unwrap();
    let mut raw = String::new();
    for table in ["memory_items", "memory_candidates"] {
        let mut stmt = conn
            .prepare(&format!("SELECT * FROM {table}"))
            .expect("prepare raw memory scan");
        let rows = stmt
            .query_map([], |row| {
                let mut values = Vec::new();
                for idx in 0..row.as_ref().column_count() {
                    values.push(row.get::<_, String>(idx).unwrap_or_default());
                }
                Ok(values.join(" "))
            })
            .unwrap();
        for row in rows {
            raw.push_str(&row.unwrap());
            raw.push('\n');
        }
    }
    for leaked in [
        "sk-import-secret",
        "imported.token",
        "sk-session-secret",
        "sk-candidate-secret",
        "imported.candidate",
        "candidate.reason.token",
        "sk-candidate-session",
    ] {
        assert!(
            !raw.contains(leaked),
            "raw SQLite contents must not contain secret fragment: {leaked}"
        );
    }
}
