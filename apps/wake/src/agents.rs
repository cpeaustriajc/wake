use std::io::{self, Write};
use std::process::ExitCode;

use wake_daemon::protocol::{AgentEntry, AgentProgress};
use wake_daemon::{Error, list_agents};

use crate::paths;

const NO_FILE_LABEL: &str = "(no progress file)";
const INVALID_FILE_LABEL: &str = "(invalid progress file)";
const BRANCH_MISSING_LABEL: &str = "(branch not pushed yet)";
const UNREACHABLE_LABEL: &str = "(cannot read the remote)";
const PENDING_LABEL: &str = "(not read yet)";
const NOT_AVAILABLE: &str = "-";

pub(crate) fn daemon_down(socket: &std::path::Path, error: &Error) -> String {
    match error {
        Error::Io(cause) => format!(
            "the daemon is not running: cannot reach {}: {cause}",
            socket.display()
        ),
        other => other.to_string(),
    }
}

fn plain_label(progress: &AgentProgress) -> &'static str {
    match progress {
        AgentProgress::Missing => NO_FILE_LABEL,
        AgentProgress::BranchMissing => BRANCH_MISSING_LABEL,
        AgentProgress::Unreachable { .. } => UNREACHABLE_LABEL,
        AgentProgress::Pending => PENDING_LABEL,
        _ => INVALID_FILE_LABEL,
    }
}

fn columns(entry: &AgentEntry) -> [String; 5] {
    let (headline, phase, updated) = match &entry.progress {
        AgentProgress::Found {
            headline,
            phase,
            updated,
        } => (headline.clone(), phase.clone(), updated.clone()),
        other => (
            plain_label(other).to_owned(),
            NOT_AVAILABLE.to_owned(),
            NOT_AVAILABLE.to_owned(),
        ),
    };
    let place = match &entry.branch {
        Some(branch) => format!("{} (branch {branch})", entry.folder),
        None => entry.folder.clone(),
    };
    [
        headline,
        phase,
        updated,
        entry.location.as_str().to_owned(),
        place,
    ]
}

fn render(entries: &[AgentEntry]) -> Vec<String> {
    let rows: Vec<[String; 5]> = entries.iter().map(columns).collect();
    let width = |column: usize| {
        rows.iter()
            .map(|row| row[column].chars().count())
            .max()
            .unwrap_or(0)
    };
    let (headline, phase, updated, location) = (width(0), width(1), width(2), width(3));
    rows.iter()
        .map(|row| {
            format!(
                "{:<headline$}  {:<phase$}  {:<updated$}  {:<location$}  {}",
                row[0], row[1], row[2], row[3], row[4]
            )
        })
        .collect()
}

pub(crate) fn run() -> Result<ExitCode, String> {
    let socket = paths::socket()?;
    let entries = list_agents(&socket).map_err(|error| daemon_down(&socket, &error))?;
    let mut out = io::stdout().lock();
    for line in render(&entries) {
        writeln!(out, "{line}").map_err(|error| error.to_string())?;
    }
    Ok(ExitCode::SUCCESS)
}
