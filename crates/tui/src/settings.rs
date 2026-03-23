//! Persistent TUI settings stored in the shared devwatch SQLite database.
//!
//! Uses a simple `settings` key/value table alongside the daemon's tables.
//! The TUI opens the DB read-write only for settings; all PR state mutations
//! continue to go through the daemon.

use std::path::PathBuf;

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::app::ColumnId;

// ── DB path (mirrors daemon/src/store.rs) ────────────────────────────────────

fn db_path() -> Result<PathBuf> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot determine local data directory"))?;
    Ok(base.join("devwatch").join("state.db"))
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Load the saved column order from the DB.
/// Returns `None` if no order has been saved yet (caller should use the default).
pub fn load_column_order() -> Option<Vec<ColumnId>> {
    let conn = open_db().ok()?;
    let value: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'column_order'",
            [],
            |row| row.get(0),
        )
        .ok()?;
    parse_column_order(&value)
}

/// Persist the current column order to the DB.
pub fn save_column_order(order: &[ColumnId]) -> Result<()> {
    let conn = open_db().context("open settings DB")?;
    let value = order
        .iter()
        .map(column_id_to_str)
        .collect::<Vec<_>>()
        .join(",");
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["column_order", value],
    )
    .context("write column_order")?;
    Ok(())
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn open_db() -> Result<Connection> {
    let path = db_path()?;
    // DB may not exist yet if the daemon has never run (e.g. demo mode).
    if !path.exists() {
        return Err(anyhow::anyhow!("DB not found at {}", path.display()));
    }
    let conn = Connection::open(&path)
        .with_context(|| format!("open {}", path.display()))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )
    .context("create settings table")?;
    Ok(conn)
}

fn column_id_to_str(col: &ColumnId) -> &'static str {
    match col {
        ColumnId::Number => "Number",
        ColumnId::Repo   => "Repo",
        ColumnId::Title  => "Title",
        ColumnId::Author => "Author",
        ColumnId::Age    => "Age",
        ColumnId::State  => "State",
    }
}

fn str_to_column_id(s: &str) -> Option<ColumnId> {
    match s {
        "Number" => Some(ColumnId::Number),
        "Repo"   => Some(ColumnId::Repo),
        "Title"  => Some(ColumnId::Title),
        "Author" => Some(ColumnId::Author),
        "Age"    => Some(ColumnId::Age),
        "State"  => Some(ColumnId::State),
        _        => None,
    }
}

/// Parse a comma-separated column list.  Only accepts strings where every
/// token maps to a known `ColumnId` *and* all six columns are present
/// (guards against a corrupt or stale DB value).
fn parse_column_order(s: &str) -> Option<Vec<ColumnId>> {
    let cols: Vec<ColumnId> = s.split(',').filter_map(str_to_column_id).collect();
    let default_len = ColumnId::default_order().len();
    // Reject if any column is missing or unknown tokens appeared.
    if cols.len() != default_len { return None; }
    // Reject if any column appears more than once.
    let mut seen = [false; 6];
    for col in &cols {
        let idx = match col {
            ColumnId::Number => 0,
            ColumnId::Repo   => 1,
            ColumnId::Title  => 2,
            ColumnId::Author => 3,
            ColumnId::Age    => 4,
            ColumnId::State  => 5,
        };
        if seen[idx] { return None; }
        seen[idx] = true;
    }
    Some(cols)
}
