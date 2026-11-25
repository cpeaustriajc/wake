//! The daemon's SQLite file.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, params};

use crate::error::Error;
use crate::protocol::{CompactionReport, Location, ProgressState};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS compactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    recorded_at INTEGER NOT NULL,
    session_id TEXT,
    cwd TEXT NOT NULL,
    source TEXT NOT NULL,
    progress TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS agents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    folder TEXT NOT NULL UNIQUE,
    location TEXT NOT NULL,
    started_at INTEGER NOT NULL
);";

/// A compaction as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCompaction {
    /// The row's ID, increasing in the order compactions were recorded.
    pub id: i64,
    /// When the daemon recorded it, in Unix seconds.
    pub recorded_at: i64,
    /// What the hook reported.
    pub report: CompactionReport,
}

/// An agent as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredAgent {
    /// The row's ID, increasing in the order agents were first registered.
    pub id: i64,
    /// The folder the agent works in.
    pub folder: String,
    /// Where the agent runs.
    pub location: Location,
    /// When the daemon last registered the agent, in Unix seconds.
    pub started_at: i64,
}

fn now() -> i64 {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    i64::try_from(seconds).unwrap_or(i64::MAX)
}

/// The daemon's database.
#[derive(Debug)]
pub struct Store {
    connection: Connection,
}

impl Store {
    /// Opens the database at `path`, creating the file and its tables if they
    /// are missing.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Sqlite`] if SQLite cannot open or set up the file.
    pub fn open(path: &Path) -> Result<Self, Error> {
        let connection = Connection::open(path)?;
        connection.execute_batch(SCHEMA)?;
        Ok(Self { connection })
    }

    /// Stores a compaction and returns its row ID.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Sqlite`] if the insert fails.
    pub fn record_compaction(&self, report: &CompactionReport) -> Result<i64, Error> {
        let recorded_at = now();
        self.connection.execute(
            "INSERT INTO compactions (recorded_at, session_id, cwd, source, progress)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                recorded_at,
                report.session_id,
                report.cwd,
                report.source,
                report.progress.as_str()
            ],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    /// Every stored compaction, oldest first.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Sqlite`] if the query fails.
    pub fn compactions(&self) -> Result<Vec<StoredCompaction>, Error> {
        let mut statement = self.connection.prepare(
            "SELECT id, recorded_at, session_id, cwd, source, progress
             FROM compactions ORDER BY id",
        )?;
        let rows = statement.query_map([], |row| {
            let word: String = row.get(5)?;
            let progress = ProgressState::from_word(&word).ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    format!("unknown progress state `{word}`").into(),
                )
            })?;
            Ok(StoredCompaction {
                id: row.get(0)?,
                recorded_at: row.get(1)?,
                report: CompactionReport {
                    session_id: row.get(2)?,
                    cwd: row.get(3)?,
                    source: row.get(4)?,
                    progress,
                },
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Stores an agent that works in `folder` and returns its row ID. A folder
    /// that is already stored keeps its row and ID, and its start time moves
    /// to now.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Sqlite`] if the write fails.
    pub fn register_agent(&self, folder: &str, location: Location) -> Result<i64, Error> {
        self.connection.execute(
            "INSERT INTO agents (folder, location, started_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(folder) DO UPDATE SET
                 location = excluded.location, started_at = excluded.started_at",
            params![folder, location.as_str(), now()],
        )?;
        Ok(self.connection.query_row(
            "SELECT id FROM agents WHERE folder = ?1",
            params![folder],
            |row| row.get(0),
        )?)
    }

    /// Every stored agent, oldest first.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Sqlite`] if the query fails.
    pub fn agents(&self) -> Result<Vec<StoredAgent>, Error> {
        let mut statement = self
            .connection
            .prepare("SELECT id, folder, location, started_at FROM agents ORDER BY id")?;
        let rows = statement.query_map([], |row| {
            let word: String = row.get(2)?;
            let location = Location::from_word(&word).ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    format!("unknown location `{word}`").into(),
                )
            })?;
            Ok(StoredAgent {
                id: row.get(0)?,
                folder: row.get(1)?,
                location,
                started_at: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}
