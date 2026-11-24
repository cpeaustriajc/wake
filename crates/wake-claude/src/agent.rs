//! Starting and talking to a local `claude` agent.

use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

use crate::stream::{Event, ParseError};

/// The environment variable that tells `wake hook reload` where the daemon's
/// socket is.
pub const SOCKET_ENV: &str = "WAKE_SOCKET";

const HOOK_MATCHER: &str = "compact";
const HOOK_COMMAND_ARGUMENTS: &str = "hook reload";
const PROGRESS_INSTRUCTIONS: &str = "Keep your progress file at .wake/progress.md current. It starts with front matter holding `phase`, `headline` and `updated` (an RFC 3339 timestamp), then your own notes in Markdown. Update it whenever your phase or headline changes and before long steps, and commit it to your branch. After your context is compacted it is printed back to you: treat it, not the compaction summary, as the source of truth.";

/// Where and how to start an agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConfig {
    /// The folder the agent works in.
    pub folder: PathBuf,
    /// The `claude` binary to run.
    pub claude: PathBuf,
    /// The `wake` binary the reload hook runs, as an absolute path.
    pub wake: PathBuf,
    /// The daemon socket the reload hook reports to.
    pub socket: PathBuf,
}

/// The settings `claude` receives for one run: a `SessionStart` hook for the
/// `compact` matcher that runs `wake hook reload`. They are passed on the
/// command line, so nothing is written to the repository or the home folder.
#[must_use]
pub fn hook_settings(wake: &Path) -> Value {
    let command = format!("{} {HOOK_COMMAND_ARGUMENTS}", shell_quote(wake));
    json!({
        "hooks": {
            "SessionStart": [{
                "matcher": HOOK_MATCHER,
                "hooks": [{ "type": "command", "command": command }]
            }]
        }
    })
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

/// The `claude` command for an agent, not yet started.
#[must_use]
pub fn command(config: &AgentConfig) -> Command {
    let mut command = Command::new(&config.claude);
    let arguments: [OsString; 9] = [
        "-p".into(),
        "--input-format".into(),
        "stream-json".into(),
        "--output-format".into(),
        "stream-json".into(),
        "--verbose".into(),
        "--settings".into(),
        hook_settings(&config.wake).to_string().into(),
        "--append-system-prompt".into(),
    ];
    command
        .args(arguments)
        .arg(PROGRESS_INSTRUCTIONS)
        .current_dir(&config.folder)
        .env(SOCKET_ENV, &config.socket)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    command
}

/// Why reading from an agent failed.
#[derive(Debug)]
#[non_exhaustive]
pub enum ReadError {
    /// The pipe from `claude` failed.
    Io(io::Error),
    /// A line was not a message Wake can read.
    Parse(ParseError),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "reading from claude failed: {error}"),
            Self::Parse(error) => error.fmt(f),
        }
    }
}

impl Error for ReadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Parse(error) => Some(error),
        }
    }
}

/// One line of `claude` output, kept as written next to what Wake read from it.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    /// The line exactly as `claude` wrote it, without the newline.
    pub raw: String,
    /// What Wake read from it.
    pub event: Event,
}

/// A running `claude` process driven over stream-json.
#[derive(Debug)]
pub struct Agent {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
}

impl Agent {
    /// Starts `claude` in the configured folder.
    ///
    /// # Errors
    ///
    /// Returns the I/O error if `claude` cannot be started.
    pub fn start(config: &AgentConfig) -> io::Result<Self> {
        let mut child = command(config).spawn()?;
        let input = child.stdin.take();
        let output = child
            .stdout
            .take()
            .map(BufReader::new)
            .ok_or_else(|| io::Error::other("claude has no standard output"))?;
        Ok(Self {
            child,
            input,
            output,
        })
    }

    /// Sends one user message. `claude` runs it as a turn, so a slash command
    /// such as `/compact` works the same way.
    ///
    /// # Errors
    ///
    /// Returns the I/O error if input is closed or the pipe is broken.
    pub fn send(&mut self, text: &str) -> io::Result<()> {
        let input = self
            .input
            .as_mut()
            .ok_or_else(|| io::Error::other("input to claude is closed"))?;
        let message = json!({ "type": "user", "message": { "role": "user", "content": text } });
        writeln!(input, "{message}")?;
        input.flush()
    }

    /// Closes input so `claude` ends the session after its current work.
    pub fn close_input(&mut self) {
        self.input = None;
    }

    /// Reads the next message, or `None` once `claude` has closed its output.
    ///
    /// # Errors
    ///
    /// Returns [`ReadError`] if the pipe fails or a line is not readable.
    pub fn read(&mut self) -> Result<Option<Message>, ReadError> {
        let mut line = String::new();
        loop {
            line.clear();
            if self.output.read_line(&mut line).map_err(ReadError::Io)? == 0 {
                return Ok(None);
            }
            let raw = line.trim_end_matches(['\n', '\r']);
            if raw.is_empty() {
                continue;
            }
            let event = Event::parse(raw).map_err(ReadError::Parse)?;
            return Ok(Some(Message {
                raw: raw.to_owned(),
                event,
            }));
        }
    }

    /// Reads messages until a turn ends, returning every message read
    /// including the [`Event::Result`] that ended it, or what was read before
    /// `claude` closed its output.
    ///
    /// # Errors
    ///
    /// Returns [`ReadError`] if the pipe fails or a line is not readable.
    pub fn read_turn(&mut self) -> Result<Vec<Message>, ReadError> {
        let mut messages = Vec::new();
        while let Some(message) = self.read()? {
            let ended = matches!(message.event, Event::Result(_));
            messages.push(message);
            if ended {
                break;
            }
        }
        Ok(messages)
    }

    /// Closes input, waits for `claude` to exit and returns whether it
    /// exited successfully.
    ///
    /// # Errors
    ///
    /// Returns the I/O error if waiting fails.
    pub fn finish(mut self) -> io::Result<bool> {
        self.input = None;
        Ok(self.child.wait()?.success())
    }
}
