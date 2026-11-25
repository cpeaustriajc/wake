//! Reading step evidence from real git repositories.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use wake_resume_test::repo::{commit_events, evidence, init};
use wake_resume_test::task::{ledger_line, step_content, step_file};

mod support;

use support::git_command;

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn scratch() -> PathBuf {
    std::env::temp_dir()
        .join(format!(
            "wake-resume-repo-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ))
        .join("repo")
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

fn finish(folder: &Path, number: u32) {
    fs::create_dir_all(folder.join("steps")).expect("steps folder");
    fs::write(
        folder.join(step_file(number)),
        format!("{}\n", step_content(number)),
    )
    .expect("step file");
    let ledger = folder.join("LEDGER.md");
    let mut text = fs::read_to_string(&ledger).unwrap_or_default();
    text.push_str(&ledger_line(number));
    text.push('\n');
    fs::write(&ledger, text).expect("ledger");
    git(folder, &["add", "-A"]);
    git(
        folder,
        &["commit", "--quiet", "-m", &format!("step {number}")],
    );
}

#[test]
fn a_clean_run_shows_each_finished_step_once() {
    let folder = scratch();
    init(&folder).expect("init");
    finish(&folder, 1);
    finish(&folder, 2);
    let steps = evidence(&folder).expect("evidence");
    assert!(steps[0].finished() && steps[1].finished(), "steps 1 and 2");
    assert!(!steps[2].finished(), "step 3 not started");
    assert!(steps.iter().all(|step| !step.redone()), "nothing redone");
}

#[test]
fn a_step_reset_and_done_again_is_redone_even_though_history_shows_one_commit() {
    let folder = scratch();
    init(&folder).expect("init");
    finish(&folder, 1);
    git(&folder, &["reset", "--quiet", "--hard", "HEAD~1"]);
    finish(&folder, 1);
    let steps = evidence(&folder).expect("evidence");
    assert_eq!(steps[0].commits, 1, "history shows one commit");
    assert!(steps[0].redone(), "the reflog shows two");
}

#[test]
fn a_reverted_step_is_not_finished() {
    let folder = scratch();
    init(&folder).expect("init");
    finish(&folder, 1);
    git(&folder, &["revert", "--no-edit", "HEAD"]);
    let steps = evidence(&folder).expect("evidence");
    assert!(steps[0].reverted, "revert seen");
    assert!(!steps[0].finished(), "not finished");
}

#[test]
fn uncommitted_work_is_not_finished() {
    let folder = scratch();
    init(&folder).expect("init");
    fs::create_dir_all(folder.join("steps")).expect("steps folder");
    fs::write(folder.join(step_file(1)), "step 1\n").expect("file");
    let steps = evidence(&folder).expect("evidence");
    assert!(!steps[0].finished(), "not committed");
}

#[test]
fn counts_commit_amend_and_ignores_other_reflog_entries() {
    let reflog = [
        "commit: step 1".to_owned(),
        "commit (amend): step 1".to_owned(),
        "reset: moving to HEAD~1".to_owned(),
        "commit: step 2".to_owned(),
        "commit (initial): start".to_owned(),
    ];
    assert_eq!(commit_events(&reflog, "step 1"), 2, "step 1");
    assert_eq!(commit_events(&reflog, "step 2"), 1, "step 2");
    assert_eq!(commit_events(&reflog, "step 3"), 0, "step 3");
}
