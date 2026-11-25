//! Cloud tasks in the list of agents, against a bare repository on this
//! machine standing in for GitHub, over a real Unix socket and SQLite file.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use wake_daemon::protocol::{AgentEntry, AgentProgress, Location};
use wake_daemon::{Store, bind, list_agents, register_agent, serve_polling, track_cloud_task};

mod support;

use support::git_command;

static NEXT: AtomicUsize = AtomicUsize::new(0);

const POLL: Duration = Duration::from_millis(100);
const NEVER_AGAIN: Duration = Duration::from_secs(3600);
const GIVE_UP: Duration = Duration::from_secs(30);

struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "wake-cloud-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&root).expect("create root");
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

    fn socket(&self, name: &str) -> PathBuf {
        self.root.join(format!("{name}.sock"))
    }

    fn database(&self) -> PathBuf {
        self.root.join("d.db")
    }

    fn push(&self, branch: &str, phase: &str, headline: &str) {
        let worker = self.worker();
        if git_succeeds(&worker, &["rev-parse", "--verify", branch]) {
            git(&worker, &["checkout", "--quiet", branch]);
        } else {
            git(&worker, &["checkout", "--quiet", "-B", branch, "main"]);
        }
        fs::create_dir_all(worker.join(".wake")).expect("progress folder");
        write_progress(&worker, phase, headline);
        git(&worker, &["add", "."]);
        git(&worker, &["commit", "-m", headline]);
        git(
            &worker,
            &["push", "origin", &format!("HEAD:refs/heads/{branch}")],
        );
    }

    fn push_without_progress_file(&self, branch: &str) {
        let worker = self.worker();
        git(&worker, &["checkout", "--quiet", "-B", branch, "main"]);
        fs::write(worker.join("notes"), "no progress here\n").expect("notes");
        git(&worker, &["add", "."]);
        git(&worker, &["commit", "-m", "notes"]);
        git(
            &worker,
            &["push", "origin", &format!("HEAD:refs/heads/{branch}")],
        );
    }

    fn push_invalid_progress_file(&self, branch: &str) {
        let worker = self.worker();
        git(&worker, &["checkout", "--quiet", "-B", branch, "main"]);
        fs::create_dir_all(worker.join(".wake")).expect("progress folder");
        fs::write(
            worker.join(".wake/progress.md"),
            "headline: no front matter\n",
        )
        .expect("invalid file");
        git(&worker, &["add", "."]);
        git(&worker, &["commit", "-m", "invalid"]);
        git(
            &worker,
            &["push", "origin", &format!("HEAD:refs/heads/{branch}")],
        );
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).ok();
    }
}

fn write_progress(folder: &Path, phase: &str, headline: &str) {
    let text = format!(
        "---\nphase: {phase}\nheadline: {headline}\nupdated: 2026-11-25T10:00:00Z\n---\nNotes\n"
    );
    fs::write(folder.join(".wake/progress.md"), text).expect("progress file");
}

fn command(dir: &Path, args: &[&str]) -> std::process::Output {
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

fn start(socket: &Path, database: &Path) {
    start_polling(socket, database, POLL);
}

fn start_polling(socket: &Path, database: &Path, interval: Duration) {
    let store = Store::open(database).expect("database opens");
    let listener = bind(socket).expect("socket binds");
    thread::spawn(move || {
        let _ = serve_polling(&listener, store, interval);
    });
}

fn folder_text(folder: &Path) -> String {
    folder.to_string_lossy().into_owned()
}

fn wait_for(socket: &Path, ready: impl Fn(&[AgentEntry]) -> bool) -> Vec<AgentEntry> {
    let started = Instant::now();
    loop {
        let agents = list_agents(socket).expect("listed");
        if ready(&agents) {
            return agents;
        }
        assert!(
            started.elapsed() < GIVE_UP,
            "the list never became ready: {agents:?}"
        );
        thread::sleep(Duration::from_millis(50));
    }
}

fn found(headline: &str, phase: &str) -> AgentProgress {
    AgentProgress::Found {
        headline: headline.to_owned(),
        phase: phase.to_owned(),
        updated: "2026-11-25T10:00:00Z".to_owned(),
    }
}

fn clone_state(clone: &Path) -> String {
    let mut files = Vec::new();
    let mut pending = vec![clone.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(&current).expect("read folder") {
            let path = entry.expect("entry").path();
            if path.file_name().is_some_and(|name| name == ".git") {
                continue;
            }
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(format!(
                    "{} {:?}",
                    path.display(),
                    fs::read(&path).expect("read")
                ));
            }
        }
    }
    files.sort();
    format!(
        "status:\n{}\nbranch: {}\nhead: {}\nbranches:\n{}\nfiles:\n{}",
        git(clone, &["status", "--porcelain"]),
        git(clone, &["branch", "--show-current"]),
        git(clone, &["rev-parse", "HEAD"]),
        git(clone, &["for-each-ref", "refs/heads"]),
        files.join("\n")
    )
}

#[test]
fn a_tracked_branch_lists_as_cloud_next_to_a_local_agent() {
    let sandbox = Sandbox::new();
    sandbox.push("task-a", "build", "Writing the parser");
    let socket = sandbox.socket("d");
    start(&socket, &sandbox.database());
    let local = sandbox.root.join("local-agent");
    fs::create_dir_all(local.join(".wake")).expect("local folder");
    write_progress(&local, "plan", "Local work");
    register_agent(&socket, &folder_text(&local)).expect("registered");

    let clone = folder_text(&sandbox.clone_folder());
    let id = track_cloud_task(&socket, &clone, "task-a").expect("tracked");
    let agents = list_agents(&socket).expect("listed");

    assert_eq!(
        agents.len(),
        2,
        "one local agent and one cloud task: {agents:?}"
    );
    assert_eq!(agents[0].location, Location::Local, "local agent first");
    assert_eq!(agents[0].branch, None, "a local agent has no branch");
    assert_eq!(agents[1].location, Location::Cloud, "then the cloud task");
    assert_eq!(agents[1].id, id, "the ID tracking returned");
    assert_eq!(agents[1].branch.as_deref(), Some("task-a"), "the branch");
    assert_eq!(agents[1].folder, clone, "the folder tracking ran in");
    assert_eq!(
        agents[1].progress,
        found("Writing the parser", "build"),
        "headline, phase and last update come from the branch"
    );
}

#[test]
fn tracking_the_same_branch_again_keeps_its_id_and_its_one_entry() {
    let sandbox = Sandbox::new();
    sandbox.push("task-a", "build", "Writing the parser");
    let socket = sandbox.socket("d");
    start(&socket, &sandbox.database());
    let clone = folder_text(&sandbox.clone_folder());

    let first = track_cloud_task(&socket, &clone, "task-a").expect("tracked");
    let second = track_cloud_task(&socket, &clone, "task-a").expect("tracked again");

    assert_eq!(first, second, "same ID");
    assert_eq!(list_agents(&socket).expect("listed").len(), 1, "one entry");
}

#[test]
fn a_branch_name_no_branch_can_have_is_refused() {
    let sandbox = Sandbox::new();
    let socket = sandbox.socket("d");
    start(&socket, &sandbox.database());
    let clone = folder_text(&sandbox.clone_folder());

    for name in ["", "two words", "a:b"] {
        let outcome = track_cloud_task(&socket, &clone, name);
        assert!(outcome.is_err(), "`{name}` must be refused: {outcome:?}");
    }
    assert!(
        list_agents(&socket).expect("listed").is_empty(),
        "nothing was tracked"
    );
}

#[test]
fn a_new_commit_on_the_remote_shows_after_the_next_read_and_the_clone_stays_as_it_was() {
    let sandbox = Sandbox::new();
    sandbox.push("task-a", "build", "Writing the parser");
    let socket = sandbox.socket("d");
    start(&socket, &sandbox.database());
    let clone = sandbox.clone_folder();
    fs::write(clone.join("scratch.txt"), "uncommitted\n").expect("untracked file");
    track_cloud_task(&socket, &folder_text(&clone), "task-a").expect("tracked");
    let before = clone_state(&clone);

    sandbox.push("task-a", "review", "Parser done");
    let agents = wait_for(&socket, |agents| {
        agents[0].progress == found("Parser done", "review")
    });

    assert_eq!(agents.len(), 1, "still one entry");
    assert_eq!(
        clone_state(&clone),
        before,
        "working tree, current branch and local branches are unchanged"
    );
}

#[test]
fn a_missing_or_unreadable_branch_gets_a_plain_label_and_the_rest_still_list() {
    let sandbox = Sandbox::new();
    sandbox.push("good", "build", "Writing the parser");
    sandbox.push_without_progress_file("no-file");
    sandbox.push_invalid_progress_file("bad-file");
    let socket = sandbox.socket("d");
    start(&socket, &sandbox.database());
    let clone = folder_text(&sandbox.clone_folder());
    let not_a_repository = sandbox.root.join("plain-folder");
    fs::create_dir_all(&not_a_repository).expect("plain folder");
    let local = sandbox.root.join("local-agent");
    fs::create_dir_all(local.join(".wake")).expect("local folder");
    write_progress(&local, "plan", "Local work");
    register_agent(&socket, &folder_text(&local)).expect("registered");
    for branch in ["good", "not-pushed-yet", "no-file", "bad-file"] {
        track_cloud_task(&socket, &clone, branch).expect("tracked");
    }
    track_cloud_task(&socket, &folder_text(&not_a_repository), "good").expect("tracked");

    let agents = list_agents(&socket).expect("listed");

    assert_eq!(agents.len(), 6, "every entry lists: {agents:?}");
    assert_eq!(
        agents[0].progress,
        found("Local work", "plan"),
        "the local agent"
    );
    assert_eq!(agents[1].progress, found("Writing the parser", "build"));
    assert_eq!(
        agents[2].progress,
        AgentProgress::BranchMissing,
        "no branch"
    );
    assert_eq!(agents[3].progress, AgentProgress::Missing, "no file");
    assert!(
        matches!(agents[4].progress, AgentProgress::Invalid { .. }),
        "invalid file: {:?}",
        agents[4].progress
    );
    assert!(
        matches!(agents[5].progress, AgentProgress::Unreachable { .. }),
        "a folder that is no repository: {:?}",
        agents[5].progress
    );
}

#[test]
fn a_remote_that_cannot_be_read_is_a_label_not_an_error() {
    let sandbox = Sandbox::new();
    sandbox.push("task-a", "build", "Writing the parser");
    let socket = sandbox.socket("d");
    start(&socket, &sandbox.database());
    let clone = sandbox.clone_folder();
    git(
        &clone,
        &["remote", "set-url", "origin", "/no/such/remote.git"],
    );

    track_cloud_task(&socket, &folder_text(&clone), "task-a").expect("tracked");
    let agents = list_agents(&socket).expect("listed");

    assert!(
        matches!(agents[0].progress, AgentProgress::Unreachable { .. }),
        "unreachable remote: {:?}",
        agents[0].progress
    );
}

#[test]
fn a_branch_pushed_later_turns_from_a_label_into_its_headline() {
    let sandbox = Sandbox::new();
    let socket = sandbox.socket("d");
    start(&socket, &sandbox.database());
    track_cloud_task(&socket, &folder_text(&sandbox.clone_folder()), "task-a").expect("tracked");
    assert_eq!(
        list_agents(&socket).expect("listed")[0].progress,
        AgentProgress::BranchMissing,
        "before the first push"
    );

    sandbox.push("task-a", "plan", "Reading the repo");
    let agents = wait_for(&socket, |agents| {
        agents[0].progress == found("Reading the repo", "plan")
    });

    assert_eq!(agents.len(), 1, "still one entry");
}

#[test]
fn two_branches_of_one_repository_are_never_mixed_up() {
    let sandbox = Sandbox::new();
    sandbox.push("task-a", "build", "Headline of a");
    sandbox.push("task-b", "review", "Headline of b");
    let socket = sandbox.socket("d");
    start(&socket, &sandbox.database());
    let clone = folder_text(&sandbox.clone_folder());
    track_cloud_task(&socket, &clone, "task-a").expect("tracked a");
    track_cloud_task(&socket, &clone, "task-b").expect("tracked b");

    for round in 0..60 {
        let agents = list_agents(&socket).expect("listed");
        assert_eq!(
            agents[0].progress,
            found("Headline of a", "build"),
            "round {round}: branch a"
        );
        assert_eq!(
            agents[1].progress,
            found("Headline of b", "review"),
            "round {round}: branch b"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn the_tracked_branches_are_the_same_after_a_daemon_restart() {
    let sandbox = Sandbox::new();
    sandbox.push("task-a", "build", "Writing the parser");
    let first = sandbox.socket("first");
    start_polling(&first, &sandbox.database(), NEVER_AGAIN);
    let clone = folder_text(&sandbox.clone_folder());
    track_cloud_task(&first, &clone, "task-a").expect("tracked");
    track_cloud_task(&first, &clone, "task-b").expect("tracked");
    let before = list_agents(&first).expect("listed");

    let second = sandbox.socket("second");
    start(&second, &sandbox.database());
    let after = wait_for(&second, |agents| {
        agents
            .iter()
            .all(|agent| agent.progress != AgentProgress::Pending)
    });

    assert_eq!(after, before, "same IDs, folders, branches and progress");
    assert_eq!(after.len(), 2, "both branches");
}
