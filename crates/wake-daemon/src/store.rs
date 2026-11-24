//! The daemon's SQLite file.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, params};

use crate::error::Error;
use crate::protocol::{CompactionReport, ProgressState};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS compactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    recorded_at INTEGER NOT NULL,
    session_id TEXT,
    cwd TEXT NOT NULL,
    source TEXT NOT NULL,
    progress TEXT NOT NULL
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
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        let recorded_at = i64::try_from(seconds).unwrap_or(i64::MAX);
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
}
