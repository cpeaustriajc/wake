//! The daemon's SQLite file.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, params};

use crate::error::Error;
use crate::protocol::{CompactionReport, FloorPosition, Location, ProgressState};

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
);
CREATE TABLE IF NOT EXISTS cloud_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    folder TEXT NOT NULL,
    branch TEXT NOT NULL,
    tracked_at INTEGER NOT NULL,
    UNIQUE (folder, branch)
);
CREATE TABLE IF NOT EXISTS floor_positions (
    location TEXT NOT NULL,
    agent_id INTEGER NOT NULL,
    x INTEGER NOT NULL,
    y INTEGER NOT NULL,
    PRIMARY KEY (location, agent_id)
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
    /// Where the agent's tile sits on the floor; `None` until it is placed.
    pub position: Option<FloorPosition>,
}

/// A tracked cloud task as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCloudTask {
    /// The row's ID, increasing in the order tasks were first tracked.
    pub id: i64,
    /// The folder of the repository whose remote `origin` holds the branch.
    pub folder: String,
    /// The branch the cloud task pushes to.
    pub branch: String,
    /// When the daemon first tracked the task, in Unix seconds.
    pub tracked_at: i64,
    /// Where the task's tile sits on the floor; `None` until it is placed.
    pub position: Option<FloorPosition>,
}

fn position(x: Option<i64>, y: Option<i64>) -> Option<FloorPosition> {
    x.zip(y).map(|(x, y)| FloorPosition { x, y })
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
    /// are missing. A database made before the floor existed gains the table
    /// of floor positions and keeps every row it had.
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
        let mut statement = self.connection.prepare(
            "SELECT agents.id, agents.folder, agents.location, agents.started_at,
                    floor_positions.x, floor_positions.y
             FROM agents
             LEFT JOIN floor_positions
                 ON floor_positions.location = agents.location
                 AND floor_positions.agent_id = agents.id
             ORDER BY agents.id",
        )?;
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
                position: position(row.get(4)?, row.get(5)?),
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Stores a cloud task to follow and returns its row ID. A folder and
    /// branch that are already stored keep their row and ID.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Sqlite`] if the write fails.
    pub fn track_cloud_task(&self, folder: &str, branch: &str) -> Result<i64, Error> {
        self.connection.execute(
            "INSERT INTO cloud_tasks (folder, branch, tracked_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(folder, branch) DO NOTHING",
            params![folder, branch, now()],
        )?;
        Ok(self.connection.query_row(
            "SELECT id FROM cloud_tasks WHERE folder = ?1 AND branch = ?2",
            params![folder, branch],
            |row| row.get(0),
        )?)
    }

    /// Every tracked cloud task, oldest first.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Sqlite`] if the query fails.
    pub fn cloud_tasks(&self) -> Result<Vec<StoredCloudTask>, Error> {
        let mut statement = self.connection.prepare(
            "SELECT cloud_tasks.id, cloud_tasks.folder, cloud_tasks.branch,
                    cloud_tasks.tracked_at, floor_positions.x, floor_positions.y
             FROM cloud_tasks
             LEFT JOIN floor_positions
                 ON floor_positions.location = ?1
                 AND floor_positions.agent_id = cloud_tasks.id
             ORDER BY cloud_tasks.id",
        )?;
        let rows = statement.query_map(params![Location::Cloud.as_str()], |row| {
            Ok(StoredCloudTask {
                id: row.get(0)?,
                folder: row.get(1)?,
                branch: row.get(2)?,
                tracked_at: row.get(3)?,
                position: position(row.get(4)?, row.get(5)?),
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Stores where the tile of the agent with this `location` and `id` sits
    /// on the floor, replacing any earlier position. Returns `false`, and
    /// stores nothing, when no such agent exists.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Sqlite`] if the lookup or the write fails.
    pub fn place_on_floor(
        &self,
        location: Location,
        id: i64,
        position: FloorPosition,
    ) -> Result<bool, Error> {
        let lookup = match location {
            Location::Local => "SELECT EXISTS (SELECT 1 FROM agents WHERE id = ?1)",
            Location::Cloud => "SELECT EXISTS (SELECT 1 FROM cloud_tasks WHERE id = ?1)",
        };
        let known: bool = self
            .connection
            .query_row(lookup, params![id], |row| row.get(0))?;
        if !known {
            return Ok(false);
        }
        self.connection.execute(
            "INSERT INTO floor_positions (location, agent_id, x, y) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(location, agent_id) DO UPDATE SET x = excluded.x, y = excluded.y",
            params![location.as_str(), id, position.x, position.y],
        )?;
        Ok(true)
    }
}
