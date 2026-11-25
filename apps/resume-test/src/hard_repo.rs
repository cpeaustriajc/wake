//! Reading the harder task's evidence and the pre-compaction snapshot from the
//! scratch repository.

use std::fs;
use std::path::Path;

use crate::hard_judge::{HardStep, Snapshot, leaks};
use crate::hard_task::{
    Decisions, PROGRESS_FILE, STEP_COUNT, commit_subject, content_matches, ledger_line,
    parse_decisions, step_file,
};
use crate::repo::{commit_events, git, lines};
use crate::task::LEDGER;

fn files_below(folder: &Path, root: &Path, found: &mut Vec<(String, String)>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.file_name().is_some_and(|name| name == ".git") {
            continue;
        }
        if path.is_dir() {
            files_below(&path, root, found);
        } else if let Ok(text) = fs::read_to_string(&path) {
            let name = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            if name != PROGRESS_FILE {
                found.push((name, text));
            }
        }
    }
}

fn places(folder: &Path) -> Vec<(String, String)> {
    let mut found = Vec::new();
    files_below(folder, folder, &mut found);
    found.push((
        "a commit message".to_owned(),
        git(folder, &["log", "--all", "--format=%B"]).unwrap_or_default(),
    ));
    found
}

/// Reads the progress file and the choices in it from the working tree, and
/// finds where else the choices appear: in any other text file of the working
/// tree, tracked or not, or in any commit message.
#[must_use]
pub fn snapshot(folder: &Path) -> Snapshot {
    let progress = fs::read_to_string(folder.join(PROGRESS_FILE)).ok();
    let decisions = parse_decisions(progress.as_deref().unwrap_or_default());
    let leaks = decisions.as_ref().map_or_else(
        |_| Vec::new(),
        |decisions| leaks(decisions, &places(folder)),
    );
    Snapshot {
        progress,
        decisions,
        leaks,
    }
}

/// Builds each step's evidence from what the repository holds, judged against
/// `decisions`.
#[must_use]
pub fn evidence_from(
    decisions: &Decisions,
    subjects: &[String],
    reflog: &[String],
    ledger: &str,
    committed_file: impl Fn(&str) -> Option<String>,
) -> Vec<HardStep> {
    (1..=STEP_COUNT)
        .map(|number| {
            let subject = commit_subject(number);
            let revert = format!("Revert \"{subject}\"");
            let line = ledger_line(number);
            let file = committed_file(&step_file(number));
            HardStep {
                number,
                file_present: file.is_some(),
                file_correct: file.is_some_and(|text| content_matches(number, &text, decisions)),
                ledger_lines: ledger.lines().filter(|text| text.trim() == line).count(),
                commits: subjects.iter().filter(|text| **text == subject).count(),
                commit_events: commit_events(reflog, &subject),
                reverted: subjects.contains(&revert),
            }
        })
        .collect()
}

/// Reads every step's evidence from the repository in `folder`, judged against
/// `decisions`.
///
/// # Errors
///
/// Returns what went wrong if a git command fails.
pub fn evidence(folder: &Path, decisions: &Decisions) -> Result<Vec<HardStep>, String> {
    let subjects = lines(&git(folder, &["log", "--format=%s"])?);
    let reflog = lines(&git(folder, &["reflog", "--format=%gs"])?);
    let ledger = git(folder, &["show", &format!("HEAD:{LEDGER}")]).unwrap_or_default();
    Ok(evidence_from(
        decisions,
        &subjects,
        &reflog,
        &ledger,
        |path| git(folder, &["show", &format!("HEAD:{path}")]).ok(),
    ))
}
