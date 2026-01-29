//! Database operations for clipboard history.

use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use wayclip_common::{ContentType, HistoryEntry};

use super::schema;

/// Database handle with connection pooling.
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    path: PathBuf,
}

impl Database {
    /// Open the database at the default path.
    pub fn open() -> Result<Self> {
        let path = wayclip_common::database_path();
        Self::open_at(path)
    }

    /// Open the database at a specific path.
    pub fn open_at(path: PathBuf) -> Result<Self> {
        let conn = Connection::open(&path)?;

        // Enable foreign keys
        conn.execute_batch("PRAGMA foreign_keys = ON")?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            path,
        })
    }

    /// Run database migrations.
    pub fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(schema::CREATE_ENTRIES_TABLE)?;
        conn.execute_batch(schema::CREATE_CONTENT_TABLE)?;
        conn.execute_batch(schema::CREATE_INDEXES)?;

        // FTS table creation might fail on older SQLite versions
        let _ = conn.execute_batch(schema::CREATE_FTS_TABLE);
        let _ = conn.execute_batch(schema::CREATE_FTS_TRIGGERS);

        Ok(())
    }

    /// Find an entry by its content hash.
    pub fn find_by_hash(&self, hash: &str) -> Result<Option<i64>> {
        let conn = self.conn.lock().unwrap();
        let id: Option<i64> = conn
            .query_row(
                "SELECT id FROM entries WHERE content_hash = ?1",
                params![hash],
                |row| row.get(0),
            )
            .optional()?;
        Ok(id)
    }

    /// Update last_used_at for an entry by hash.
    pub fn touch_by_hash(&self, hash: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "UPDATE entries SET last_used_at = ?1, use_count = use_count + 1 WHERE content_hash = ?2",
            params![now, hash],
        )?;
        Ok(())
    }

    /// Update last_used_at for an entry by ID.
    pub fn touch_entry(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "UPDATE entries SET last_used_at = ?1, use_count = use_count + 1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    /// Insert a new clipboard entry.
    pub fn insert_entry(
        &self,
        hash: &str,
        content_type: ContentType,
        mime_type: &str,
        preview: &str,
        content: &[u8],
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let content_type_str = match content_type {
            ContentType::Text => "text",
            ContentType::Image => "image",
        };

        conn.execute(
            "INSERT INTO entries (content_hash, content_type, mime_type, preview, byte_size, created_at, last_used_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![hash, content_type_str, mime_type, preview, content.len() as i64, now],
        )?;

        let id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO content (entry_id, data) VALUES (?1, ?2)",
            params![id, content],
        )?;

        Ok(id)
    }

    /// Get clipboard history.
    pub fn get_history(
        &self,
        limit: Option<u32>,
        offset: Option<u32>,
        search: Option<&str>,
    ) -> Result<(Vec<HistoryEntry>, u64)> {
        let conn = self.conn.lock().unwrap();
        let limit = limit.unwrap_or(100) as i64;
        let offset = offset.unwrap_or(0) as i64;

        let (entries, total) = if let Some(search) = search {
            // Use FTS search
            let search_query = format!("{}*", search.replace('"', "\"\""));

            let total: i64 = conn.query_row(
                "SELECT COUNT(*) FROM entries_fts WHERE entries_fts MATCH ?1",
                params![search_query],
                |row| row.get(0),
            ).unwrap_or(0);

            let mut stmt = conn.prepare(
                "SELECT e.id, e.content_type, e.mime_type, e.preview, e.byte_size, e.created_at, e.pinned
                 FROM entries e
                 INNER JOIN entries_fts fts ON e.id = fts.rowid
                 WHERE entries_fts MATCH ?1
                 ORDER BY e.created_at DESC
                 LIMIT ?2 OFFSET ?3"
            )?;

            let entries: Vec<HistoryEntry> = stmt
                .query_map(params![search_query, limit, offset], |row| {
                    Ok(row_to_entry(row))
                })?
                .filter_map(|r| r.ok())
                .collect();

            (entries, total as u64)
        } else {
            let total: i64 = conn.query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))?;

            let mut stmt = conn.prepare(
                "SELECT id, content_type, mime_type, preview, byte_size, created_at, pinned
                 FROM entries
                 ORDER BY created_at DESC
                 LIMIT ?1 OFFSET ?2",
            )?;

            let entries: Vec<HistoryEntry> = stmt
                .query_map(params![limit, offset], |row| Ok(row_to_entry(row)))?
                .filter_map(|r| r.ok())
                .collect();

            (entries, total as u64)
        };

        Ok((entries, total))
    }

    /// Get the content of an entry.
    pub fn get_content(&self, id: i64) -> Result<Option<(String, Vec<u8>)>> {
        let conn = self.conn.lock().unwrap();

        let result: Option<(String, Vec<u8>)> = conn
            .query_row(
                "SELECT e.mime_type, c.data
                 FROM entries e
                 INNER JOIN content c ON e.id = c.entry_id
                 WHERE e.id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        Ok(result)
    }

    /// Delete an entry.
    pub fn delete_entry(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();

        // Content is deleted automatically via CASCADE
        let rows = conn.execute("DELETE FROM entries WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }

    /// Clear all non-pinned entries.
    pub fn clear_unpinned(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM entries WHERE pinned = 0", [])?;
        Ok(())
    }

    /// Set pinned status.
    pub fn set_pinned(&self, id: i64, pinned: bool) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "UPDATE entries SET pinned = ?1 WHERE id = ?2",
            params![pinned as i32, id],
        )?;
        Ok(rows > 0)
    }

    /// Count total entries.
    pub fn count_entries(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))?;
        Ok(count as u64)
    }

    /// Get database size in bytes.
    pub fn database_size(&self) -> Result<u64> {
        let metadata = std::fs::metadata(&self.path)?;
        Ok(metadata.len())
    }

    /// Cleanup old entries to stay within max_entries limit.
    pub fn cleanup(&self, max_entries: u32) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Count non-pinned entries
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM entries WHERE pinned = 0",
            [],
            |row| row.get(0),
        )?;

        if count > max_entries as i64 {
            let to_delete = count - max_entries as i64;

            conn.execute(
                "DELETE FROM entries WHERE id IN (
                    SELECT id FROM entries WHERE pinned = 0
                    ORDER BY last_used_at ASC
                    LIMIT ?1
                )",
                params![to_delete],
            )?;

            tracing::debug!("Cleaned up {} old entries", to_delete);
        }

        Ok(())
    }
}

fn row_to_entry(row: &rusqlite::Row) -> HistoryEntry {
    let content_type_str: String = row.get(1).unwrap_or_default();
    let content_type = match content_type_str.as_str() {
        "image" => ContentType::Image,
        _ => ContentType::Text,
    };

    HistoryEntry {
        id: row.get(0).unwrap_or(0),
        content_type,
        mime_type: row.get(2).unwrap_or_default(),
        preview: row.get(3).unwrap_or_default(),
        byte_size: row.get::<_, i64>(4).unwrap_or(0) as u64,
        created_at: row.get(5).unwrap_or(0),
        pinned: row.get::<_, i32>(6).unwrap_or(0) != 0,
        thumbnail: None,
    }
}
