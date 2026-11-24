//! Wake's own types for the messages `claude` writes in stream-json mode.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use serde::Deserialize;
use serde_json::Value;

const TYPE: &str = "type";
const SUBTYPE: &str = "subtype";

/// `system` / `init`: the first thing a session announces.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Init {
    /// The session's ID.
    pub session_id: String,
    /// The folder the agent runs in.
    pub cwd: String,
    /// The model the session started with.
    pub model: String,
    /// The version of the `claude` binary.
    pub claude_code_version: String,
    /// The permission mode the session runs under.
    #[serde(rename = "permissionMode")]
    pub permission_mode: String,
}

/// One usage window inside a [`RateLimit`].
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct UsageWindow {
    /// How much of the window is used, as `claude` reports it.
    pub utilization: f64,
    /// When the window resets, in Unix seconds.
    #[serde(rename = "resetsAt")]
    pub resets_at: i64,
    /// Whether the window still allows requests, for example `allowed`.
    pub status: String,
}

/// `rate_limit_event`: the account's usage windows.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RateLimit {
    /// Whether requests are allowed right now, for example `allowed`.
    pub status: String,
    /// The window `claude` is reporting on, for example `seven_day`.
    #[serde(rename = "rateLimitType")]
    pub rate_limit_type: String,
    /// When that window resets, in Unix seconds, if `claude` says.
    #[serde(rename = "resetsAt")]
    pub resets_at: Option<i64>,
    /// Every usage window `claude` reported, by name.
    #[serde(rename = "unifiedWindows", default)]
    pub windows: BTreeMap<String, UsageWindow>,
}

#[derive(Deserialize)]
struct RateLimitEnvelope {
    rate_limit_info: RateLimit,
}

/// `autocompact_state`: the context window and where compaction starts.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AutocompactState {
    /// Whether automatic compaction is on.
    pub enabled: bool,
    /// The context window in tokens.
    pub effective_window: u64,
    /// The token count at which compaction starts.
    pub threshold: u64,
    /// Whether `claude` enforces the threshold.
    pub enforced: bool,
    /// Where the setting comes from, for example `model-default`.
    pub source: String,
}

#[derive(Deserialize)]
struct AutocompactEnvelope {
    value: AutocompactState,
}

/// `system` / `hook_response`: a hook finished.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HookResponse {
    /// The hook as `claude` names it, for example `SessionStart:compact`.
    pub hook_name: String,
    /// What the hook wrote to standard output.
    pub stdout: String,
    /// The hook's exit code.
    pub exit_code: i64,
    /// `success` or the failure `claude` reports.
    pub outcome: String,
}

/// What `claude` records about a compaction in `compact_boundary`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CompactMetadata {
    /// What started the compaction: `manual` or `auto`.
    pub trigger: String,
    /// Tokens in the context before compaction.
    pub pre_tokens: u64,
    /// Tokens in the context after compaction.
    pub post_tokens: u64,
}

#[derive(Deserialize)]
struct CompactBoundaryEnvelope {
    compact_metadata: CompactMetadata,
}

/// `result`: a turn ended.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TurnResult {
    /// Whether the turn ended in an error.
    pub is_error: bool,
    /// The number of model turns the request took.
    pub num_turns: u64,
}

/// One message read from `claude`'s standard output.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// The session started.
    Init(Init),
    /// The usage windows changed or were reported.
    RateLimit(RateLimit),
    /// The context window and compaction threshold.
    AutocompactState(AutocompactState),
    /// A hook ran.
    HookResponse(HookResponse),
    /// A compaction finished.
    CompactBoundary(CompactMetadata),
    /// A turn ended; the next message can be sent.
    Result(TurnResult),
    /// A message Wake does not read, with its `type` and `subtype`.
    Other {
        /// The message's `type`.
        kind: String,
        /// The message's `subtype`, if it has one.
        subtype: Option<String>,
    },
}

/// Why a line of `claude` output could not be read.
#[derive(Debug)]
#[non_exhaustive]
pub enum ParseError {
    /// The line is not JSON, or a message Wake reads lacks a field it needs.
    Json(serde_json::Error),
    /// The line is JSON but has no `type`.
    MissingType,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(f, "unreadable stream-json line: {error}"),
            Self::MissingType => write!(f, "stream-json line has no type"),
        }
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::MissingType => None,
        }
    }
}

impl From<serde_json::Error> for ParseError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl Event {
    /// Reads one line of `claude` output.
    ///
    /// Messages Wake does not read become [`Event::Other`]. A message Wake
    /// does read but whose shape changed is an error, so a `claude` update
    /// that moves a field fails loudly.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the line is not JSON, has no `type`, or is a
    /// message Wake reads that lacks a field Wake needs.
    pub fn parse(line: &str) -> Result<Self, ParseError> {
        let value: Value = serde_json::from_str(line)?;
        let kind = value
            .get(TYPE)
            .and_then(Value::as_str)
            .ok_or(ParseError::MissingType)?
            .to_owned();
        let subtype = value
            .get(SUBTYPE)
            .and_then(Value::as_str)
            .map(str::to_owned);
        let event = match (kind.as_str(), subtype.as_deref()) {
            ("system", Some("init")) => Self::Init(serde_json::from_value(value)?),
            ("system", Some("hook_response")) => Self::HookResponse(serde_json::from_value(value)?),
            ("system", Some("compact_boundary")) => Self::CompactBoundary(
                serde_json::from_value::<CompactBoundaryEnvelope>(value)?.compact_metadata,
            ),
            ("rate_limit_event", _) => {
                Self::RateLimit(serde_json::from_value::<RateLimitEnvelope>(value)?.rate_limit_info)
            }
            ("autocompact_state", _) => {
                Self::AutocompactState(serde_json::from_value::<AutocompactEnvelope>(value)?.value)
            }
            ("result", _) => Self::Result(serde_json::from_value(value)?),
            _ => Self::Other { kind, subtype },
        };
        Ok(event)
    }
}
