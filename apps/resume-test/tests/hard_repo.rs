//! The snapshot and step evidence read from real git repositories.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use wake_resume_test::hard_judge::HardStep;
use wake_resume_test::hard_repo::{evidence, snapshot};
use wake_resume_test::repo::init;

mod support;

use support::git_command;

static NEXT: AtomicUsize = AtomicUsize::new(0);

const PROGRESS: &str = "---\nphase: build\nheadline: working\nupdated: 2026-11-25T10:00:00Z\n---\n\ndecision codename: marzipan\ndecision port: 24817\ndecision colour: 3fa9c2\ndecision motto: lantern quiet orchard\n";

fn scratch() -> PathBuf {
    let folder = std::env::temp_dir()
        .join(format!(
            "wake-hard-repo-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ))
        .join("repo");
    init(&folder).expect("repository starts");
    folder
}

fn git(folder: &Path, arguments: &[&str]) {
    let status = git_command()
        .arg("-C")
        .arg(folder)
        .args(arguments)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .status()
        .expect("git runs");
    assert!(status.success(), "git {arguments:?}");
}

fn write(folder: &Path, name: &str, text: &str) {
    let path = folder.join(name);
    fs::create_dir_all(path.parent().expect("a parent")).expect("folder");
    fs::write(path, text).expect("file");
}

fn finish_step(folder: &Path, number: u32, text: &str) {
    write(folder, &format!("steps/{number}.txt"), text);
    let ledger = fs::read_to_string(folder.join("LEDGER.md")).unwrap_or_default();
    write(
        folder,
        "LEDGER.md",
        &format!("{ledger}step {number} done\n"),
    );
    git(folder, &["add", "."]);
    git(
        folder,
        &["commit", "--quiet", "-m", &format!("step {number}")],
    );
}

fn four_steps(folder: &Path) {
    write(folder, ".wake/progress.md", PROGRESS);
    for number in 1..=4 {
        finish_step(folder, number, &format!("step {number}\n"));
    }
}

#[test]
fn choices_only_in_the_progress_file_leave_no_leak() {
    let folder = scratch();
    four_steps(&folder);
    let seen = snapshot(&folder);
    assert!(seen.leaks.is_empty(), "{:?}", seen.leaks);
    assert_eq!(seen.decisions.expect("choices")["port"], "24817");
}

#[test]
fn a_choice_in_a_step_file_an_untracked_file_or_a_commit_message_is_a_leak() {
    let folder = scratch();
    write(&folder, ".wake/progress.md", PROGRESS);
    finish_step(&folder, 1, "step 1\ncodename=marzipan\n");
    write(&folder, "notes.txt", "colour 3fa9c2\n");
    write(&folder, "steps/2.txt", "step 2\n");
    git(&folder, &["add", "steps/2.txt"]);
    git(&folder, &["commit", "--quiet", "-m", "step 2, port 24817"]);
    let leaks = snapshot(&folder).leaks;
    for expected in [
        "codename in steps/1.txt",
        "colour in notes.txt",
        "port in a commit message",
    ] {
        assert!(
            leaks.iter().any(|leak| leak == expected),
            "{expected}: {leaks:?}"
        );
    }
}

#[test]
fn an_invalid_progress_file_gives_problems_and_no_leak_search() {
    let folder = scratch();
    write(&folder, ".wake/progress.md", "decision port: 24817\n");
    let seen = snapshot(&folder);
    assert!(seen.decisions.is_err());
    assert_eq!(seen.leaks.len(), 0, "nothing to search for");
}

#[test]
fn steps_after_compaction_are_correct_only_with_the_earlier_choices() {
    let folder = scratch();
    four_steps(&folder);
    let decisions = snapshot(&folder).decisions.expect("choices");
    finish_step(&folder, 5, "step 5\ncodename=marzipan\n");
    finish_step(&folder, 6, "step 6\nport=24818\n");
    let steps = evidence(&folder, &decisions).expect("evidence");
    assert!(steps[..5].iter().all(HardStep::finished), "{steps:?}");
    assert!(steps[5].wrong_choice(), "{:?}", steps[5]);
    assert!(!steps[5].finished());
    assert!(!steps[6].file_present, "step 7 was not written");
}

#[test]
fn a_recommitted_step_counts_as_redone() {
    let folder = scratch();
    four_steps(&folder);
    git(&folder, &["commit", "--quiet", "--amend", "-m", "step 4"]);
    let decisions = snapshot(&folder).decisions.expect("choices");
    let steps = evidence(&folder, &decisions).expect("evidence");
    assert!(steps[3].redone(), "{:?}", steps[3]);
}
