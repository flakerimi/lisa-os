//! SQLite storage for Notes: one table, soft deletes so the bus's undo
//! journal can compensate (`delete_note` ↔ `restore_note`).

use rusqlite::{Connection, params};
use std::path::Path;

/// One row as `list_notes` reports it.
#[derive(Debug, PartialEq, Eq)]
pub struct NoteSummary {
    pub id: i64,
    pub title: String,
    pub created: String,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS notes(
    id      INTEGER PRIMARY KEY,
    title   TEXT NOT NULL,
    body    TEXT NOT NULL,
    created TEXT NOT NULL,
    deleted INTEGER NOT NULL DEFAULT 0
)";

pub struct Store {
    conn: Connection,
}

impl Store {
    /// Open the database at `path` (created on first use) and migrate.
    /// The parent directory must already exist.
    pub fn open(path: &Path) -> rusqlite::Result<Store> {
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Store { conn })
    }

    /// Insert a note; returns its id. `created` is stamped by SQLite
    /// (UTC, RFC 3339) so this crate needs no clock dependency.
    pub fn create(&self, title: &str, body: &str) -> rusqlite::Result<i64> {
        self.conn.execute(
            "INSERT INTO notes(title, body, created)
             VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            params![title, body],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Active (non-deleted) notes, oldest first.
    pub fn list(&self) -> rusqlite::Result<Vec<NoteSummary>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, title, created FROM notes WHERE deleted = 0 ORDER BY id")?;
        let rows = stmt.query_map([], |row| {
            Ok(NoteSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                created: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    /// Soft-delete. `false` when no *active* note has that id (unknown
    /// or already deleted) — the caller turns that into a tool error.
    pub fn delete(&self, id: i64) -> rusqlite::Result<bool> {
        let n = self.conn.execute(
            "UPDATE notes SET deleted = 1 WHERE id = ?1 AND deleted = 0",
            params![id],
        )?;
        Ok(n > 0)
    }

    /// Undo of [`Store::delete`]. `false` when no *deleted* note has
    /// that id (unknown or still active).
    pub fn restore(&self, id: i64) -> rusqlite::Result<bool> {
        let n = self.conn.execute(
            "UPDATE notes SET deleted = 0 WHERE id = ?1 AND deleted = 1",
            params![id],
        )?;
        Ok(n > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("notes.db")).unwrap();
        (dir, store)
    }

    #[test]
    fn create_then_list_returns_the_summary() {
        let (_dir, store) = fixture();
        let id = store.create("first", "hello").unwrap();
        assert!(id > 0);
        let second = store.create("second", "").unwrap();
        assert!(second > id, "ids are monotonic");

        let notes = store.list().unwrap();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].id, id);
        assert_eq!(notes[0].title, "first");
        assert!(
            notes[0].created.ends_with('Z') && notes[0].created.contains('T'),
            "created is an RFC 3339 UTC stamp: {:?}",
            notes[0].created
        );
    }

    #[test]
    fn delete_hides_from_list_and_restore_brings_it_back() {
        let (_dir, store) = fixture();
        let id = store.create("ephemeral", "bye").unwrap();

        assert!(store.delete(id).unwrap());
        assert!(store.list().unwrap().is_empty());

        assert!(store.restore(id).unwrap());
        let notes = store.list().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].id, id);
    }

    #[test]
    fn delete_and_restore_are_strict_about_state() {
        let (_dir, store) = fixture();
        let id = store.create("note", "").unwrap();

        assert!(!store.delete(999).unwrap(), "unknown id deletes nothing");
        assert!(store.delete(id).unwrap());
        assert!(!store.delete(id).unwrap(), "already deleted");

        assert!(!store.restore(999).unwrap(), "unknown id restores nothing");
        assert!(store.restore(id).unwrap());
        assert!(!store.restore(id).unwrap(), "already active");
    }

    #[test]
    fn reopening_the_db_keeps_the_notes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.db");
        let id = Store::open(&path).unwrap().create("durable", "x").unwrap();
        let notes = Store::open(&path).unwrap().list().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].id, id);
    }
}
