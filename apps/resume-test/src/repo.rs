//! The scratch repository: creating it and reading each step's evidence.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::judge::StepEvidence;
use crate::task::{LEDGER, STEP_COUNT, commit_subject, ledger_line, step_content, step_file};

const NULL_CONFIG: &str = "/dev/null";
const REPOSITORY_VARIABLES: [&str; 15] = [
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CONFIG",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
    "GIT_OBJECT_DIRECTORY",
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_GRAFT_FILE",
    "GIT_INDEX_FILE",
    "GIT_NO_REPLACE_OBJECTS",
    "GIT_REPLACE_REF_BASE",
    "GIT_PREFIX",
    "GIT_SHALLOW_FILE",
    "GIT_COMMON_DIR",
];

pub(crate) fn git(folder: &Path, arguments: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    for variable in REPOSITORY_VARIABLES {
        command.env_remove(variable);
    }
    let output = command
        .arg("-C")
        .arg(folder)
        .args(arguments)
        .env("GIT_CONFIG_GLOBAL", NULL_CONFIG)
        .env("GIT_CONFIG_SYSTEM", NULL_CONFIG)
        .output()
        .map_err(|error| format!("git cannot start: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!(
            "git {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Creates the scratch repository in `folder` with one starting commit.
///
/// # Errors
///
/// Returns what went wrong if the folder or any git command fails.
pub fn init(folder: &Path) -> Result<(), String> {
    fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    git(folder, &["init", "--quiet", "-b", "main"])?;
    git(folder, &["config", "user.name", "Wake resume test"])?;
    git(
        folder,
        &["config", "user.email", "resume-test@example.invalid"],
    )?;
    git(folder, &["config", "commit.gpgsign", "false"])?;
    fs::write(folder.join("README.md"), "Scratch repository.\n")
        .map_err(|error| error.to_string())?;
    git(folder, &["add", "README.md"])?;
    git(folder, &["commit", "--quiet", "-m", "start"])?;
    Ok(())
}

/// How many reflog entries record `subject` being committed, amended or
/// recommitted. Each entry is a reflog message such as `commit: step 1`.
#[must_use]
pub fn commit_events(reflog: &[String], subject: &str) -> usize {
    reflog
        .iter()
        .filter(|entry| {
            entry
                .strip_prefix("commit")
                .and_then(|rest| rest.split_once(": "))
                .is_some_and(|(_, message)| message == subject)
        })
        .count()
}

pub(crate) fn lines(text: &str) -> Vec<String> {
    text.lines().map(str::to_owned).collect()
}

/// Builds each step's evidence from what the repository holds.
#[must_use]
pub fn evidence_from(
    subjects: &[String],
    reflog: &[String],
    ledger: &str,
    committed_file: impl Fn(&str) -> Option<String>,
) -> Vec<StepEvidence> {
    (1..=STEP_COUNT)
        .map(|number| {
            let subject = commit_subject(number);
            let revert = format!("Revert \"{subject}\"");
            let line = ledger_line(number);
            StepEvidence {
                number,
                file_committed: committed_file(&step_file(number))
                    .is_some_and(|text| text.trim() == step_content(number)),
                ledger_lines: ledger.lines().filter(|text| text.trim() == line).count(),
                commits: subjects.iter().filter(|text| **text == subject).count(),
                commit_events: commit_events(reflog, &subject),
                reverted: subjects.contains(&revert),
            }
        })
        .collect()
}

/// Reads every step's evidence from the repository in `folder`, from what is
/// committed in `HEAD` and what the reflog remembers.
///
/// # Errors
///
/// Returns what went wrong if a git command fails.
pub fn evidence(folder: &Path) -> Result<Vec<StepEvidence>, String> {
    let subjects = lines(&git(folder, &["log", "--format=%s"])?);
    let reflog = lines(&git(folder, &["reflog", "--format=%gs"])?);
    let ledger = git(folder, &["show", &format!("HEAD:{LEDGER}")]).unwrap_or_default();
    Ok(evidence_from(&subjects, &reflog, &ledger, |path| {
        git(folder, &["show", &format!("HEAD:{path}")]).ok()
    }))
}
