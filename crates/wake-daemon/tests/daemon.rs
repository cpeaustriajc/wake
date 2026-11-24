//! The daemon over a real Unix socket and a real SQLite file.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use wake_daemon::protocol::{CompactionReport, ProgressState};
use wake_daemon::{Error, Store, bind, record_compaction, serve};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn scratch() -> PathBuf {
    let folder = std::env::temp_dir().join(format!(
        "wake-daemon-test-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    fs::create_dir_all(&folder).expect("scratch folder");
    folder
}

fn report(progress: ProgressState) -> CompactionReport {
    CompactionReport {
        session_id: Some("session-1".to_owned()),
        cwd: "/work/project".to_owned(),
        source: "compact".to_owned(),
        progress,
    }
}

fn start(folder: &std::path::Path) -> PathBuf {
    let socket = folder.join("d.sock");
    let store = Store::open(&folder.join("d.db")).expect("database opens");
    let listener = bind(&socket).expect("socket binds");
    thread::spawn(move || {
        let _ = serve(&listener, store);
    });
    socket
}

#[test]
fn records_a_compaction_in_sqlite() {
    let folder = scratch();
    let socket = start(&folder);

    let first = record_compaction(&socket, &report(ProgressState::Found)).expect("recorded");
    let second = record_compaction(&socket, &report(ProgressState::Missing)).expect("recorded");
    assert!(second > first, "row IDs increase");

    let stored = Store::open(&folder.join("d.db"))
        .expect("database opens")
        .compactions()
        .expect("rows read");
    assert_eq!(stored.len(), 2, "two rows");
    assert_eq!(stored[0].id, first, "first row id");
    assert_eq!(stored[0].report, report(ProgressState::Found), "first row");
    assert_eq!(
        stored[1].report,
        report(ProgressState::Missing),
        "second row"
    );
    assert!(stored[0].recorded_at > 0, "recorded time");
}

#[test]
fn answers_json_rpc_errors() {
    let folder = scratch();
    let socket = start(&folder);
    let stream = UnixStream::connect(&socket).expect("connects");
    let mut writer = stream.try_clone().expect("clones");
    let mut reader = BufReader::new(stream);

    writeln!(
        writer,
        r#"{{"jsonrpc":"2.0","id":7,"method":"nope","params":{{}}}}"#
    )
    .expect("sent");
    let mut line = String::new();
    reader.read_line(&mut line).expect("answered");
    assert!(
        line.contains(r#""id":7"#) && line.contains("-32601"),
        "unknown method: {line}"
    );

    line.clear();
    writeln!(
        writer,
        r#"{{"jsonrpc":"2.0","id":8,"method":"compaction.record","params":{{}}}}"#
    )
    .expect("sent");
    reader.read_line(&mut line).expect("answered");
    assert!(line.contains("-32602"), "bad params: {line}");

    line.clear();
    writeln!(writer, "not json").expect("sent");
    reader.read_line(&mut line).expect("answered");
    assert!(line.contains("-32700"), "parse error: {line}");
}

#[test]
fn the_socket_is_private_and_only_one_daemon_binds_it() {
    let folder = scratch();
    let socket = start(&folder);
    let mode = fs::metadata(&socket)
        .expect("socket exists")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600, "only the owner can connect");
    assert!(
        matches!(bind(&socket), Err(Error::AlreadyRunning)),
        "a second daemon is refused"
    );
}

#[test]
fn replaces_a_socket_left_by_a_stopped_daemon() {
    let folder = scratch();
    let socket = folder.join("old.sock");
    drop(bind(&socket).expect("first bind"));
    assert!(bind(&socket).is_ok(), "a stale socket file is replaced");
}

#[test]
fn reports_an_unreachable_daemon() {
    let folder = scratch();
    let result = record_compaction(&folder.join("none.sock"), &report(ProgressState::Found));
    assert!(matches!(result, Err(Error::Io(_))), "no daemon, no row");
}
