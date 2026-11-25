//! Runs the compaction resume test once and writes its result under the
//! results folder.

use std::cell::RefCell;
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
use wake_claude::stream::Event;
use wake_daemon::{Store, bind, serve};
use wake_resume_test::hard_judge::{HardStep, Snapshot};
use wake_resume_test::hard_report::{HardObserved, HardRun, build_hard};
use wake_resume_test::judge::StepEvidence;
use wake_resume_test::report::{DaemonRow, Observed, Run, build};
use wake_resume_test::{compare, hard_repo, hard_task, repo, task};

const CLAUDE_ENV: &str = "WAKE_CLAUDE";
const NO_HOOK_FLAG: &str = "--no-hook";
const HARDER_FLAG: &str = "--harder";
const TRIAL_FLAG: &str = "--trial";
const COMPARE_COMMAND: &str = "compare";
const USAGE: &str = "usage: wake-resume-test <results folder> <wake binary> [--harder] [--trial] [--no-hook]\n       wake-resume-test compare <results folder>";
const ISOLATION_ARGUMENTS: [&str; 2] = [
    "--disallowedTools",
    "Bash(git push*),Bash(git -C*),Bash(git --git-dir*),Bash(git --work-tree*),Bash(git remote*)",
];
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

fn converse(
    scratch: &Scratch,
    wake: &Path,
    launch: &Launch,
    messages: &[String; 3],
    observe: &mut dyn FnMut(&Event),
    before_compact: &mut dyn FnMut() -> Result<(), String>,
) -> Result<(), String> {
    let config = AgentConfig {
        folder: scratch.repository.clone(),
        claude: env::var_os(CLAUDE_ENV).map_or_else(|| "claude".into(), PathBuf::from),
        wake: wake.to_owned(),
        socket: scratch.socket.clone(),
    };
    let mut agent = Agent::start_with(&config, launch)
        .map_err(|error| format!("cannot start claude: {error}"))?;
    let mut turns_ended = 0;
    for (index, text) in messages.iter().enumerate() {
        if text == task::COMPACT {
            before_compact()?;
        }
        agent.send(text).map_err(|error| error.to_string())?;
        for message in agent.read_turn().map_err(|error| error.to_string())? {
            if matches!(message.event, Event::Result(_)) {
                turns_ended += 1;
            }
            observe(&message.event);
        }
        if turns_ended <= index {
            return Err(format!(
                "claude closed its output during turn {}",
                index + 1
            ));
        }
    }
    agent.close_input();
    while let Some(message) = agent.read().map_err(|error| error.to_string())? {
        observe(&message.event);
    }
    if agent.finish().map_err(|error| error.to_string())? {
        Ok(())
    } else {
        Err("claude exited with a failure status".to_owned())
    }
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
    let observed = RefCell::new(&mut driven.observed);
    let before = RefCell::new(&mut driven.before);
    converse(
        scratch,
        wake,
        launch,
        &task::messages(),
        &mut |event| observed.borrow_mut().ingest(event),
        &mut || {
            **before.borrow_mut() = Some(repo::evidence(&scratch.repository)?);
            Ok(())
        },
    )
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

struct HardDriven {
    observed: HardObserved,
    snapshot: Option<Snapshot>,
    before: Option<Vec<HardStep>>,
    errors: Vec<String>,
}

fn drive_hard(
    scratch: &Scratch,
    wake: &Path,
    launch: &Launch,
    driven: &mut HardDriven,
) -> Result<(), String> {
    let observed = RefCell::new(&mut driven.observed);
    let snapshot = RefCell::new(&mut driven.snapshot);
    let before = RefCell::new(&mut driven.before);
    converse(
        scratch,
        wake,
        launch,
        &hard_task::messages(),
        &mut |event| observed.borrow_mut().ingest(event),
        &mut || {
            let seen = hard_repo::snapshot(&scratch.repository);
            let decisions = seen.decisions.clone().unwrap_or_default();
            **before.borrow_mut() = Some(hard_repo::evidence(&scratch.repository, &decisions)?);
            **snapshot.borrow_mut() = Some(seen);
            Ok(())
        },
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Trial,
    Recorded,
}

fn run_hard(results: &Path, wake: &Path, launch: &Launch, kind: Kind) -> Result<PathBuf, String> {
    let trial = kind == Kind::Trial;
    let started = OffsetDateTime::now_utc();
    let reload_hook = launch.reload_hook;
    let arm = if reload_hook { "hook" } else { "control" };
    let suffix = if trial { "-trial" } else { "" };
    let run_id = format!("{}-harder-{arm}{suffix}", timestamp(started));
    let mut driven = HardDriven {
        observed: HardObserved::default(),
        snapshot: None,
        before: None,
        errors: Vec::new(),
    };
    let scratch = prepare(&run_id);
    let mut daemon_rows = Vec::new();
    let mut after = None;
    match &scratch {
        Ok(scratch) => {
            if let Err(error) = drive_hard(scratch, wake, launch, &mut driven) {
                driven.errors.push(error);
            }
            let decisions = driven
                .snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.decisions.clone().ok())
                .unwrap_or_default();
            match hard_repo::evidence(&scratch.repository, &decisions) {
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
    let result = build_hard(HardRun {
        run_id: run_id.clone(),
        trial,
        reload_hook,
        started_at: rfc3339(started),
        finished_at: rfc3339(OffsetDateTime::now_utc()),
        observed: driven.observed,
        daemon_rows,
        snapshot: driven.snapshot,
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

fn compare_results(results: &str) -> ExitCode {
    match compare::read_rows(Path::new(results)) {
        Ok(rows) => {
            let _ = write!(io::stdout().lock(), "{}", compare::render(&rows));
            ExitCode::SUCCESS
        }
        Err(message) => {
            let _ = writeln!(io::stderr().lock(), "resume-test: {message}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if let [command, results] = arguments.as_slice()
        && command == COMPARE_COMMAND
    {
        return compare_results(results);
    }
    let [results, wake, flags @ ..] = arguments.as_slice() else {
        let _ = writeln!(io::stderr().lock(), "{USAGE}");
        return ExitCode::FAILURE;
    };
    let known = [NO_HOOK_FLAG, HARDER_FLAG, TRIAL_FLAG];
    let has = |flag: &str| flags.iter().any(|given| given == flag);
    if flags.iter().any(|given| !known.contains(&given.as_str()))
        || (has(TRIAL_FLAG) && !has(HARDER_FLAG))
    {
        let _ = writeln!(io::stderr().lock(), "{USAGE}");
        return ExitCode::FAILURE;
    }
    let isolation: &[&str] = if has(HARDER_FLAG) {
        &ISOLATION_ARGUMENTS
    } else {
        &[]
    };
    let launch = Launch {
        reload_hook: !has(NO_HOOK_FLAG),
        extra_arguments: PERMISSION_ARGUMENTS
            .iter()
            .chain(isolation)
            .map(OsString::from)
            .collect(),
    };
    let outcome = Path::new(wake)
        .canonicalize()
        .map_err(|error| format!("{wake}: {error}"))
        .and_then(|wake| {
            if has(HARDER_FLAG) {
                run_hard(
                    Path::new(results),
                    &wake,
                    &launch,
                    if has(TRIAL_FLAG) {
                        Kind::Trial
                    } else {
                        Kind::Recorded
                    },
                )
            } else {
                run_once(Path::new(results), &wake, &launch)
            }
        });
    match outcome {
        Ok(_) => ExitCode::SUCCESS,
        Err(message) => {
            let _ = writeln!(io::stderr().lock(), "resume-test: {message}");
            ExitCode::FAILURE
        }
    }
}
