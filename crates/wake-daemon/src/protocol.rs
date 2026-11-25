//! The daemon's JSON-RPC 2.0 messages, one JSON object per line.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The only JSON-RPC version the daemon speaks.
pub const VERSION: &str = "2.0";
/// The method that records a compaction.
pub const RECORD_COMPACTION: &str = "compaction.record";
/// The method that tells the daemon an agent started.
pub const REGISTER_AGENT: &str = "agent.register";
/// The method that lists every agent the daemon knows.
pub const LIST_AGENTS: &str = "agent.list";
/// The method that tells the daemon to follow a cloud task's branch.
pub const TRACK_CLOUD_TASK: &str = "cloud.track";

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

/// Where an agent runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Location {
    /// On this machine.
    Local,
    /// In the harness's cloud.
    Cloud,
}

impl Location {
    /// The word stored in the database and shown to the user.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Cloud => "cloud",
        }
    }

    /// The location stored under `word`, if there is one.
    #[must_use]
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "local" => Some(Self::Local),
            "cloud" => Some(Self::Cloud),
            _ => None,
        }
    }
}

/// The parameters of [`REGISTER_AGENT`]. Registering a folder that is already
/// registered keeps its agent and its ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRegistration {
    /// The folder the agent works in, as an absolute path.
    pub folder: String,
}

/// The parameters of [`TRACK_CLOUD_TASK`]. Tracking a folder and branch that are
/// already tracked keeps the task and its ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudTracking {
    /// The folder of the repository whose remote `origin` holds the branch, as
    /// an absolute path.
    pub folder: String,
    /// The branch the cloud task pushes to.
    pub branch: String,
}

/// What the daemon read from an agent's progress file when it built the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
#[non_exhaustive]
pub enum AgentProgress {
    /// The file was there and valid.
    Found {
        /// The one-line summary the agent wrote.
        headline: String,
        /// The agent's phase.
        phase: String,
        /// When the agent last updated the file, as an RFC 3339 timestamp.
        updated: String,
    },
    /// There was no file.
    Missing,
    /// The file was there but could not be read as a progress file.
    Invalid {
        /// Why the file was rejected.
        reason: String,
    },
    /// A cloud task's branch is not on the remote yet. A cloud task pushes its
    /// first commit minutes after it starts, so this is a normal state.
    BranchMissing,
    /// A cloud task's remote could not be read, for example when the network is
    /// down.
    Unreachable {
        /// What `git` reported.
        reason: String,
    },
    /// The daemon has not read a cloud task's branch yet since it started.
    Pending,
}

/// One agent in the answer to [`LIST_AGENTS`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentEntry {
    /// The agent's ID, which stays the same across daemon restarts. An ID is
    /// unique only together with `location`: a local agent and a cloud task can
    /// share one.
    pub id: i64,
    /// The folder the agent works in, or for a cloud task the folder of the
    /// repository that holds its branch.
    pub folder: String,
    /// Where the agent runs.
    pub location: Location,
    /// The branch a cloud task pushes to; `None` for a local agent.
    #[serde(default)]
    pub branch: Option<String>,
    /// When the daemon last registered a local agent, or first tracked a cloud
    /// task, in Unix seconds.
    pub started_at: i64,
    /// What the agent's progress file said when the daemon last read it: at the
    /// time of the call for a local agent, within one polling interval for a
    /// cloud task.
    pub progress: AgentProgress,
}

/// The result of [`LIST_AGENTS`]: local agents oldest first, then cloud tasks
/// oldest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentList {
    /// Every agent the daemon knows.
    pub agents: Vec<AgentEntry>,
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
