//! The daemon's JSON-RPC 2.0 messages, one JSON object per line.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The only JSON-RPC version the daemon speaks.
pub const VERSION: &str = "2.0";
/// The method that records a compaction.
pub const RECORD_COMPACTION: &str = "compaction.record";

/// The request was not valid JSON.
pub const PARSE_ERROR: i64 = -32700;
/// The method does not exist.
pub const METHOD_NOT_FOUND: i64 = -32601;
/// The parameters do not fit the method.
pub const INVALID_PARAMS: i64 = -32602;
/// The daemon failed while handling the request.
pub const INTERNAL_ERROR: i64 = -32603;

/// What `wake hook reload` found when it looked for the progress file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ProgressState {
    /// The file was there and valid.
    Found,
    /// There was no file.
    Missing,
    /// The file was there but could not be read as a progress file.
    Invalid,
}

impl ProgressState {
    /// The word stored in the database.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Found => "found",
            Self::Missing => "missing",
            Self::Invalid => "invalid",
        }
    }

    /// The state stored under `word`, if there is one.
    #[must_use]
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "found" => Some(Self::Found),
            "missing" => Some(Self::Missing),
            "invalid" => Some(Self::Invalid),
            _ => None,
        }
    }
}

/// The parameters of [`RECORD_COMPACTION`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionReport {
    /// The session that compacted, when the hook was told.
    pub session_id: Option<String>,
    /// The folder the agent works in.
    pub cwd: String,
    /// What the hook was told started the session, for example `compact`.
    pub source: String,
    /// Whether the progress file could be reloaded.
    pub progress: ProgressState,
}

/// A JSON-RPC request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// Always [`VERSION`].
    pub jsonrpc: String,
    /// Echoed back in the response.
    pub id: u64,
    /// The method to call.
    pub method: String,
    /// The method's parameters.
    #[serde(default)]
    pub params: Value,
}

/// A JSON-RPC error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RpcError {
    /// One of the error code constants in this module.
    pub code: i64,
    /// What went wrong.
    pub message: String,
}

/// A JSON-RPC response: `result` on success, `error` otherwise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    /// Always [`VERSION`].
    pub jsonrpc: String,
    /// The request's `id`, or `0` if the request could not be read.
    pub id: u64,
    /// The result, on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// The error, on failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl Response {
    pub(crate) fn success(id: u64, result: Value) -> Self {
        Self {
            jsonrpc: VERSION.to_owned(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub(crate) fn failure(id: u64, code: i64, message: String) -> Self {
        Self {
            jsonrpc: VERSION.to_owned(),
            id,
            result: None,
            error: Some(RpcError { code, message }),
        }
    }
}
