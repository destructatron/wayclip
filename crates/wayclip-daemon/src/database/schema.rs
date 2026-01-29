//! Database schema definitions.

/// SQL to create the entries table.
pub const CREATE_ENTRIES_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content_hash TEXT NOT NULL UNIQUE,
    content_type TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    preview TEXT,
    byte_size INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    last_used_at INTEGER NOT NULL,
    use_count INTEGER DEFAULT 1,
    pinned INTEGER DEFAULT 0
)
"#;

/// SQL to create the content table (separate for BLOB efficiency).
pub const CREATE_CONTENT_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS content (
    entry_id INTEGER PRIMARY KEY,
    data BLOB NOT NULL,
    FOREIGN KEY (entry_id) REFERENCES entries(id) ON DELETE CASCADE
)
"#;

/// SQL to create indexes.
pub const CREATE_INDEXES: &str = r#"
CREATE INDEX IF NOT EXISTS idx_entries_created_at ON entries(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_entries_content_hash ON entries(content_hash);
CREATE INDEX IF NOT EXISTS idx_entries_pinned ON entries(pinned)
"#;

/// SQL to create FTS table for text search.
pub const CREATE_FTS_TABLE: &str = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS entries_fts USING fts5(
    preview,
    content='entries',
    content_rowid='id'
)
"#;

/// SQL to create FTS triggers.
pub const CREATE_FTS_TRIGGERS: &str = r#"
CREATE TRIGGER IF NOT EXISTS entries_fts_insert AFTER INSERT ON entries BEGIN
    INSERT INTO entries_fts(rowid, preview) VALUES (new.id, new.preview);
END;

CREATE TRIGGER IF NOT EXISTS entries_fts_delete AFTER DELETE ON entries BEGIN
    INSERT INTO entries_fts(entries_fts, rowid, preview) VALUES('delete', old.id, old.preview);
END;

CREATE TRIGGER IF NOT EXISTS entries_fts_update AFTER UPDATE ON entries BEGIN
    INSERT INTO entries_fts(entries_fts, rowid, preview) VALUES('delete', old.id, old.preview);
    INSERT INTO entries_fts(rowid, preview) VALUES (new.id, new.preview);
END
"#;
