#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_index::{SessionIndexDb, SessionIndexer, IndexerConfig, SessionQuery, SessionFilter, Pagination};
    use rusqlite::Connection;
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    fn create_test_db() -> (TempDir, SessionIndexDb) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = SessionIndexDb::new(db_path).unwrap();
        db.init_schema().unwrap();
        (temp_dir, db)
    }

    fn create_test_jsonl(dir: &Path, project: &str, session_id: &str) -> PathBuf {
        let project_dir = dir.join(project);
        fs::create_dir_all(&project_dir).unwrap();
        let file_path = project_dir.join(format!("{}.jsonl", session_id));

        let mut file = fs::File::create(&file_path).unwrap();

        // First message (defines session) - use unique message IDs
        writeln!(
            file,
            r#"{{"uuid":"msg-{}-1","role":"user","content":"Test session","ts":1704067200000}}"#,
            session_id
        ).unwrap();

        // Second message
        writeln!(
            file,
            r#"{{"uuid":"msg-{}-2","role":"assistant","content":"Test response","ts":1704067260000}}"#,
            session_id
        ).unwrap();

        file_path
    }

    #[test]
    fn test_schema_creation() {
        let (_temp_dir, db) = create_test_db();
        let conn = db.connection().unwrap();

        // Check that tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();

        assert!(tables.contains(&"sessions".to_string()));
        assert!(tables.contains(&"messages".to_string()));
        assert!(tables.contains(&"projects".to_string()));
        assert!(tables.contains(&"index_metadata".to_string()));
    }

    #[test]
    fn test_index_single_session() {
        let (_temp_dir, db) = create_test_db();
        let temp_sessions = TempDir::new().unwrap();

        let session_id = "test-session-123";
        let file_path = create_test_jsonl(temp_sessions.path(), "test-project", session_id);

        let config = IndexerConfig {
            claude_sessions_dir: temp_sessions.path().to_path_buf(),
            codex_sessions_dir: temp_sessions.path().to_path_buf(),
            batch_size: 100,
        };

        let indexer = SessionIndexer::new(config);
        let mut conn = db.connection().unwrap();

        let stats = indexer
            .index_file(&mut conn, &file_path, "test")
            .unwrap();

        assert_eq!(stats.sessions_indexed, 1);
        assert_eq!(stats.messages_indexed, 2);

        // Verify session was indexed
        let query = SessionQuery::new(&conn);
        let session = query.get_session(session_id).unwrap();

        assert!(session.is_some());
        let session = session.unwrap();
        assert_eq!(session.session_id, session_id);
        assert_eq!(session.project, "test-project");
        assert_eq!(session.message_count, 2);
    }

    #[test]
    fn test_query_sessions_with_filter() {
        let (_temp_dir, db) = create_test_db();
        let temp_sessions = TempDir::new().unwrap();

        // Create multiple sessions
        create_test_jsonl(temp_sessions.path(), "project-a", "session-1");
        create_test_jsonl(temp_sessions.path(), "project-a", "session-2");
        create_test_jsonl(temp_sessions.path(), "project-b", "session-3");

        let config = IndexerConfig {
            claude_sessions_dir: temp_sessions.path().to_path_buf(),
            codex_sessions_dir: temp_sessions.path().to_path_buf(),
            batch_size: 100,
        };

        let indexer = SessionIndexer::new(config);
        let mut conn = db.connection().unwrap();

        // Index all sessions
        indexer.initial_scan(&mut conn).unwrap();

        // Query with project filter
        let query = SessionQuery::new(&conn);
        let filter = SessionFilter {
            project: Some("project-a".to_string()),
            ..Default::default()
        };
        let pagination = Pagination {
            page: 0,
            page_size: 10,
        };

        let result = query.query_sessions(&filter, &pagination).unwrap();

        assert_eq!(result.total, 2);
        assert_eq!(result.items.len(), 2);
        assert!(result.items.iter().all(|s| s.project == "project-a"));
    }

    #[test]
    fn test_project_stats() {
        let (_temp_dir, db) = create_test_db();
        let temp_sessions = TempDir::new().unwrap();

        // Create sessions in different projects
        create_test_jsonl(temp_sessions.path(), "project-a", "session-1");
        create_test_jsonl(temp_sessions.path(), "project-a", "session-2");
        create_test_jsonl(temp_sessions.path(), "project-b", "session-3");

        let config = IndexerConfig {
            claude_sessions_dir: temp_sessions.path().to_path_buf(),
            codex_sessions_dir: temp_sessions.path().to_path_buf(),
            batch_size: 100,
        };

        let indexer = SessionIndexer::new(config);
        let mut conn = db.connection().unwrap();

        indexer.initial_scan(&mut conn).unwrap();

        let query = SessionQuery::new(&conn);
        let stats = query.get_project_stats().unwrap();

        assert_eq!(stats.len(), 2);

        let project_a = stats.iter().find(|s| s.project_name == "project-a").unwrap();
        assert_eq!(project_a.session_count, 2);
        assert_eq!(project_a.message_count, 4); // 2 messages per session

        let project_b = stats.iter().find(|s| s.project_name == "project-b").unwrap();
        assert_eq!(project_b.session_count, 1);
        assert_eq!(project_b.message_count, 2);
    }

    #[test]
    fn test_pagination() {
        let (_temp_dir, db) = create_test_db();
        let temp_sessions = TempDir::new().unwrap();

        // Create 5 sessions
        for i in 1..=5 {
            create_test_jsonl(
                temp_sessions.path(),
                "test-project",
                &format!("session-{}", i),
            );
        }

        let config = IndexerConfig {
            claude_sessions_dir: temp_sessions.path().to_path_buf(),
            codex_sessions_dir: temp_sessions.path().to_path_buf(),
            batch_size: 100,
        };

        let indexer = SessionIndexer::new(config);
        let mut conn = db.connection().unwrap();

        indexer.initial_scan(&mut conn).unwrap();

        let query = SessionQuery::new(&conn);

        // Page 1: 2 items
        let page1 = query
            .query_sessions(&SessionFilter::default(), &Pagination { page: 0, page_size: 2 })
            .unwrap();
        assert_eq!(page1.items.len(), 2);
        assert_eq!(page1.total, 5);
        assert_eq!(page1.total_pages, 3);

        // Page 2: 2 items
        let page2 = query
            .query_sessions(&SessionFilter::default(), &Pagination { page: 1, page_size: 2 })
            .unwrap();
        assert_eq!(page2.items.len(), 2);

        // Page 3: 1 item
        let page3 = query
            .query_sessions(&SessionFilter::default(), &Pagination { page: 2, page_size: 2 })
            .unwrap();
        assert_eq!(page3.items.len(), 1);
    }
}
