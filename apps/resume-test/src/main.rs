//! Runs the compaction resume test once and writes its result under the
//! results folder.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use wake_claude::agent::{Agent, AgentConfig, Launch};
use wake_daemon::{Store, bind, serve};
use wake_resume_test::judge::StepEvidence;
use wake_resume_test::report::{DaemonRow, Observed, Run, build};
use wake_resume_test::{repo, task};

const CLAUDE_ENV: &str = "WAKE_CLAUDE";
const NO_HOOK_FLAG: &str = "--no-hook";
const USAGE: &str = "usage: wake-resume-test <results folder> <wake binary> [--no-hook]";
const PERMISSION_ARGUMENTS: [&str; 4] = [
    "--permission-mode",
    "acceptEdits",
    "--allowedTools",
    "Bash(git *)",
];

struct Scratch {
    folder: PathBuf,
    repository: PathBuf,
    socket: PathBuf,
    database: PathBuf,
}

fn timestamp(now: OffsetDateTime) -> String {
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

fn rfc3339(now: OffsetDateTime) -> String {
    now.format(&Rfc3339).unwrap_or_else(|_| timestamp(now))
}

fn prepare(run_id: &str) -> Result<Scratch, String> {
    let folder = env::temp_dir().join(format!("wake-resume-{run_id}"));
    let repository = folder.join("repo");
    repo::init(&repository)?;
    let repository = repository
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let socket = folder.join("d.sock");
    let database = folder.join("d.db");
    let store = Store::open(&database).map_err(|error| error.to_string())?;
    let listener = bind(&socket).map_err(|error| error.to_string())?;
    thread::spawn(move || {
        let _ = serve(&listener, store);
    });
    Ok(Scratch {
        folder,
        repository,
        socket,
        database,
    })
}

struct Driven {
    observed: Observed,
    before: Option<Vec<StepEvidence>>,
    errors: Vec<String>,
}

fn drive(
    scratch: &Scratch,
    wake: &Path,
    launch: &Launch,
    driven: &mut Driven,
) -> Result<(), String> {
    let config = AgentConfig {
        folder: scratch.repository.clone(),
        claude: env::var_os(CLAUDE_ENV).map_or_else(|| "claude".into(), PathBuf::from),
        wake: wake.to_owned(),
        socket: scratch.socket.clone(),
    };
    let mut agent = Agent::start_with(&config, launch)
        .map_err(|error| format!("cannot start claude: {error}"))?;
    let [first, compact, resume] = task::messages();
    for (index, text) in [first, compact, resume].iter().enumerate() {
        if text == task::COMPACT {
            driven.before = Some(repo::evidence(&scratch.repository)?);
        }
        agent.send(text).map_err(|error| error.to_string())?;
        for message in agent.read_turn().map_err(|error| error.to_string())? {
            driven.observed.ingest(&message.event);
        }
        if driven.observed.turns.len() <= index {
            return Err(format!(
                "claude closed its output during turn {}",
                index + 1
            ));
        }
    }
    agent.close_input();
    while let Some(message) = agent.read().map_err(|error| error.to_string())? {
        driven.observed.ingest(&message.event);
    }
    if agent.finish().map_err(|error| error.to_string())? {
        Ok(())
    } else {
        Err("claude exited with a failure status".to_owned())
    }
}

fn run_once(results: &Path, wake: &Path, launch: &Launch) -> Result<PathBuf, String> {
    let started = OffsetDateTime::now_utc();
    let reload_hook = launch.reload_hook;
    let label = if reload_hook { "hook" } else { "control" };
    let run_id = format!("{}-{label}", timestamp(started));
    let mut driven = Driven {
        observed: Observed::default(),
        before: None,
        errors: Vec::new(),
    };
    let scratch = prepare(&run_id);
    let mut daemon_rows = Vec::new();
    let mut after = None;
    match &scratch {
        Ok(scratch) => {
            if let Err(error) = drive(scratch, wake, launch, &mut driven) {
                driven.errors.push(error);
            }
            match repo::evidence(&scratch.repository) {
                Ok(evidence) => after = Some(evidence),
                Err(error) => driven.errors.push(error),
            }
            match Store::open(&scratch.database).and_then(|store| store.compactions()) {
                Ok(rows) => daemon_rows = rows.iter().map(DaemonRow::from).collect(),
                Err(error) => driven.errors.push(format!("the daemon's store: {error}")),
            }
        }
        Err(error) => driven.errors.push(error.clone()),
    }
    let result = build(Run {
        run_id: run_id.clone(),
        reload_hook,
        started_at: rfc3339(started),
        finished_at: rfc3339(OffsetDateTime::now_utc()),
        observed: driven.observed,
        daemon_rows,
        before: driven.before,
        after,
        errors: driven.errors,
    });
    fs::create_dir_all(results).map_err(|error| error.to_string())?;
    let path = results.join(format!("{run_id}.json"));
    let text = result.to_json().map_err(|error| error.to_string())?;
    fs::write(&path, format!("{text}\n")).map_err(|error| error.to_string())?;
    if let Ok(scratch) = &scratch {
        let _ = fs::remove_dir_all(&scratch.folder);
    }
    let _ = writeln!(
        io::stdout().lock(),
        "{run_id}: {:?}, {}",
        result.verdict,
        path.display()
    );
    Ok(path)
}

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let (results, wake, reload_hook) = match arguments.as_slice() {
        [results, wake] => (results, wake, true),
        [results, wake, flag] if flag == NO_HOOK_FLAG => (results, wake, false),
        _ => {
            let _ = writeln!(io::stderr().lock(), "{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let launch = Launch {
        reload_hook,
        extra_arguments: PERMISSION_ARGUMENTS.iter().map(OsString::from).collect(),
    };
    let outcome = Path::new(wake)
        .canonicalize()
        .map_err(|error| format!("{wake}: {error}"))
        .and_then(|wake| run_once(Path::new(results), &wake, &launch));
    match outcome {
        Ok(_) => ExitCode::SUCCESS,
        Err(message) => {
            let _ = writeln!(io::stderr().lock(), "resume-test: {message}");
            ExitCode::FAILURE
        }
    }
}
