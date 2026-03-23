use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use devwatch_core::types::{PullRequest, VcsEvent};

pub struct Store {
    conn: Connection,
}

impl Store {
    /// Open (or create) the SQLite database at
    /// `{data_local_dir}/devwatch/state.db`.
    pub fn open() -> Result<Self> {
        let db_path = db_path()?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create dir {}", parent.display()))?;
        }

        let conn = Connection::open(&db_path)
            .with_context(|| format!("open SQLite at {}", db_path.display()))?;

        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "PRAGMA journal_mode=WAL;

            CREATE TABLE IF NOT EXISTS pull_requests (
                provider   TEXT    NOT NULL,
                repo       TEXT    NOT NULL,
                number     INTEGER NOT NULL,
                title      TEXT    NOT NULL DEFAULT '',
                state      TEXT    NOT NULL DEFAULT '',
                url        TEXT    NOT NULL DEFAULT '',
                author     TEXT    NOT NULL DEFAULT '',
                seen_at    INTEGER NOT NULL,
                PRIMARY KEY (provider, repo, number)
            );

            CREATE TABLE IF NOT EXISTS events (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type  TEXT    NOT NULL,
                provider    TEXT    NOT NULL,
                repo        TEXT    NOT NULL,
                pr_number   INTEGER NOT NULL,
                occurred_at INTEGER NOT NULL
            );",
        )?;

        // Additive migrations — ignore "duplicate column name" if already applied.
        for sql in [
            "ALTER TABLE pull_requests ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE pull_requests ADD COLUMN draft      INTEGER NOT NULL DEFAULT 0",
        ] {
            match self.conn.execute_batch(sql) {
                Ok(_) => {}
                Err(e) if e.to_string().contains("duplicate column name") => {}
                Err(e) => return Err(e.into()),
            }
        }

        Ok(())
    }

    pub fn load_prs(&self) -> Result<Vec<PullRequest>> {
        let mut stmt = self.conn.prepare(
            "SELECT provider, repo, number, title, state, url, author, created_at, draft
             FROM pull_requests",
        )?;
        let prs = stmt
            .query_map([], |row| {
                Ok(PullRequest {
                    id:         0, // not stored; re-fetched from API on first poll
                    number:     row.get::<_, u64>(2)?,
                    title:      row.get(3)?,
                    state:      row.get(4)?,
                    url:        row.get(5)?,
                    author:     row.get(6)?,
                    repo:       row.get(1)?,
                    provider:   row.get(0)?,
                    created_at: row.get::<_, i64>(7).map(|v| v.max(0) as u64).unwrap_or(0),
                    draft:      row.get::<_, i32>(8).map(|v| v != 0).unwrap_or(false),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(prs)
    }

    pub fn upsert_pr(&self, pr: &PullRequest) -> Result<()> {
        let now = unix_now();
        self.conn.execute(
            "INSERT INTO pull_requests
                (provider, repo, number, title, state, url, author, seen_at, created_at, draft)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(provider, repo, number) DO UPDATE SET
                title      = excluded.title,
                state      = excluded.state,
                url        = excluded.url,
                author     = excluded.author,
                seen_at    = excluded.seen_at,
                created_at = excluded.created_at,
                draft      = excluded.draft",
            params![
                pr.provider, pr.repo, pr.number, pr.title, pr.state, pr.url, pr.author, now,
                pr.created_at as i64, pr.draft as i32,
            ],
        )?;
        Ok(())
    }

    pub fn delete_pr(&self, provider: &str, repo: &str, number: u64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM pull_requests WHERE provider = ?1 AND repo = ?2 AND number = ?3",
            params![provider, repo, number],
        )?;
        Ok(())
    }

    pub fn record_event(&self, event: &VcsEvent) -> Result<()> {
        let now = unix_now();
        let (event_type, provider, repo, number) = match event {
            VcsEvent::NewPullRequest(pr) => ("new", &pr.provider, &pr.repo, pr.number),
            VcsEvent::PullRequestUpdated { new, .. } => ("updated", &new.provider, &new.repo, new.number),
            VcsEvent::PullRequestClosed(pr) => ("closed", &pr.provider, &pr.repo, pr.number),
        };
        self.conn.execute(
            "INSERT INTO events (event_type, provider, repo, pr_number, occurred_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![event_type, provider, repo, number, now],
        )?;
        Ok(())
    }
}

fn db_path() -> Result<PathBuf> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot determine local data directory"))?;
    Ok(base.join("devwatch").join("state.db"))
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
