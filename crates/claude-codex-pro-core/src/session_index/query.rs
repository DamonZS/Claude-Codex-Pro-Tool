// Query interface for session index

use anyhow::Result;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use crate::session_index::schema::{SessionIndex, MessageIndex, ProjectStats};

/// Session filter criteria
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionFilter {
    pub project: Option<String>,
    pub source: Option<String>, // "claude" | "codex"
    pub time_range: Option<(i64, i64)>, // (start, end) Unix timestamps
    pub search_query: Option<String>, // Title search
}

/// Pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub page: usize,
    pub page_size: usize,
}

impl Default for Pagination {
    fn default() -> Self {
        Self { page: 0, page_size: 20 }
    }
}

/// Paginated query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagedResult<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

/// Session query interface
pub struct SessionQuery<'a> {
    conn: &'a Connection,
}

impl<'a> SessionQuery<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Query sessions with filters and pagination
    pub fn query_sessions(
        &self,
        filter: &SessionFilter,
        pagination: &Pagination,
    ) -> Result<PagedResult<SessionIndex>> {
        // Build WHERE clause
        let mut conditions = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref project) = filter.project {
            conditions.push("project = ?");
            params_vec.push(Box::new(project.clone()));
        }

        if let Some(ref source) = filter.source {
            conditions.push("source = ?");
            params_vec.push(Box::new(source.clone()));
        }

        if let Some((start, end)) = filter.time_range {
            conditions.push("updated_at BETWEEN ? AND ?");
            params_vec.push(Box::new(start));
            params_vec.push(Box::new(end));
        }

        if let Some(ref query) = filter.search_query {
            conditions.push("title LIKE ?");
            params_vec.push(Box::new(format!("%{}%", query)));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Count total
        let count_sql = format!("SELECT COUNT(*) FROM sessions {}", where_clause);
        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
        let total: usize = self.conn.query_row(&count_sql, params_refs.as_slice(), |row| row.get(0))?;

        // Query with pagination
        let offset = pagination.page * pagination.page_size;
        let query_sql = format!(
            "SELECT session_id, project, title, source, created_at, updated_at, \
             message_count, file_path, file_mtime, indexed_at \
             FROM sessions {} ORDER BY updated_at DESC LIMIT ? OFFSET ?",
            where_clause
        );

        let mut params_with_limit = params_vec;
        params_with_limit.push(Box::new(pagination.page_size as i64));
        params_with_limit.push(Box::new(offset as i64));
        let params_refs: Vec<&dyn rusqlite::ToSql> = params_with_limit.iter().map(|b| b.as_ref()).collect();

        let mut stmt = self.conn.prepare(&query_sql)?;
        let sessions = stmt.query_map(params_refs.as_slice(), |row| {
            Ok(SessionIndex {
                session_id: row.get(0)?,
                project: row.get(1)?,
                title: row.get(2)?,
                source: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                message_count: row.get(6)?,
                file_path: row.get(7)?,
                file_mtime: row.get(8)?,
                indexed_at: row.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        let total_pages = (total + pagination.page_size - 1) / pagination.page_size;

        Ok(PagedResult {
            items: sessions,
            total,
            page: pagination.page,
            page_size: pagination.page_size,
            total_pages,
        })
    }

    /// Get messages for a session
    pub fn get_session_messages(
        &self,
        session_id: &str,
        line_range: Option<(i32, i32)>,
    ) -> Result<Vec<MessageIndex>> {
        let (where_clause, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = match line_range {
            Some((start, end)) => (
                "WHERE session_id = ? AND line_number BETWEEN ? AND ?".to_string(),
                vec![Box::new(session_id.to_string()), Box::new(start), Box::new(end)],
            ),
            None => (
                "WHERE session_id = ?".to_string(),
                vec![Box::new(session_id.to_string())],
            ),
        };

        let sql = format!(
            "SELECT message_id, session_id, role, created_at, content_preview, \
             token_estimate, line_number FROM messages {} ORDER BY line_number",
            where_clause
        );

        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|b| b.as_ref()).collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let messages = stmt.query_map(params_refs.as_slice(), |row| {
            Ok(MessageIndex {
                message_id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                created_at: row.get(3)?,
                content_preview: row.get(4)?,
                token_estimate: row.get(5)?,
                line_number: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(messages)
    }

    /// Get project statistics
    pub fn get_project_stats(&self) -> Result<Vec<ProjectStats>> {
        let mut stmt = self.conn.prepare(
            "SELECT project_name, session_count, message_count, last_activity, source_mask \
             FROM projects ORDER BY last_activity DESC",
        )?;

        let stats = stmt.query_map([], |row| {
            Ok(ProjectStats {
                project_name: row.get(0)?,
                session_count: row.get(1)?,
                message_count: row.get(2)?,
                last_activity: row.get(3)?,
                source_mask: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(stats)
    }

    /// Get session by ID
    pub fn get_session(&self, session_id: &str) -> Result<Option<SessionIndex>> {
        let result = self.conn.query_row(
            "SELECT session_id, project, title, source, created_at, updated_at, \
             message_count, file_path, file_mtime, indexed_at \
             FROM sessions WHERE session_id = ?",
            params![session_id],
            |row| {
                Ok(SessionIndex {
                    session_id: row.get(0)?,
                    project: row.get(1)?,
                    title: row.get(2)?,
                    source: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    message_count: row.get(6)?,
                    file_path: row.get(7)?,
                    file_mtime: row.get(8)?,
                    indexed_at: row.get(9)?,
                })
            },
        );

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
