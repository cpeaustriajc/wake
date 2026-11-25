//! The list of agents, over a real Unix socket and a real SQLite file.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use wake_daemon::protocol::{AgentProgress, Location};
use wake_daemon::{Store, bind, list_agents, register_agent, serve};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const VALID: &str = "---\nphase: build\nheadline: Adding the parser\nupdated: 2026-11-24T14:00:00Z\n---\nNext: tests\n";
const CHANGED: &str =
    "---\nphase: review\nheadline: Parser done\nupdated: 2026-11-24T15:30:00+02:00\n---\n";

fn scratch() -> PathBuf {
    let folder = std::env::temp_dir().join(format!(
        "wake-agents-test-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    fs::create_dir_all(&folder).expect("scratch folder");
    folder
}

fn workspace(parent: &Path, name: &str, progress: Option<&str>) -> String {
    let folder = parent.join(name);
    fs::create_dir_all(folder.join(".wake")).expect("workspace");
    if let Some(text) = progress {
        fs::write(folder.join(".wake/progress.md"), text).expect("progress file");
    }
    folder.to_string_lossy().into_owned()
}

fn start(socket: &Path, database: &Path) {
    let store = Store::open(database).expect("database opens");
    let listener = bind(socket).expect("socket binds");
    thread::spawn(move || {
        let _ = serve(&listener, store);
    });
}

#[test]
fn lists_each_agent_with_what_its_progress_file_says() {
    let folder = scratch();
    let socket = folder.join("d.sock");
    start(&socket, &folder.join("d.db"));
    let good = workspace(&folder, "good", Some(VALID));
    let absent = workspace(&folder, "absent", None);
    let broken = workspace(&folder, "broken", Some("headline: no front matter\n"));
    for path in [&good, &absent, &broken] {
        register_agent(&socket, path).expect("registered");
    }

    let agents = list_agents(&socket).expect("listed");

    assert_eq!(agents.len(), 3, "every agent lists");
    assert!(
        agents.iter().all(|agent| agent.location == Location::Local),
        "agents start local"
    );
    assert_eq!(agents[0].folder, good, "oldest first");
    assert_eq!(
        agents[0].progress,
        AgentProgress::Found {
            headline: "Adding the parser".to_owned(),
            phase: "build".to_owned(),
            updated: "2026-11-24T14:00:00Z".to_owned(),
        },
        "valid file"
    );
    assert_eq!(agents[1].progress, AgentProgress::Missing, "missing file");
    assert!(
        matches!(agents[2].progress, AgentProgress::Invalid { .. }),
        "invalid file: {:?}",
        agents[2].progress
    );
}

#[test]
fn a_changed_progress_file_shows_on_the_next_call() {
    let folder = scratch();
    let socket = folder.join("d.sock");
    start(&socket, &folder.join("d.db"));
    let path = workspace(&folder, "agent", Some(VALID));
    register_agent(&socket, &path).expect("registered");
    let before = list_agents(&socket).expect("listed");

    fs::write(Path::new(&path).join(".wake/progress.md"), CHANGED).expect("file changes");
    let after = list_agents(&socket).expect("listed");

    assert_ne!(before[0].progress, after[0].progress, "the entry changed");
    assert_eq!(
        after[0].progress,
        AgentProgress::Found {
            headline: "Parser done".to_owned(),
            phase: "review".to_owned(),
            updated: "2026-11-24T15:30:00+02:00".to_owned(),
        },
        "new content"
    );
}

#[test]
fn a_file_that_goes_missing_or_invalid_does_not_hide_the_other_agents() {
    let folder = scratch();
    let socket = folder.join("d.sock");
    start(&socket, &folder.join("d.db"));
    let first = workspace(&folder, "first", Some(VALID));
    let second = workspace(&folder, "second", Some(VALID));
    register_agent(&socket, &first).expect("registered");
    register_agent(&socket, &second).expect("registered");

    fs::remove_file(Path::new(&first).join(".wake/progress.md")).expect("file removed");
    fs::write(
        Path::new(&second).join(".wake/progress.md"),
        "---\nphase: x\n---\n",
    )
    .expect("file broken");
    let agents = list_agents(&socket).expect("listed");

    assert_eq!(agents.len(), 2, "both still list");
    assert_eq!(agents[0].progress, AgentProgress::Missing, "removed file");
    assert!(
        matches!(agents[1].progress, AgentProgress::Invalid { .. }),
        "broken file"
    );
}

#[test]
fn a_folder_that_no_longer_exists_lists_as_missing() {
    let folder = scratch();
    let socket = folder.join("d.sock");
    start(&socket, &folder.join("d.db"));
    register_agent(&socket, "/no/such/folder").expect("registered");

    let agents = list_agents(&socket).expect("listed");

    assert_eq!(agents[0].progress, AgentProgress::Missing, "no folder");
}

#[test]
fn the_list_survives_a_daemon_restart() {
    let folder = scratch();
    let database = folder.join("d.db");
    let first = folder.join("first.sock");
    start(&first, &database);
    let one = workspace(&folder, "one", Some(VALID));
    let two = workspace(&folder, "two", None);
    register_agent(&first, &one).expect("registered");
    register_agent(&first, &two).expect("registered");
    let before = list_agents(&first).expect("listed");

    let second = folder.join("second.sock");
    start(&second, &database);
    let after = list_agents(&second).expect("listed after restart");

    assert_eq!(before, after, "the same list from the same file");
    assert_eq!(after.len(), 2, "both agents");
}

#[test]
fn registering_a_folder_twice_keeps_one_agent() {
    let folder = scratch();
    let socket = folder.join("d.sock");
    start(&socket, &folder.join("d.db"));
    let path = workspace(&folder, "agent", Some(VALID));

    let first = register_agent(&socket, &path).expect("registered");
    let second = register_agent(&socket, &path).expect("registered again");

    assert_eq!(first, second, "same ID");
    assert_eq!(list_agents(&socket).expect("listed").len(), 1, "one agent");
}

#[test]
fn rejects_a_registration_without_a_folder() {
    let folder = scratch();
    let socket = folder.join("d.sock");
    start(&socket, &folder.join("d.db"));
    let stream = UnixStream::connect(&socket).expect("connects");
    let mut writer = stream.try_clone().expect("clones");
    let mut reader = BufReader::new(stream);

    writeln!(
        writer,
        r#"{{"jsonrpc":"2.0","id":3,"method":"agent.register","params":{{}}}}"#
    )
    .expect("sent");
    let mut line = String::new();
    reader.read_line(&mut line).expect("answered");

    assert!(line.contains("-32602"), "bad params: {line}");
}
