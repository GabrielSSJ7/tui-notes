use std::path::Path;

use anyhow::Result;
use chrono::NaiveDate;
use rusqlite::{params, Connection, Row};

use crate::models::Reminder;

const DATE_FMT: &str = "%Y-%m-%d";

/// Thin owned wrapper over the sqlite connection holding reminders. All
/// timestamps are passed in by the caller so the store stays pure/testable.
pub struct ReminderStore {
    conn: Connection,
}

impl ReminderStore {
    pub fn open(path: &Path) -> Result<Self> {
        Self::from_conn(Connection::open(path)?)
    }

    /// In-memory database for tests.
    pub fn in_memory() -> Result<Self> {
        Self::from_conn(Connection::open_in_memory()?)
    }

    fn from_conn(conn: Connection) -> Result<Self> {
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS reminders (
                id           INTEGER PRIMARY KEY,
                text         TEXT NOT NULL,
                due          TEXT,
                created_at   INTEGER NOT NULL,
                dismissed_at INTEGER
            );",
        )?;
        Ok(())
    }

    /// Insert an active reminder, returning its new id.
    pub fn add(&self, text: &str, due: Option<NaiveDate>, now: i64) -> Result<i64> {
        let due_str = due.map(|d| d.format(DATE_FMT).to_string());
        self.conn.execute(
            "INSERT INTO reminders (text, due, created_at) VALUES (?1, ?2, ?3)",
            params![text, due_str, now],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Active (non-dismissed) reminders, dated ones first, earliest due first.
    pub fn active(&self) -> Result<Vec<Reminder>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, text, due, created_at FROM reminders
             WHERE dismissed_at IS NULL
             ORDER BY due IS NULL, due ASC, created_at ASC",
        )?;
        let rows = stmt.query_map([], map_reminder)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Mark a reminder dismissed; it disappears from `active`.
    pub fn dismiss(&self, id: i64, now: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE reminders SET dismissed_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }
}

fn map_reminder(row: &Row) -> rusqlite::Result<Reminder> {
    let due_str: Option<String> = row.get(2)?;
    let due = due_str.and_then(|s| NaiveDate::parse_from_str(&s, DATE_FMT).ok());
    Ok(Reminder {
        id: row.get(0)?,
        text: row.get(1)?,
        due,
        created_at: row.get(3)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn add_then_active_returns_it() {
        let store = ReminderStore::in_memory().unwrap();
        store.add("buy milk", Some(date(2026, 7, 4)), 100).unwrap();
        let active = store.active().unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].text, "buy milk");
        assert_eq!(active[0].due, Some(date(2026, 7, 4)));
    }

    #[test]
    fn dismiss_removes_from_active() {
        let store = ReminderStore::in_memory().unwrap();
        let id = store.add("temp", None, 1).unwrap();
        store.dismiss(id, 2).unwrap();
        assert!(store.active().unwrap().is_empty());
    }

    #[test]
    fn dated_reminders_sort_before_undated() {
        let store = ReminderStore::in_memory().unwrap();
        store.add("no date", None, 1).unwrap();
        store.add("dated", Some(date(2026, 7, 4)), 2).unwrap();
        let active = store.active().unwrap();
        assert_eq!(active[0].text, "dated");
    }
}
