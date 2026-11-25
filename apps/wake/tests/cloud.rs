//! `wake cloud track` and `wake agents` against a real daemon process and a
//! bare repository on this machine, so no login and no network are needed.

use std::fs;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

mod support;

use support::git_command;

static NEXT: AtomicUsize = AtomicUsize::new(0);

const POLL_MILLISECONDS: &str = "100";
const GIVE_UP: Duration = Duration::from_secs(30);

struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "wake-cloud-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&root).expect("create root");
        let root = root.canonicalize().expect("canonical root");
        let sandbox = Self { root };
        git(&sandbox.root, &["init", "--bare", "remote.git"]);
        git(&sandbox.root, &["clone", "remote.git", "worker"]);
        fs::write(sandbox.worker().join("README"), "hello\n").expect("write readme");
        git(&sandbox.worker(), &["add", "."]);
        git(&sandbox.worker(), &["commit", "-m", "first"]);
        git(
            &sandbox.worker(),
            &["push", "origin", "HEAD:refs/heads/main"],
        );
        git(&sandbox.root, &["clone", "remote.git", "clone"]);
        sandbox
    }

    fn worker(&self) -> PathBuf {
        self.root.join("worker")
    }

    fn clone_folder(&self) -> PathBuf {
        self.root.join("clone")
    }

    fn push(&self, branch: &str, phase: &str, headline: &str) {
        let worker = self.worker();
        if git_succeeds(&worker, &["rev-parse", "--verify", branch]) {
            git(&worker, &["checkout", "--quiet", branch]);
        } else {
            git(&worker, &["checkout", "--quiet", "-B", branch, "main"]);
        }
        fs::create_dir_all(worker.join(".wake")).expect("progress folder");
        fs::write(worker.join(".wake/progress.md"), progress(phase, headline))
            .expect("progress file");
        git(&worker, &["add", "."]);
        git(&worker, &["commit", "-m", headline]);
        git(
            &worker,
            &["push", "origin", &format!("HEAD:refs/heads/{branch}")],
        );
    }

    fn local_agent(&self, name: &str, headline: &str) -> PathBuf {
        let folder = self.root.join(name);
        fs::create_dir_all(folder.join(".wake")).expect("agent folder");
        fs::write(folder.join(".wake/progress.md"), progress("plan", headline))
            .expect("progress file");
        folder
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).ok();
    }
}

fn progress(phase: &str, headline: &str) -> String {
    format!(
        "---\nphase: {phase}\nheadline: {headline}\nupdated: 2026-11-25T10:00:00Z\n---\nNotes\n"
    )
}

fn command(dir: &Path, args: &[&str]) -> Output {
    git_command()
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
        .expect("git runs")
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = command(dir, args);
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn git_succeeds(dir: &Path, args: &[&str]) -> bool {
    command(dir, args).status.success()
}

fn wake(sandbox: &Sandbox, in_folder: &Path, arguments: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_wake"));
    command
        .args(arguments)
        .current_dir(in_folder)
        .env("WAKE_SOCKET", sandbox.root.join("d.sock"))
        .env("WAKE_DB", sandbox.root.join("d.db"))
        .env("WAKE_CLOUD_POLL_MS", POLL_MILLISECONDS)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

struct Daemon(Child);

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn start_daemon(sandbox: &Sandbox) -> Daemon {
    let daemon = Daemon(
        wake(sandbox, &sandbox.root, &["daemon"])
            .spawn()
            .expect("daemon starts"),
    );
    for _ in 0..200 {
        if UnixStream::connect(sandbox.root.join("d.sock")).is_ok() {
            return daemon;
        }
        thread::sleep(Duration::from_millis(25));
    }
    panic!("the daemon never listened");
}

fn track(sandbox: &Sandbox, branch: &str) -> Output {
    wake(
        sandbox,
        &sandbox.clone_folder(),
        &["cloud", "track", branch],
    )
    .output()
    .expect("wake runs")
}

fn agents(sandbox: &Sandbox) -> Vec<String> {
    let output = wake(sandbox, &sandbox.root, &["agents"])
        .output()
        .expect("wake runs");
    assert!(
        output.status.success(),
        "wake agents: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

fn wait_for_line(sandbox: &Sandbox, wanted: &str) -> Vec<String> {
    let started = Instant::now();
    loop {
        let lines = agents(sandbox);
        if lines.iter().any(|line| line.contains(wanted)) {
            return lines;
        }
        assert!(
            started.elapsed() < GIVE_UP,
            "no line with `{wanted}` appeared: {lines:?}"
        );
        thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn a_tracked_branch_lists_as_cloud_with_headline_phase_and_update_next_to_a_local_agent() {
    let sandbox = Sandbox::new();
    sandbox.push("task-a", "build", "Writing the parser");
    let _daemon = start_daemon(&sandbox);
    let local = sandbox.local_agent("local-agent", "Local work");
    let registered = wake(
        &sandbox,
        &sandbox.root,
        &["agent", "start", &local.to_string_lossy()],
    )
    .env("WAKE_CLAUDE", "/usr/bin/true")
    .output()
    .expect("wake runs");
    assert!(
        registered.status.success(),
        "agent start: {}",
        String::from_utf8_lossy(&registered.stderr)
    );

    let tracked = track(&sandbox, "task-a");
    assert!(
        tracked.status.success(),
        "cloud track: {}",
        String::from_utf8_lossy(&tracked.stderr)
    );
    let lines = agents(&sandbox);

    assert_eq!(lines.len(), 2, "one line per agent: {lines:?}");
    assert!(
        lines[0].contains("Local work") && lines[0].contains("local"),
        "the local agent: {}",
        lines[0]
    );
    for wanted in [
        "Writing the parser",
        "build",
        "2026-11-25T10:00:00Z",
        "cloud",
        "task-a",
    ] {
        assert!(
            lines[1].contains(wanted),
            "`{wanted}` is missing from the cloud line: {}",
            lines[1]
        );
    }
}

#[test]
fn a_new_commit_on_the_remote_shows_in_wake_agents_and_the_clone_stays_as_it_was() {
    let sandbox = Sandbox::new();
    sandbox.push("task-a", "build", "Writing the parser");
    let _daemon = start_daemon(&sandbox);
    assert!(track(&sandbox, "task-a").status.success(), "tracked");
    let clone = sandbox.clone_folder();
    let before = (
        git(&clone, &["status", "--porcelain"]),
        git(&clone, &["branch", "--show-current"]),
        git(&clone, &["rev-parse", "HEAD"]),
        git(&clone, &["for-each-ref", "refs/heads"]),
    );

    sandbox.push("task-a", "review", "Parser done");
    let lines = wait_for_line(&sandbox, "Parser done");

    assert!(lines[0].contains("review"), "the new phase: {}", lines[0]);
    assert!(
        !lines[0].contains("Writing the parser"),
        "the old headline is gone: {}",
        lines[0]
    );
    assert_eq!(
        before,
        (
            git(&clone, &["status", "--porcelain"]),
            git(&clone, &["branch", "--show-current"]),
            git(&clone, &["rev-parse", "HEAD"]),
            git(&clone, &["for-each-ref", "refs/heads"]),
        ),
        "status, current branch, head and local branches are unchanged"
    );
}

#[test]
fn a_branch_that_is_not_pushed_yet_shows_a_label_and_then_its_headline() {
    let sandbox = Sandbox::new();
    sandbox.push("other", "plan", "Another task");
    let _daemon = start_daemon(&sandbox);
    assert!(track(&sandbox, "other").status.success(), "tracked other");
    assert!(track(&sandbox, "task-a").status.success(), "tracked task-a");

    let lines = agents(&sandbox);
    assert_eq!(lines.len(), 2, "every entry lists: {lines:?}");
    assert!(
        lines[0].contains("Another task"),
        "the other task: {}",
        lines[0]
    );
    assert!(
        lines[1].contains("(branch not pushed yet)") && lines[1].contains("task-a"),
        "a plain label: {}",
        lines[1]
    );

    sandbox.push("task-a", "plan", "Reading the repo");
    wait_for_line(&sandbox, "Reading the repo");
}

#[test]
fn the_tracked_branches_are_the_same_after_the_daemon_restarts() {
    let sandbox = Sandbox::new();
    sandbox.push("task-a", "build", "Writing the parser");
    let daemon = start_daemon(&sandbox);
    assert!(track(&sandbox, "task-a").status.success(), "tracked");
    let before = agents(&sandbox);
    drop(daemon);

    let _restarted = start_daemon(&sandbox);
    let after = wait_for_line(&sandbox, "Writing the parser");

    assert_eq!(after, before, "the same list after the restart");
}

#[test]
fn a_branch_name_no_branch_can_have_is_refused_with_one_line() {
    let sandbox = Sandbox::new();
    let _daemon = start_daemon(&sandbox);

    let output = track(&sandbox, "two words");

    assert!(!output.status.success(), "refused");
    let message = String::from_utf8_lossy(&output.stderr);
    assert_eq!(message.lines().count(), 1, "one line: {message}");
    assert!(message.contains("not a branch name"), "says why: {message}");
    assert!(agents(&sandbox).is_empty(), "nothing was tracked");
}

#[test]
fn with_the_daemon_down_tracking_says_so_in_one_line_and_fails() {
    let sandbox = Sandbox::new();

    let output = track(&sandbox, "task-a");

    assert!(!output.status.success(), "failed");
    let message = String::from_utf8_lossy(&output.stderr);
    assert_eq!(message.lines().count(), 1, "one line: {message}");
    assert!(message.contains("the daemon is not running"), "{message}");
}
