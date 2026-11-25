//! Git's repository variables must not move the branch reader to another repository.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use wake::branch::read_progress;

mod support;

use support::git_command;

const INNER: &str = "reads_the_clone_while_the_variables_point_at_the_decoy";
const CLONE_VARIABLE: &str = "WAKE_TEST_CLONE";
const COMMIT_VARIABLE: &str = "WAKE_TEST_COMMIT";
const PROGRESS: &str =
    "---\nphase: plan\nheadline: Reading the repo\nupdated: 2026-11-24T14:00:00Z\n---\nNotes\n";

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

fn snapshot(folder: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    let mut pending = vec![folder.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(&current).expect("read folder") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push((path.clone(), fs::read(&path).expect("read file")));
            }
        }
    }
    files.sort();
    files
}

#[test]
#[ignore = "runs inside the test below with the variables set"]
fn reads_the_clone_while_the_variables_point_at_the_decoy() {
    let clone = PathBuf::from(std::env::var(CLONE_VARIABLE).expect("clone path"));
    let expected = std::env::var(COMMIT_VARIABLE).expect("commit id");
    let progress = read_progress(&clone, "origin", "main").expect("progress is read");
    assert_eq!(progress.commit, expected, "the commit came from the clone");
}

#[test]
fn the_variables_leave_the_decoy_and_its_remote_unchanged() {
    let root = std::env::temp_dir().join(format!("wake-decoy-{}", std::process::id()));
    fs::remove_dir_all(&root).ok();
    fs::create_dir_all(&root).expect("create root");

    git(&root, &["init", "--bare", "decoy-remote.git"]);
    git(&root, &["clone", "decoy-remote.git", "decoy"]);
    let decoy = root.join("decoy");
    fs::write(decoy.join("DECOY"), "decoy\n").expect("write decoy file");
    git(&decoy, &["add", "."]);
    git(&decoy, &["commit", "-m", "decoy"]);
    git(&decoy, &["push", "origin", "HEAD:refs/heads/main"]);

    git(&root, &["init", "--bare", "remote.git"]);
    git(&root, &["clone", "remote.git", "worker"]);
    let worker = root.join("worker");
    fs::create_dir_all(worker.join(".wake")).expect("create dir");
    fs::write(worker.join(".wake/progress.md"), PROGRESS).expect("write progress");
    git(&worker, &["add", "."]);
    git(&worker, &["commit", "-m", "progress"]);
    git(&worker, &["push", "origin", "HEAD:refs/heads/main"]);
    let commit = git(&worker, &["rev-parse", "HEAD"]);
    git(&root, &["clone", "remote.git", "clone"]);

    let before_decoy = snapshot(&decoy);
    let before_remote = snapshot(&root.join("decoy-remote.git"));

    let output = Command::new(std::env::current_exe().expect("test binary"))
        .args(["--exact", INNER, "--ignored", "--test-threads=1"])
        .env("GIT_DIR", decoy.join(".git"))
        .env("GIT_WORK_TREE", &decoy)
        .env("GIT_INDEX_FILE", decoy.join(".git/index"))
        .env(CLONE_VARIABLE, root.join("clone"))
        .env(COMMIT_VARIABLE, &commit)
        .output()
        .expect("test binary runs");
    let report = String::from_utf8_lossy(&output.stdout).into_owned();

    assert!(output.status.success(), "inner test failed: {report}");
    assert!(
        report.contains("1 passed"),
        "inner test did not run: {report}"
    );
    assert_eq!(snapshot(&decoy), before_decoy, "the decoy changed");
    assert_eq!(
        snapshot(&root.join("decoy-remote.git")),
        before_remote,
        "the decoy's remote changed"
    );
    fs::remove_dir_all(&root).ok();
}
