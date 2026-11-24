use std::error::Error as StdError;
use std::fmt;
use std::io;

use crate::protocol::RpcError;

/// Why a daemon or client operation failed.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A file or socket operation failed.
    Io(io::Error),
    /// The database failed.
    Sqlite(rusqlite::Error),
    /// A message was not valid JSON.
    Json(serde_json::Error),
    /// The daemon answered with an error.
    Rpc(RpcError),
    /// Another daemon already listens on the socket.
    AlreadyRunning,
    /// The daemon closed the connection without answering.
    NoResponse,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Sqlite(error) => write!(f, "database error: {error}"),
            Self::Json(error) => write!(f, "unreadable message: {error}"),
            Self::Rpc(error) => write!(f, "daemon error {}: {}", error.code, error.message),
            Self::AlreadyRunning => write!(f, "a daemon is already listening on that socket"),
            Self::NoResponse => write!(f, "the daemon closed the connection without answering"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Sqlite(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Rpc(_) | Self::AlreadyRunning | Self::NoResponse => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}
