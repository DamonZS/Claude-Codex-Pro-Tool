// Session index commands for Tauri frontend

use claude_codex_pro_core::session_index::{
    SessionIndexDb, SessionIndexer, IndexerConfig,
    SessionQuery, SessionFilter, Pagination, PagedResult,
    SessionIndex, MessageIndex, ProjectStats,
};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

/// Global session index database
pub struct SessionIndexState {
    db: Mutex<Option<SessionIndexDb>>,
}

impl SessionIndexState {
    pub fn new() -> Self {
        Self {
            db: Mutex::new(None),
        }
    }
}

/// Initialize session index database
#[tauri::command]
pub async fn init_session_index(state: State<'_, SessionIndexState>) -> Result<String, String> {
    let db_path = directories::BaseDirs::new()
        .ok_or("Failed to get home directory")?
        .data_dir()
        .join("claude-codex-pro")
        .join("session_index.db");

    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let db = SessionIndexDb::new(db_path.clone()).map_err(|e| e.to_string())?;
    db.init_schema().map_err(|e| e.to_string())?;

    *state.db.lock().unwrap() = Some(db);

    Ok(format!("Session index initialized at {:?}", db_path))
}

/// Perform initial scan of all sessions
#[tauri::command]
pub async fn scan_sessions(state: State<'_, SessionIndexState>) -> Result<ScanResult, String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let indexer = SessionIndexer::new(IndexerConfig::default());
    let mut conn = db.connection().map_err(|e| e.to_string())?;

    let stats = indexer
        .initial_scan(&mut conn)
        .map_err(|e| e.to_string())?;

    Ok(ScanResult {
        files_indexed: stats.files_indexed,
        sessions_indexed: stats.sessions_indexed,
        messages_indexed: stats.messages_indexed,
        files_failed: stats.files_failed,
    })
}

/// Query sessions with filters
#[tauri::command]
pub async fn query_sessions(
    state: State<'_, SessionIndexState>,
    filter: SessionFilter,
    page: usize,
    page_size: usize,
) -> Result<PagedResult<SessionIndex>, String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let conn = db.connection().map_err(|e| e.to_string())?;
    let query = SessionQuery::new(&conn);

    let pagination = Pagination { page, page_size };
    query
        .query_sessions(&filter, &pagination)
        .map_err(|e| e.to_string())
}

/// Get messages for a session
#[tauri::command]
pub async fn get_session_messages(
    state: State<'_, SessionIndexState>,
    session_id: String,
) -> Result<Vec<MessageIndex>, String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let conn = db.connection().map_err(|e| e.to_string())?;
    let query = SessionQuery::new(&conn);

    query
        .get_session_messages(&session_id, None)
        .map_err(|e| e.to_string())
}

/// Get project statistics
#[tauri::command]
pub async fn get_project_stats(
    state: State<'_, SessionIndexState>,
) -> Result<Vec<ProjectStats>, String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let conn = db.connection().map_err(|e| e.to_string())?;
    let query = SessionQuery::new(&conn);

    query.get_project_stats().map_err(|e| e.to_string())
}

/// Get a single session by ID
#[tauri::command]
pub async fn get_session(
    state: State<'_, SessionIndexState>,
    session_id: String,
) -> Result<Option<SessionIndex>, String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let conn = db.connection().map_err(|e| e.to_string())?;
    let query = SessionQuery::new(&conn);

    query.get_session(&session_id).map_err(|e| e.to_string())
}

/// Index a specific session file (for incremental updates)
#[tauri::command]
pub async fn index_session_file(
    state: State<'_, SessionIndexState>,
    file_path: String,
    source: String,
) -> Result<ScanResult, String> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let indexer = SessionIndexer::new(IndexerConfig::default());
    let mut conn = db.connection().map_err(|e| e.to_string())?;

    let path = PathBuf::from(file_path);
    let stats = indexer
        .index_file(&mut conn, &path, &source)
        .map_err(|e| e.to_string())?;

    Ok(ScanResult {
        files_indexed: stats.files_indexed,
        sessions_indexed: stats.sessions_indexed,
        messages_indexed: stats.messages_indexed,
        files_failed: stats.files_failed,
    })
}

/// Scan result (serializable)
#[derive(Debug, serde::Serialize)]
pub struct ScanResult {
    pub files_indexed: usize,
    pub sessions_indexed: usize,
    pub messages_indexed: usize,
    pub files_failed: usize,
}
