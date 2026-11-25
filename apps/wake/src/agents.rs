use std::io::{self, Write};
use std::process::ExitCode;

use wake_daemon::protocol::{AgentEntry, AgentProgress};
use wake_daemon::{Error, list_agents};

use crate::paths;

const NO_FILE_LABEL: &str = "(no progress file)";
const INVALID_FILE_LABEL: &str = "(invalid progress file)";
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

fn columns(entry: &AgentEntry) -> [String; 5] {
    let (headline, phase, updated) = match &entry.progress {
        AgentProgress::Found {
            headline,
            phase,
            updated,
        } => (headline.clone(), phase.clone(), updated.clone()),
        AgentProgress::Missing => (
            NO_FILE_LABEL.to_owned(),
            NOT_AVAILABLE.to_owned(),
            NOT_AVAILABLE.to_owned(),
        ),
        _ => (
            INVALID_FILE_LABEL.to_owned(),
            NOT_AVAILABLE.to_owned(),
            NOT_AVAILABLE.to_owned(),
        ),
    };
    [
        headline,
        phase,
        updated,
        entry.location.as_str().to_owned(),
        entry.folder.clone(),
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
