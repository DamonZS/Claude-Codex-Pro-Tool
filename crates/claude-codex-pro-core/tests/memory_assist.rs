use claude_codex_pro_core::memory_assist::{
    MemoryAssistStore, MemoryCandidateRequest, MemoryImportRequest, MemoryItemRequest,
    MemoryQueryRequest, MemorySelfCheckRequest, MemorySessionRequest,
};
use rusqlite::Connection;

fn store_at(path: &std::path::Path) -> MemoryAssistStore {
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
    assert_eq!(store.status().unwrap().total_items, 3);
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
            limit: 20,
        })
        .unwrap();
    assert_eq!(listed[0].access_count, 0);

    let queried = store
        .query(MemoryQueryRequest {
            query: "列表刷新".into(),
            workspace: "repo-a".into(),
            include_global: true,
            limit: 20,
        })
        .unwrap();
    assert_eq!(queried.results.len(), 1);

    let listed_after_query = store
        .list_items(MemoryQueryRequest {
            query: String::new(),
            workspace: "repo-a".into(),
            include_global: true,
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

    let summary = store
        .session_summary(MemorySessionRequest {
            workspace: "repo-a".into(),
            query: "插件中心会话修复".into(),
            max_items: 3,
        })
        .unwrap();

    assert_eq!(summary.injected_items.len(), 3);
    assert_eq!(summary.workspace, "repo-a");
    assert_eq!(summary.total_items, 8);
    assert!(summary.summary.contains("repo-a"));
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
    assert_eq!(imported_store.status().unwrap().total_items, 1);

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
