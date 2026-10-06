// Session indexer - scans JSONL files and updates database

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use serde_json::Value;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Indexer configuration
#[derive(Debug, Clone)]
pub struct IndexerConfig {
    pub claude_sessions_dir: PathBuf,  // ~/.claude/projects/
    pub codex_sessions_dir: PathBuf,   // ~/.codex/sessions/
    pub batch_size: usize,              // Commit every N messages
}

impl Default for IndexerConfig {
    fn default() -> Self {
        let home = directories::BaseDirs::new()
            .expect("Failed to get home directory")
            .home_dir()
            .to_path_buf();

        Self {
            claude_sessions_dir: home.join(".claude/projects"),
            codex_sessions_dir: home.join(".codex/sessions"),
            batch_size: 1000,
        }
    }
}

/// Session indexer
pub struct SessionIndexer {
    config: IndexerConfig,
}

impl SessionIndexer {
    pub fn new(config: IndexerConfig) -> Self {
        Self { config }
    }

    /// Perform initial full scan
    pub fn initial_scan(&self, conn: &mut Connection) -> Result<IndexStats> {
        let mut stats = IndexStats::default();

        // Scan Claude sessions
        if self.config.claude_sessions_dir.exists() {
            self.scan_directory(conn, &self.config.claude_sessions_dir, "claude", &mut stats)?;
        }

        // Scan Codex sessions
        if self.config.codex_sessions_dir.exists() {
            self.scan_directory(conn, &self.config.codex_sessions_dir, "codex", &mut stats)?;
        }

        // Rebuild project aggregations
        self.rebuild_project_stats(conn)?;

        Ok(stats)
    }

    /// Scan a directory recursively for JSONL files
    fn scan_directory(
        &self,
        conn: &mut Connection,
        dir: &Path,
        source: &str,
        stats: &mut IndexStats,
    ) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                self.scan_directory(conn, &path, source, stats)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                match self.index_session_file(conn, &path, source) {
                    Ok(file_stats) => {
                        stats.files_indexed += 1;
                        stats.sessions_indexed += 1;
                        stats.messages_indexed += file_stats.messages_indexed;
                    }
                    Err(e) => {
                        eprintln!("Failed to index {:?}: {}", path, e);
                        stats.files_failed += 1;
                    }
                }
            }
        }

        Ok(())
    }

    /// Index a single JSONL session file
    fn index_session_file(
        &self,
        conn: &mut Connection,
        path: &Path,
        source: &str,
    ) -> Result<IndexStats> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        // Get file modification time
        let metadata = fs::metadata(path)?;
        let file_mtime = metadata
            .modified()?
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_millis() as i64;

        // Read first line to get session metadata
        let first_line = lines.next()
            .context("Empty JSONL file")?
            .context("Failed to read first line")?;

        let first_msg: Value = serde_json::from_str(&first_line)?;

        // Extract session_id from filename (without .jsonl extension)
        let session_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .context("Invalid filename")?
            .to_string();

        // Extract session metadata
        let project = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let title = first_msg["content"]
            .as_str()
            .unwrap_or("Untitled Session")
            .chars()
            .take(100)
            .collect::<String>();

        let created_at = first_msg["ts"]
            .as_i64()
            .unwrap_or(0);

        // Check if session already indexed and up-to-date
        let existing: Option<(i64, i64)> = conn
            .query_row(
                "SELECT file_mtime, indexed_at FROM sessions WHERE session_id = ?",
                params![session_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();

        if let Some((existing_mtime, _)) = existing {
            if existing_mtime >= file_mtime {
                // Already up-to-date
                return Ok(IndexStats::default());
            }
        }

        // Start transaction
        let tx = conn.transaction()?;

        let indexed_at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_millis() as i64;

        // Step 1: Ensure project exists first (to satisfy foreign key if we add it later)
        let source_mask = if source == "claude" { 1 } else { 2 };
        tx.execute(
            "INSERT OR IGNORE INTO projects \
             (project_name, session_count, message_count, last_activity, source_mask) \
             VALUES (?, 0, 0, 0, ?)",
            params![project, source_mask],
        )?;

        // Step 2: Insert or update session (must exist before messages due to FK)
        tx.execute(
            "INSERT OR REPLACE INTO sessions \
             (session_id, project, title, source, created_at, updated_at, \
              message_count, file_path, file_mtime, indexed_at) \
             VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?, ?)",
            params![
                session_id,
                project,
                title,
                source,
                created_at,
                created_at, // Will update later
                path.to_string_lossy().to_string(),
                file_mtime,
                indexed_at,
            ],
        )?;

        // Step 3: Delete existing messages if re-indexing
        tx.execute("DELETE FROM messages WHERE session_id = ?", params![session_id])?;

        // Step 4: Index all messages
        let mut message_count = 0;
        let mut line_number = 1;

        // Insert first message
        self.insert_message(&tx, &first_msg, &session_id, line_number)?;
        message_count += 1;
        line_number += 1;

        // Process remaining messages
        for line_result in lines {
            let line = line_result?;
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<Value>(&line) {
                Ok(msg) => {
                    self.insert_message(&tx, &msg, &session_id, line_number)?;
                    message_count += 1;
                }
                Err(e) => {
                    eprintln!("Failed to parse line {}: {}", line_number, e);
                }
            }

            line_number += 1;

            // Periodic commit for large files
            if message_count % self.config.batch_size == 0 {
                tx.execute("SAVEPOINT batch", [])?;
            }
        }

        // Step 5: Update session with final message_count and updated_at
        let updated_at = tx.query_row(
            "SELECT MAX(created_at) FROM messages WHERE session_id = ?",
            params![session_id],
            |row| row.get::<_, Option<i64>>(0),
        )?.unwrap_or(created_at);

        tx.execute(
            "UPDATE sessions SET message_count = ?, updated_at = ? WHERE session_id = ?",
            params![message_count, updated_at, session_id],
        )?;

        tx.commit()?;

        Ok(IndexStats {
            files_indexed: 1,
            sessions_indexed: 1,
            messages_indexed: message_count,
            files_failed: 0,
        })
    }

    /// Insert a message into the index
    fn insert_message(
        &self,
        conn: &Connection,
        msg: &Value,
        session_id: &str,
        line_number: i32,
    ) -> Result<()> {
        let message_id = msg["uuid"]
            .as_str()
            .unwrap_or(&format!("msg-{}", line_number))
            .to_string();

        let role = msg["role"]
            .as_str()
            .unwrap_or("user")
            .to_string();

        let created_at = msg["ts"]
            .as_i64()
            .unwrap_or(0);

        let content = msg["content"]
            .as_str()
            .unwrap_or("");

        let content_preview = content
            .chars()
            .take(200)
            .collect::<String>();

        // Rough token estimate (4 chars ≈ 1 token)
        let token_estimate = (content.len() / 4) as i32;

        conn.execute(
            "INSERT INTO messages \
             (message_id, session_id, role, created_at, content_preview, token_estimate, line_number) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                message_id,
                session_id,
                role,
                created_at,
                Some(content_preview),
                token_estimate,
                line_number,
            ],
        )?;

        Ok(())
    }

    /// Rebuild project aggregation statistics
    fn rebuild_project_stats(&self, conn: &Connection) -> Result<()> {
        conn.execute("DELETE FROM projects", [])?;

        conn.execute(
            r#"
            INSERT INTO projects (project_name, session_count, message_count, last_activity, source_mask)
            SELECT
                project,
                COUNT(*) as session_count,
                SUM(message_count) as message_count,
                MAX(updated_at) as last_activity,
                SUM(CASE source WHEN 'claude' THEN 1 WHEN 'codex' THEN 2 ELSE 0 END) as source_mask
            FROM sessions
            GROUP BY project
            "#,
            [],
        )?;

        Ok(())
    }

    /// Index a specific file (for incremental updates)
    pub fn index_file(&self, conn: &mut Connection, path: &Path, source: &str) -> Result<IndexStats> {
        self.index_session_file(conn, path, source)
    }

    /// Remove a session from the index (when file is deleted)
    pub fn remove_session(&self, conn: &Connection, session_id: &str) -> Result<()> {
        conn.execute("DELETE FROM sessions WHERE session_id = ?", params![session_id])?;
        // Messages are cascade deleted
        Ok(())
    }
}

/// Indexing statistics
#[derive(Debug, Clone, Default)]
pub struct IndexStats {
    pub files_indexed: usize,
    pub sessions_indexed: usize,
    pub messages_indexed: usize,
    pub files_failed: usize,
}
