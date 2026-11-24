//! Re-records the `claude` stream-json fixtures from real runs, replacing
//! anything personal with placeholders before writing them.

mod redact;

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;

use wake_claude::agent::{Agent, AgentConfig, Message};
use wake_daemon::{Store, bind, serve};

use redact::Redactor;

const CLAUDE_ENV: &str = "WAKE_CLAUDE";
const SEEDED_PROGRESS: &str = "---\nphase: build\nheadline: Adding the parser\nupdated: 2026-11-24T14:00:00Z\n---\nNext step: write the tests for the parser.\n";
const SESSION_MESSAGES: [&str; 1] = ["Reply with the single word ok."];
const COMPACTION_MESSAGES: [&str; 3] = [
    "Remember the number 7421. Reply with ok.",
    "/compact",
    "What number did I ask you to remember, and what is the headline in .wake/progress.md? Answer in one short sentence.",
];

struct Scratch {
    folder: PathBuf,
    socket: PathBuf,
}

fn prepare() -> Result<Scratch, String> {
    let folder = env::temp_dir().join(format!("wake-fixtures-{}", std::process::id()));
    fs::create_dir_all(folder.join(".wake")).map_err(|error| error.to_string())?;
    fs::write(folder.join(".wake/progress.md"), SEEDED_PROGRESS)
        .map_err(|error| error.to_string())?;
    let folder = folder.canonicalize().map_err(|error| error.to_string())?;
    let socket = folder.join("d.sock");
    let store = Store::open(&folder.join("d.db")).map_err(|error| error.to_string())?;
    let listener = bind(&socket).map_err(|error| error.to_string())?;
    thread::spawn(move || {
        let _ = serve(&listener, store);
    });
    Ok(Scratch { folder, socket })
}

fn run(scratch: &Scratch, wake: &Path, messages: &[&str]) -> Result<Vec<Message>, String> {
    let config = AgentConfig {
        folder: scratch.folder.clone(),
        claude: env::var_os(CLAUDE_ENV).map_or_else(|| "claude".into(), PathBuf::from),
        wake: wake.to_owned(),
        socket: scratch.socket.clone(),
    };
    let mut agent = Agent::start(&config).map_err(|error| error.to_string())?;
    let mut recorded = Vec::new();
    for text in messages {
        agent.send(text).map_err(|error| error.to_string())?;
        recorded.extend(agent.read_turn().map_err(|error| error.to_string())?);
    }
    agent.close_input();
    while let Some(message) = agent.read().map_err(|error| error.to_string())? {
        recorded.push(message);
    }
    agent.finish().map_err(|error| error.to_string())?;
    Ok(recorded)
}

fn record(scratch: &Scratch, wake: &Path, messages: &[&str], output: &Path) -> Result<(), String> {
    let recorded = run(scratch, wake, messages)?;
    let mut redactor = Redactor::new(&scratch.folder);
    let mut text = String::new();
    for message in &recorded {
        text.push_str(&redactor.line(&message.raw)?);
        text.push('\n');
    }
    redactor.check(&text)?;
    fs::write(output, text).map_err(|error| error.to_string())?;
    let _ = writeln!(io::stdout().lock(), "wrote {}", output.display());
    Ok(())
}

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let [folder, wake] = arguments.as_slice() else {
        let _ = writeln!(
            io::stderr().lock(),
            "usage: wake-record-fixtures <output folder> <wake binary>"
        );
        return ExitCode::FAILURE;
    };
    let outcome = (|| {
        let output = PathBuf::from(folder);
        fs::create_dir_all(&output).map_err(|error| error.to_string())?;
        let wake = PathBuf::from(wake)
            .canonicalize()
            .map_err(|error| format!("{wake}: {error}"))?;
        let scratch = prepare()?;
        record(
            &scratch,
            &wake,
            &SESSION_MESSAGES,
            &output.join("session.jsonl"),
        )?;
        record(
            &scratch,
            &wake,
            &COMPACTION_MESSAGES,
            &output.join("compaction.jsonl"),
        )?;
        fs::remove_dir_all(&scratch.folder).map_err(|error| error.to_string())
    })();
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            let _ = writeln!(io::stderr().lock(), "record-fixtures: {message}");
            ExitCode::FAILURE
        }
    }
}
