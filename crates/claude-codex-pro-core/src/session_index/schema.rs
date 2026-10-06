// Database schema for session indexing

use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// Session metadata index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionIndex {
    pub session_id: String,
    pub project: String,
    pub title: String,
    pub source: String, // "claude" | "codex"
    pub created_at: i64, // Unix timestamp (ms)
    pub updated_at: i64,
    pub message_count: i32,
    pub file_path: String,
    pub file_mtime: i64,
    pub indexed_at: i64,
}

/// Message index (lightweight, for quick navigation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageIndex {
    pub message_id: String,
    pub session_id: String,
    pub role: String, // "user" | "assistant"
    pub created_at: i64,
    pub content_preview: Option<String>, // First 200 chars
    pub token_estimate: i32,
    pub line_number: i32, // Line number in JSONL file
}

/// Project aggregation stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStats {
    pub project_name: String,
    pub session_count: i32,
    pub message_count: i32,
    pub last_activity: i64,
    pub source_mask: i32, // Bitmask: 1=claude, 2=codex, 3=both
}

/// Index metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    pub key: String,
    pub value: String,
}

/// Create all database tables (idempotent)
pub fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- Session metadata
        CREATE TABLE IF NOT EXISTS sessions (
            session_id TEXT PRIMARY KEY,
            project TEXT NOT NULL,
            title TEXT NOT NULL,
            source TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            message_count INTEGER DEFAULT 0,
            file_path TEXT NOT NULL,
            file_mtime INTEGER NOT NULL,
            indexed_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project);
        CREATE INDEX IF NOT EXISTS idx_sessions_source ON sessions(source);
        CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at DESC);

        -- Message index
        CREATE TABLE IF NOT EXISTS messages (
            message_id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            content_preview TEXT,
            token_estimate INTEGER DEFAULT 0,
            line_number INTEGER NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, line_number);
        CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at);

        -- Project aggregation
        CREATE TABLE IF NOT EXISTS projects (
            project_name TEXT PRIMARY KEY,
            session_count INTEGER DEFAULT 0,
            message_count INTEGER DEFAULT 0,
            last_activity INTEGER NOT NULL,
            source_mask INTEGER DEFAULT 0
        );

        CREATE INDEX IF NOT EXISTS idx_projects_last_activity ON projects(last_activity DESC);

        -- Index metadata
        CREATE TABLE IF NOT EXISTS index_metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
    )?;

    // Set schema version
    conn.execute(
        "INSERT OR REPLACE INTO index_metadata (key, value) VALUES ('schema_version', '1')",
        [],
    )?;

    Ok(())
}

/// Get schema version
pub fn get_schema_version(conn: &Connection) -> Result<i32> {
    let version: String = conn.query_row(
        "SELECT value FROM index_metadata WHERE key = 'schema_version'",
        [],
        |row| row.get(0),
    )?;
    Ok(version.parse()?)
}
