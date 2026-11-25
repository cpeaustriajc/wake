//! Tests for reading a progress file from a branch.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use wake::branch::{BranchError, read_progress};

mod support;

use support::git_command;

static NEXT: AtomicUsize = AtomicUsize::new(0);

const FIRST: &str =
    "---\nphase: plan\nheadline: Reading the repo\nupdated: 2026-11-24T14:00:00Z\n---\nNotes\n";
const SECOND: &str =
    "---\nphase: build\nheadline: Writing the parser\nupdated: 2026-11-24T14:01:00Z\n---\nNotes\n";

struct Sandbox {
    root: PathBuf,
    clone: PathBuf,
    worker: PathBuf,
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).ok();
    }
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = git_command()
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com"])
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn sandbox() -> Sandbox {
    let root = std::env::temp_dir().join(format!(
        "wake-branch-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let clone = root.join("clone");
    let worker = root.join("worker");
    fs::create_dir_all(&root).expect("create root");
    git(&root, &["init", "--bare", "remote.git"]);
    git(&root, &["clone", "remote.git", "worker"]);
    fs::write(worker.join("README"), "hello\n").expect("write readme");
    git(&worker, &["add", "."]);
    git(&worker, &["commit", "-m", "first"]);
    git(&worker, &["push", "origin", "HEAD:refs/heads/main"]);
    git(&root, &["clone", "remote.git", "clone"]);
    Sandbox {
        root,
        clone,
        worker,
    }
}

fn push_progress(sandbox: &Sandbox, branch: &str, text: &str) -> String {
    let worker = &sandbox.worker;
    let exists = git_command()
        .arg("-C")
        .arg(worker)
        .args([
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .output()
        .expect("git runs")
        .status
        .success();
    if exists {
        git(worker, &["checkout", branch]);
    } else {
        git(worker, &["checkout", "-b", branch]);
    }
    fs::create_dir_all(worker.join(".wake")).expect("create dir");
    fs::write(worker.join(".wake/progress.md"), text).expect("write progress");
    git(worker, &["add", "."]);
    git(worker, &["commit", "-m", "progress"]);
    git(worker, &["push", "origin", &format!("{branch}:{branch}")]);
    git(worker, &["rev-parse", "HEAD"])
}

#[test]
fn reads_phase_headline_and_commit() {
    let sandbox = sandbox();
    let commit = push_progress(&sandbox, "task", FIRST);
    let progress = read_progress(&sandbox.clone, "origin", "task").expect("readable");
    assert_eq!(progress.commit, commit, "commit");
    assert_eq!(progress.phase, "plan", "phase");
    assert_eq!(progress.headline, "Reading the repo", "headline");
    assert_eq!(progress.updated.minute(), 0, "updated minute");
    assert_eq!(progress.updated.hour(), 14, "updated hour");
}

#[test]
fn sees_the_file_change_between_two_reads() {
    let sandbox = sandbox();
    let first_commit = push_progress(&sandbox, "task", FIRST);
    let first = read_progress(&sandbox.clone, "origin", "task").expect("first read");
    let second_commit = push_progress(&sandbox, "task", SECOND);
    let second = read_progress(&sandbox.clone, "origin", "task").expect("second read");
    assert_eq!(first.commit, first_commit, "first commit");
    assert_eq!(second.commit, second_commit, "second commit");
    assert_ne!(first.commit, second.commit, "commits differ");
    assert_eq!(second.phase, "build", "second phase");
    assert_eq!(second.headline, "Writing the parser", "second headline");
    assert!(second.updated > first.updated, "updated moves forward");
}

#[test]
fn leaves_the_working_tree_alone() {
    let sandbox = sandbox();
    push_progress(&sandbox, "task", FIRST);
    read_progress(&sandbox.clone, "origin", "task").expect("readable");
    assert!(
        !sandbox.clone.join(".wake").exists(),
        "nothing is checked out"
    );
    assert_eq!(
        git(&sandbox.clone, &["branch", "--list"]),
        "* main",
        "no local branch is made"
    );
}

#[test]
fn reports_a_missing_branch() {
    let sandbox = sandbox();
    let error = read_progress(&sandbox.clone, "origin", "nope").expect_err("no such branch");
    assert!(
        matches!(&error, BranchError::BranchMissing { branch } if branch == "nope"),
        "got {error:?}"
    );
}

#[test]
fn reports_a_missing_file() {
    let sandbox = sandbox();
    let error = read_progress(&sandbox.clone, "origin", "main").expect_err("no file on main");
    assert!(
        matches!(&error, BranchError::FileMissing { branch, .. } if branch == "main"),
        "got {error:?}"
    );
}

#[test]
fn reports_an_invalid_file() {
    let sandbox = sandbox();
    push_progress(&sandbox, "task", "not a progress file\n");
    let error = read_progress(&sandbox.clone, "origin", "task").expect_err("invalid file");
    assert!(
        matches!(&error, BranchError::FileInvalid { .. }),
        "got {error:?}"
    );
}

#[test]
fn reports_git_failing() {
    let sandbox = sandbox();
    let error = read_progress(&sandbox.clone, "no-such-remote", "task").expect_err("bad remote");
    assert!(
        matches!(&error, BranchError::GitFailed { .. }),
        "got {error:?}"
    );
    let outside = sandbox.root.join("not-a-repo");
    fs::create_dir_all(&outside).expect("create dir");
    let error = read_progress(&outside, "origin", "task").expect_err("not a repo");
    assert!(
        matches!(&error, BranchError::GitFailed { .. }),
        "got {error:?}"
    );
}
