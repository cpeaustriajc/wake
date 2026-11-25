use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use serde::Deserialize;
use wake::progress::{PATH, ProgressFile};
use wake_daemon::protocol::{CompactionReport, ProgressState};
use wake_daemon::record_compaction;

use crate::paths;

const DEFAULT_SOURCE: &str = "compact";

#[derive(Deserialize, Default)]
struct HookInput {
    session_id: Option<String>,
    cwd: Option<String>,
    source: Option<String>,
}

fn read_input() -> HookInput {
    let mut stdin = io::stdin();
    if stdin.is_terminal() {
        return HookInput::default();
    }
    let mut text = String::new();
    if stdin.read_to_string(&mut text).is_err() {
        return HookInput::default();
    }
    serde_json::from_str(&text).unwrap_or_default()
}

fn load(path: &PathBuf) -> (ProgressState, String) {
    match fs::read_to_string(path) {
        Ok(text) => match ProgressFile::parse(&text) {
            Ok(_) => (ProgressState::Found, text),
            Err(error) => (
                ProgressState::Invalid,
                format!(
                    "wake: the progress file at {} is not valid: {error}\n",
                    path.display()
                ),
            ),
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => (
            ProgressState::Missing,
            format!("wake: there is no progress file at {}\n", path.display()),
        ),
        Err(error) => (
            ProgressState::Invalid,
            format!(
                "wake: the progress file at {} cannot be read: {error}\n",
                path.display()
            ),
        ),
    }
}

pub(crate) fn reload() -> ExitCode {
    let input = read_input();
    let folder = input
        .cwd
        .map(PathBuf::from)
        .or_else(|| env::current_dir().ok())
        .unwrap_or_default();
    let (progress, text) = load(&folder.join(PATH));
    let _ = io::stdout().lock().write_all(text.as_bytes());

    let report = CompactionReport {
        session_id: input.session_id,
        cwd: folder.to_string_lossy().into_owned(),
        source: input.source.unwrap_or_else(|| DEFAULT_SOURCE.to_owned()),
        progress,
    };
    let reported = paths::socket()
        .and_then(|socket| record_compaction(&socket, &report).map_err(|error| error.to_string()));
    if let Err(error) = reported {
        let _ = writeln!(
            io::stderr().lock(),
            "wake: the daemon was not told about the compaction: {error}"
        );
    }
    ExitCode::SUCCESS
}
