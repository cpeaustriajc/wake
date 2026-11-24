//! `wake hook reload` against a real daemon on a temporary socket.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use wake_daemon::protocol::{CompactionReport, ProgressState};
use wake_daemon::{Store, bind, serve};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const PROGRESS: &str = "---\nphase: build\nheadline: Adding the parser\nupdated: 2026-11-24T14:00:00Z\n---\nNext: tests\n";

fn scratch() -> PathBuf {
    let folder = std::env::temp_dir().join(format!(
        "wake-reload-test-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    fs::create_dir_all(folder.join("repo/.wake")).expect("scratch folder");
    folder
}

fn start_daemon(folder: &Path) -> PathBuf {
    let socket = folder.join("d.sock");
    let store = Store::open(&folder.join("d.db")).expect("database opens");
    let listener = bind(&socket).expect("socket binds");
    thread::spawn(move || {
        let _ = serve(&listener, store);
    });
    socket
}

fn hook(socket: &Path, payload: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wake"))
        .args(["hook", "reload"])
        .env("WAKE_SOCKET", socket)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("wake starts");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(payload.as_bytes())
        .expect("payload written");
    child.wait_with_output().expect("wake exits")
}

fn payload(folder: &Path) -> String {
    serde_json::json!({
        "session_id": "session-1",
        "cwd": folder.join("repo"),
        "hook_event_name": "SessionStart",
        "source": "compact",
    })
    .to_string()
}

fn stored(folder: &Path) -> Vec<CompactionReport> {
    Store::open(&folder.join("d.db"))
        .expect("database opens")
        .compactions()
        .expect("rows read")
        .into_iter()
        .map(|row| row.report)
        .collect()
}

#[test]
fn prints_the_progress_file_and_the_daemon_records_the_compaction() {
    let folder = scratch();
    fs::write(folder.join("repo/.wake/progress.md"), PROGRESS).expect("progress file");
    let socket = start_daemon(&folder);

    let output = hook(&socket, &payload(&folder));

    assert!(output.status.success(), "the hook exits 0");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        PROGRESS,
        "the progress file is printed as it is"
    );
    assert_eq!(
        stored(&folder),
        vec![CompactionReport {
            session_id: Some("session-1".to_owned()),
            cwd: folder.join("repo").to_string_lossy().into_owned(),
            source: "compact".to_owned(),
            progress: ProgressState::Found,
        }],
        "the daemon stored the compaction"
    );
}

#[test]
fn says_so_when_the_progress_file_is_missing_and_still_reports() {
    let folder = scratch();
    let socket = start_daemon(&folder);

    let output = hook(&socket, &payload(&folder));

    assert!(output.status.success(), "the hook exits 0");
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("there is no progress file"),
        "the output says the file is missing"
    );
    assert_eq!(
        stored(&folder)[0].progress,
        ProgressState::Missing,
        "stored state"
    );
}

#[test]
fn says_so_when_the_progress_file_is_invalid_and_still_reports() {
    let folder = scratch();
    fs::write(folder.join("repo/.wake/progress.md"), "no front matter\n").expect("progress file");
    let socket = start_daemon(&folder);

    let output = hook(&socket, &payload(&folder));

    assert!(output.status.success(), "the hook exits 0");
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("is not valid"),
        "the output says the file is invalid"
    );
    assert_eq!(
        stored(&folder)[0].progress,
        ProgressState::Invalid,
        "stored state"
    );
}

#[test]
fn a_stopped_daemon_does_not_break_the_hook() {
    let folder = scratch();
    fs::write(folder.join("repo/.wake/progress.md"), PROGRESS).expect("progress file");

    let output = hook(&folder.join("none.sock"), &payload(&folder));

    assert!(output.status.success(), "the hook still exits 0");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        PROGRESS,
        "the progress file is still printed"
    );
}
