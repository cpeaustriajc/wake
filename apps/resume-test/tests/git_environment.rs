//! Git's repository variables must not move the scratch repository code to another repository.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use wake_resume_test::repo::{evidence, init};

mod support;

use support::git_command;

const INNER: &str = "builds_the_scratch_repository_while_the_variables_point_at_the_decoy";
const FOLDER_VARIABLE: &str = "WAKE_TEST_FOLDER";

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
fn builds_the_scratch_repository_while_the_variables_point_at_the_decoy() {
    let folder = PathBuf::from(std::env::var(FOLDER_VARIABLE).expect("folder path"));
    init(&folder).expect("scratch repository is created");
    evidence(&folder).expect("evidence is read");
    assert!(
        folder.join(".git").is_dir(),
        "the scratch repository has its own .git"
    );
    let parent = folder.parent().expect("parent folder");
    git(parent, &["init", "--bare", "scratch-remote.git"]);
    git(
        &folder,
        &["remote", "add", "origin", "../scratch-remote.git"],
    );
    fs::write(folder.join("CHANGE"), "change\n").expect("write change");
    git(&folder, &["add", "."]);
    git(&folder, &["commit", "-m", "change"]);
    git(&folder, &["push", "origin", "HEAD:refs/heads/pushed"]);
    let head = git(&folder, &["rev-parse", "HEAD"]);
    let listed = git(&folder, &["ls-remote", "origin", "refs/heads/pushed"]);
    assert!(
        listed.starts_with(&head),
        "the push reached the scratch remote"
    );
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

    let folder = root.join("scratch");
    let before_decoy = snapshot(&decoy);
    let before_remote = snapshot(&root.join("decoy-remote.git"));

    let output = Command::new(std::env::current_exe().expect("test binary"))
        .args(["--exact", INNER, "--ignored", "--test-threads=1"])
        .env("GIT_DIR", decoy.join(".git"))
        .env("GIT_WORK_TREE", &decoy)
        .env("GIT_INDEX_FILE", decoy.join(".git/index"))
        .env(FOLDER_VARIABLE, &folder)
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
