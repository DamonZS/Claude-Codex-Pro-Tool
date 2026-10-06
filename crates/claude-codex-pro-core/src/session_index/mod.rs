// Session Database Indexing
//
// Architecture:
// - JSONL files remain the source of truth (append-only, immutable)
// - SQLite provides fast indexed queries (sessions, messages, projects)
// - Async background service monitors file changes and updates index
// - UI reads from database, exports read from JSONL

mod schema;
mod indexer;
mod query;

#[cfg(test)]
mod tests;

pub use schema::{SessionIndex, MessageIndex, ProjectStats, IndexMetadata};
pub use indexer::{SessionIndexer, IndexerConfig};
pub use query::{SessionQuery, SessionFilter, Pagination, PagedResult};

use anyhow::Result;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// Session index database manager
pub struct SessionIndexDb {
    db_path: PathBuf,
}

impl SessionIndexDb {
    /// Create or open session index database
    pub fn new(db_path: PathBuf) -> Result<Self> {
        Ok(Self { db_path })
    }

    /// Get database connection
    pub fn connection(&self) -> Result<Connection> {
        let conn = Connection::open(&self.db_path)?;
        // Enable WAL mode for better concurrent reads
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        Ok(conn)
    }

    /// Initialize database schema (idempotent)
    pub fn init_schema(&self) -> Result<()> {
        let conn = self.connection()?;
        schema::create_tables(&conn)?;
        Ok(())
    }

    /// Get database path
    pub fn path(&self) -> &Path {
        &self.db_path
    }
}
