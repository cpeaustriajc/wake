//! Polls a cloud task's branch for its progress file and saves each read as JSON.
//!
//! Arguments, in order: repository directory, remote, branch, seconds between
//! reads, timeout in seconds, distinct commits wanted, output file, session id,
//! start time (RFC 3339) and the `claude` command line that started the task.

use std::collections::HashSet;
use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use wake::branch::{BranchError, BranchProgress, read_progress};

struct Read {
    at: OffsetDateTime,
    progress: BranchProgress,
}

struct Settings {
    repo: PathBuf,
    remote: String,
    branch: String,
    interval: Duration,
    timeout: Duration,
    commits_wanted: usize,
    output: PathBuf,
    session: String,
    started: String,
    command: String,
}

fn settings() -> Result<Settings, Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [
        repo,
        remote,
        branch,
        interval,
        timeout,
        commits_wanted,
        output,
        session,
        started,
        command,
    ] = <[String; 10]>::try_from(args)
        .map_err(|_| "expected 10 arguments; see the doc comment at the top of the program")?;
    Ok(Settings {
        repo: repo.into(),
        remote,
        branch,
        interval: Duration::from_secs(interval.parse()?),
        timeout: Duration::from_secs(timeout.parse()?),
        commits_wanted: commits_wanted.parse()?,
        output: output.into(),
        session,
        started,
        command,
    })
}

fn now() -> Result<OffsetDateTime, Box<dyn Error>> {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    Ok(OffsetDateTime::from_unix_timestamp(i64::try_from(
        seconds,
    )?)?)
}

fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other if u32::from(other) < 0x20 => {
                write!(out, "\\u{:04x}", u32::from(other)).ok();
            }
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn render(settings: &Settings, reads: &[Read]) -> Result<String, Box<dyn Error>> {
    let mut out = String::new();
    writeln!(out, "{{")?;
    writeln!(out, "  \"session\": {},", quote(&settings.session))?;
    writeln!(out, "  \"branch\": {},", quote(&settings.branch))?;
    writeln!(out, "  \"started\": {},", quote(&settings.started))?;
    writeln!(out, "  \"command\": {},", quote(&settings.command))?;
    writeln!(out, "  \"reads\": [")?;
    for (index, read) in reads.iter().enumerate() {
        let separator = if index + 1 == reads.len() { "" } else { "," };
        writeln!(
            out,
            "    {{\"read_at\": {}, \"commit\": {}, \"phase\": {}, \"headline\": {}, \"updated\": {}}}{separator}",
            quote(&read.at.format(&Rfc3339)?),
            quote(&read.progress.commit),
            quote(&read.progress.phase),
            quote(&read.progress.headline),
            quote(&read.progress.updated.format(&Rfc3339)?),
        )?;
    }
    writeln!(out, "  ]")?;
    writeln!(out, "}}")?;
    Ok(out)
}

fn main() -> Result<(), Box<dyn Error>> {
    let settings = settings()?;
    let mut stderr = io::stderr();
    let mut reads: Vec<Read> = Vec::new();
    let mut commits: HashSet<String> = HashSet::new();
    let begun = Instant::now();
    while begun.elapsed() < settings.timeout && commits.len() < settings.commits_wanted {
        let at = now()?;
        match read_progress(&settings.repo, &settings.remote, &settings.branch) {
            Ok(progress) => {
                writeln!(
                    stderr,
                    "{} {} {} | {}",
                    at.format(&Rfc3339)?,
                    &progress.commit[..progress.commit.len().min(10)],
                    progress.phase,
                    progress.headline
                )?;
                commits.insert(progress.commit.clone());
                reads.push(Read { at, progress });
            }
            Err(error @ (BranchError::BranchMissing { .. } | BranchError::FileMissing { .. })) => {
                writeln!(stderr, "{} not yet: {error}", at.format(&Rfc3339)?)?;
            }
            Err(error) => return Err(error.into()),
        }
        if commits.len() < settings.commits_wanted {
            sleep(settings.interval);
        }
    }
    if reads.is_empty() {
        return Err("no read succeeded before the timeout; no result file written".into());
    }
    fs::write(&settings.output, render(&settings, &reads)?)?;
    writeln!(
        stderr,
        "{} reads, {} distinct commits, written to {}",
        reads.len(),
        commits.len(),
        settings.output.display()
    )?;
    Ok(())
}
