//! `wake agents` and `wake agent start` against a real daemon and a fake
//! `claude`, so no login is needed.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

static NEXT: AtomicUsize = AtomicUsize::new(0);

const VALID: &str = "---\nphase: build\nheadline: Adding the parser\nupdated: 2026-11-24T14:00:00Z\n---\nNext: tests\n";
const CHANGED: &str =
    "---\nphase: review\nheadline: Parser done\nupdated: 2026-11-24T15:30:00Z\n---\n";

fn scratch() -> PathBuf {
    let folder = std::env::temp_dir().join(format!(
        "wake-agents-cli-test-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    fs::create_dir_all(&folder).expect("scratch folder");
    folder.canonicalize().expect("canonical scratch folder")
}

fn workspace(parent: &Path, name: &str, progress: Option<&str>) -> PathBuf {
    let folder = parent.join(name);
    fs::create_dir_all(folder.join(".wake")).expect("workspace");
    if let Some(text) = progress {
        fs::write(folder.join(".wake/progress.md"), text).expect("progress file");
    }
    folder
}

fn fake_claude(folder: &Path) -> PathBuf {
    let script = folder.join("claude");
    fs::write(
        &script,
        "#!/bin/sh\ntouch \"$(pwd)/claude-ran\"\ncat >/dev/null\n",
    )
    .expect("script");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).expect("executable");
    script
}

fn wake(folder: &Path, arguments: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_wake"));
    command
        .args(arguments)
        .env("WAKE_SOCKET", folder.join("d.sock"))
        .env("WAKE_DB", folder.join("d.db"))
        .env("WAKE_CLAUDE", fake_claude(folder))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

fn run(folder: &Path, arguments: &[&str]) -> Output {
    wake(folder, arguments).output().expect("wake runs")
}

struct Daemon(Child);

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn start_daemon(folder: &Path) -> Daemon {
    let daemon = Daemon(wake(folder, &["daemon"]).spawn().expect("daemon starts"));
    for _ in 0..200 {
        if UnixStream::connect(folder.join("d.sock")).is_ok() {
            return daemon;
        }
        thread::sleep(Duration::from_millis(25));
    }
    panic!("the daemon never listened");
}

fn stop(daemon: Daemon) {
    drop(daemon);
}

fn lines(output: &Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

fn agent_start(folder: &Path, workspace: &Path) {
    let output = run(folder, &["agent", "start", &workspace.to_string_lossy()]);
    assert!(
        output.status.success(),
        "agent start: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn lists_two_started_agents_one_line_each() {
    let folder = scratch();
    let daemon = start_daemon(&folder);
    let one = workspace(&folder, "one", Some(VALID));
    let two = workspace(&folder, "two", Some(CHANGED));
    agent_start(&folder, &one);
    agent_start(&folder, &two);

    let output = run(&folder, &["agents"]);
    stop(daemon);

    assert!(output.status.success(), "wake agents exits 0");
    let lines = lines(&output);
    assert_eq!(lines.len(), 2, "one line per agent: {lines:?}");
    for (line, (headline, phase, updated, path)) in lines.iter().zip([
        ("Adding the parser", "build", "2026-11-24T14:00:00Z", &one),
        ("Parser done", "review", "2026-11-24T15:30:00Z", &two),
    ]) {
        for word in [headline, phase, updated, "local", &path.to_string_lossy()] {
            assert!(line.contains(word), "`{word}` is in `{line}`");
        }
    }
}

#[test]
fn a_changed_progress_file_shows_on_the_next_call() {
    let folder = scratch();
    let daemon = start_daemon(&folder);
    let agent = workspace(&folder, "agent", Some(VALID));
    agent_start(&folder, &agent);
    let before = lines(&run(&folder, &["agents"]));

    fs::write(agent.join(".wake/progress.md"), CHANGED).expect("file changes");
    let after = lines(&run(&folder, &["agents"]));
    stop(daemon);

    assert!(
        before[0].contains("Adding the parser"),
        "before: {before:?}"
    );
    assert!(
        after[0].contains("Parser done") && after[0].contains("review"),
        "after: {after:?}"
    );
}

#[test]
fn a_missing_and_an_invalid_file_get_a_plain_label_and_the_rest_still_list() {
    let folder = scratch();
    let daemon = start_daemon(&folder);
    let good = workspace(&folder, "good", Some(VALID));
    let absent = workspace(&folder, "absent", None);
    let broken = workspace(&folder, "broken", Some("not a progress file\n"));
    for path in [&absent, &broken, &good] {
        agent_start(&folder, path);
    }

    let output = run(&folder, &["agents"]);
    stop(daemon);

    let lines = lines(&output);
    assert_eq!(lines.len(), 3, "every agent lists: {lines:?}");
    assert!(
        lines[0].contains("(no progress file)"),
        "missing: {}",
        lines[0]
    );
    assert!(
        lines[1].contains("(invalid progress file)"),
        "invalid: {}",
        lines[1]
    );
    assert!(
        lines[2].contains("Adding the parser"),
        "valid: {}",
        lines[2]
    );
}

#[test]
fn the_list_is_the_same_after_the_daemon_restarts() {
    let folder = scratch();
    let daemon = start_daemon(&folder);
    agent_start(&folder, &workspace(&folder, "one", Some(VALID)));
    agent_start(&folder, &workspace(&folder, "two", None));
    let before = lines(&run(&folder, &["agents"]));
    stop(daemon);

    let daemon = start_daemon(&folder);
    let after = lines(&run(&folder, &["agents"]));
    stop(daemon);

    assert_eq!(before.len(), 2, "two agents before: {before:?}");
    assert_eq!(before, after, "the same list after a restart");
}

#[test]
fn the_command_line_prints_what_the_daemon_says_and_reads_no_progress_file() {
    let folder = scratch();
    let listener = UnixListener::bind(folder.join("d.sock")).expect("socket");
    let answer = r#"{"jsonrpc":"2.0","id":1,"result":{"agents":[{"id":9,"folder":"/nowhere","location":"cloud","started_at":0,"progress":{"state":"found","headline":"Said only by the daemon","phase":"plan","updated":"2026-01-01T00:00:00Z"}}]}}"#;
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("client connects");
        let mut request = String::new();
        BufReader::new(stream.try_clone().expect("clones"))
            .read_line(&mut request)
            .expect("request read");
        writeln!(&stream, "{answer}").expect("answer sent");
        request
    });

    let output = run(&folder, &["agents"]);
    let request = server.join().expect("server done");

    assert!(request.contains("agent.list"), "asks the daemon: {request}");
    let lines = lines(&output);
    assert_eq!(lines.len(), 1, "one line: {lines:?}");
    for word in [
        "Said only by the daemon",
        "plan",
        "2026-01-01T00:00:00Z",
        "cloud",
    ] {
        assert!(lines[0].contains(word), "`{word}` is in `{}`", lines[0]);
    }
}

#[test]
fn with_the_daemon_down_it_says_so_in_one_line_and_fails() {
    let folder = scratch();

    let output = run(&folder, &["agents"]);

    assert!(!output.status.success(), "a non-zero exit code");
    assert!(output.stdout.is_empty(), "nothing on stdout");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.lines().count(), 1, "one line: {stderr}");
    assert!(
        stderr.contains("daemon is not running"),
        "says why: {stderr}"
    );
}

#[test]
fn an_agent_is_not_started_when_the_daemon_is_down() {
    let folder = scratch();
    let agent = workspace(&folder, "agent", None);

    let output = run(&folder, &["agent", "start", &agent.to_string_lossy()]);

    assert!(!output.status.success(), "a non-zero exit code");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("daemon is not running"),
        "says why"
    );
    assert!(!agent.join("claude-ran").exists(), "claude never started");
}

#[test]
fn starting_an_agent_runs_claude_in_its_folder() {
    let folder = scratch();
    let daemon = start_daemon(&folder);
    let agent = workspace(&folder, "agent", None);

    agent_start(&folder, &agent);
    stop(daemon);

    assert!(
        agent.join("claude-ran").exists(),
        "claude ran in the folder"
    );
}
