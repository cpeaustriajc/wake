//! Floor positions, over a real Unix socket and a real SQLite file.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use rusqlite::Connection;
use wake_daemon::protocol::{FloorPlacement, FloorPosition, INVALID_PARAMS, Location};
use wake_daemon::{
    Error, Store, bind, list_agents, place_on_floor, register_agent, serve, track_cloud_task,
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const SCHEMA_BEFORE_THE_FLOOR: &str = "
CREATE TABLE IF NOT EXISTS compactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    recorded_at INTEGER NOT NULL,
    session_id TEXT,
    cwd TEXT NOT NULL,
    source TEXT NOT NULL,
    progress TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS agents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    folder TEXT NOT NULL UNIQUE,
    location TEXT NOT NULL,
    started_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS cloud_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    folder TEXT NOT NULL,
    branch TEXT NOT NULL,
    tracked_at INTEGER NOT NULL,
    UNIQUE (folder, branch)
);";

fn scratch() -> PathBuf {
    let folder = std::env::temp_dir().join(format!(
        "wake-floor-test-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    fs::create_dir_all(&folder).expect("scratch folder");
    folder
}

fn workspace(parent: &Path, name: &str) -> String {
    let folder = parent.join(name);
    fs::create_dir_all(&folder).expect("workspace");
    folder.to_string_lossy().into_owned()
}

fn start(folder: &Path) -> PathBuf {
    let socket = folder.join("d.sock");
    let store = Store::open(&folder.join("d.db")).expect("database opens");
    let listener = bind(&socket).expect("socket binds");
    thread::spawn(move || {
        let _ = serve(&listener, store);
    });
    socket
}

fn placement(location: Location, id: i64, x: i64, y: i64) -> FloorPlacement {
    FloorPlacement { location, id, x, y }
}

fn refusal(outcome: Result<i64, Error>) -> (i64, String) {
    match outcome {
        Err(Error::Rpc(error)) => (error.code, error.message),
        other => panic!("expected the daemon to refuse, got {other:?}"),
    }
}

#[test]
fn an_agent_has_no_position_until_it_is_placed() {
    let folder = scratch();
    let socket = start(&folder);
    register_agent(&socket, &workspace(&folder, "one")).expect("registered");

    let agents = list_agents(&socket).expect("listed");

    assert_eq!(agents.len(), 1, "the agent lists");
    assert_eq!(agents[0].position, None, "nobody placed it yet");
}

#[test]
fn a_placed_local_agent_and_a_placed_cloud_task_list_with_their_positions() {
    let folder = scratch();
    let socket = start(&folder);
    let local = register_agent(&socket, &workspace(&folder, "local")).expect("registered");
    let cloud = track_cloud_task(&socket, &workspace(&folder, "repository"), "claude/task")
        .expect("tracked");

    let placed_local =
        place_on_floor(&socket, &placement(Location::Local, local, 360, 40)).expect("placed");
    let placed_cloud =
        place_on_floor(&socket, &placement(Location::Cloud, cloud, 40, 480)).expect("placed");

    assert_eq!(placed_local, local, "the answer names the local agent");
    assert_eq!(placed_cloud, cloud, "the answer names the cloud task");
    let agents = list_agents(&socket).expect("listed");
    assert_eq!(agents.len(), 2, "both list");
    assert_eq!(
        agents[0].location,
        Location::Local,
        "local agents come first"
    );
    assert_eq!(
        agents[0].position,
        Some(FloorPosition { x: 360, y: 40 }),
        "the local agent sits where it was placed"
    );
    assert_eq!(
        agents[1].location,
        Location::Cloud,
        "cloud tasks come second"
    );
    assert_eq!(
        agents[1].position,
        Some(FloorPosition { x: 40, y: 480 }),
        "the cloud task sits where it was placed"
    );
}

#[test]
fn a_local_agent_and_a_cloud_task_with_the_same_id_keep_separate_positions() {
    let folder = scratch();
    let socket = start(&folder);
    let local = register_agent(&socket, &workspace(&folder, "local")).expect("registered");
    let cloud = track_cloud_task(&socket, &workspace(&folder, "repository"), "claude/task")
        .expect("tracked");
    assert_eq!(local, cloud, "the first of each kind share an ID");

    place_on_floor(&socket, &placement(Location::Local, local, 40, 40)).expect("placed");

    let agents = list_agents(&socket).expect("listed");
    assert_eq!(
        agents[0].position,
        Some(FloorPosition { x: 40, y: 40 }),
        "the local agent was placed"
    );
    assert_eq!(agents[1].position, None, "the cloud task was not");
}

#[test]
fn placing_again_moves_the_agent() {
    let folder = scratch();
    let socket = start(&folder);
    let agent = register_agent(&socket, &workspace(&folder, "one")).expect("registered");

    place_on_floor(&socket, &placement(Location::Local, agent, 40, 40)).expect("placed");
    place_on_floor(&socket, &placement(Location::Local, agent, 680, 260)).expect("moved");

    let agents = list_agents(&socket).expect("listed");
    assert_eq!(
        agents[0].position,
        Some(FloorPosition { x: 680, y: 260 }),
        "the later position wins"
    );
}

#[test]
fn registering_the_folder_again_keeps_its_position() {
    let folder = scratch();
    let socket = start(&folder);
    let path = workspace(&folder, "one");
    let agent = register_agent(&socket, &path).expect("registered");
    place_on_floor(&socket, &placement(Location::Local, agent, 360, 260)).expect("placed");

    let again = register_agent(&socket, &path).expect("registered again");

    assert_eq!(again, agent, "the folder keeps its ID");
    let agents = list_agents(&socket).expect("listed");
    assert_eq!(
        agents[0].position,
        Some(FloorPosition { x: 360, y: 260 }),
        "and its position"
    );
}

#[test]
fn a_negative_position_is_refused_and_nothing_is_stored() {
    let folder = scratch();
    let socket = start(&folder);
    let agent = register_agent(&socket, &workspace(&folder, "one")).expect("registered");

    let left = refusal(place_on_floor(
        &socket,
        &placement(Location::Local, agent, -20, 40),
    ));
    let above = refusal(place_on_floor(
        &socket,
        &placement(Location::Local, agent, 40, -20),
    ));

    assert_eq!(left.0, INVALID_PARAMS, "a negative x is invalid");
    assert_eq!(above.0, INVALID_PARAMS, "a negative y is invalid");
    assert!(
        left.1.contains("cannot be negative"),
        "the message says why: {}",
        left.1
    );
    let agents = list_agents(&socket).expect("listed");
    assert_eq!(agents[0].position, None, "the agent stays unplaced");
}

#[test]
fn an_agent_the_daemon_does_not_know_is_refused() {
    let folder = scratch();
    let socket = start(&folder);
    let agent = register_agent(&socket, &workspace(&folder, "one")).expect("registered");

    let missing = refusal(place_on_floor(
        &socket,
        &placement(Location::Local, agent + 1, 40, 40),
    ));
    let wrong_kind = refusal(place_on_floor(
        &socket,
        &placement(Location::Cloud, agent, 40, 40),
    ));

    assert_eq!(missing.0, INVALID_PARAMS, "an unknown ID is invalid");
    assert_eq!(
        wrong_kind.0, INVALID_PARAMS,
        "a local agent's ID names no cloud task"
    );
    assert!(
        wrong_kind.1.contains("no cloud agent"),
        "the message names the kind: {}",
        wrong_kind.1
    );
}

#[test]
fn a_position_is_still_there_after_the_database_is_opened_again() {
    let folder = scratch();
    let database = folder.join("d.db");
    let store = Store::open(&database).expect("database opens");
    let local = store
        .register_agent("/work/one", Location::Local)
        .expect("registered");
    let cloud = store
        .track_cloud_task("/work/repository", "claude/task")
        .expect("tracked");
    let stored_local = store
        .place_on_floor(Location::Local, local, FloorPosition { x: 40, y: 260 })
        .expect("placed");
    let stored_cloud = store
        .place_on_floor(Location::Cloud, cloud, FloorPosition { x: 360, y: 40 })
        .expect("placed");
    drop(store);

    let reopened = Store::open(&database).expect("database opens again");

    assert!(stored_local, "the local agent was known");
    assert!(stored_cloud, "the cloud task was known");
    assert_eq!(
        reopened.agents().expect("agents")[0].position,
        Some(FloorPosition { x: 40, y: 260 }),
        "the local agent's position survived"
    );
    assert_eq!(
        reopened.cloud_tasks().expect("cloud tasks")[0].position,
        Some(FloorPosition { x: 360, y: 40 }),
        "the cloud task's position survived"
    );
}

#[test]
fn a_database_made_before_the_floor_opens_keeps_its_rows_and_takes_a_position() {
    let folder = scratch();
    let database = folder.join("old.db");
    let old = Connection::open(&database).expect("old database");
    old.execute_batch(SCHEMA_BEFORE_THE_FLOOR)
        .expect("old schema");
    old.execute(
        "INSERT INTO agents (folder, location, started_at) VALUES ('/work/old', 'local', 1700)",
        [],
    )
    .expect("old agent");
    old.execute(
        "INSERT INTO cloud_tasks (folder, branch, tracked_at) VALUES ('/work/repo', 'main', 1800)",
        [],
    )
    .expect("old cloud task");
    drop(old);

    let store = Store::open(&database).expect("the old database opens");

    let before = store.agents().expect("agents");
    assert_eq!(before.len(), 1, "the old agent is still there");
    assert_eq!(before[0].folder, "/work/old", "with its folder");
    assert_eq!(before[0].started_at, 1700, "and its start time");
    assert_eq!(before[0].position, None, "and no position yet");
    assert_eq!(
        store.cloud_tasks().expect("cloud tasks")[0].position,
        None,
        "the old cloud task has no position either"
    );
    let placed = store
        .place_on_floor(
            Location::Local,
            before[0].id,
            FloorPosition { x: 40, y: 40 },
        )
        .expect("placed");
    assert!(placed, "the old agent can be placed");
    assert_eq!(
        store.agents().expect("agents")[0].position,
        Some(FloorPosition { x: 40, y: 40 }),
        "and lists with its position"
    );
}
