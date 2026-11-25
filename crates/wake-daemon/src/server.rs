//! The daemon's Unix socket and the client that talks to it.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::agents::entry;
use crate::cloud::CloudReader;
use crate::error::Error;
use crate::protocol::{
    AgentEntry, AgentList, AgentRegistration, CloudTracking, CompactionReport, FloorPlacement,
    FloorPosition, INTERNAL_ERROR, INVALID_PARAMS, LIST_AGENTS, Location, METHOD_NOT_FOUND,
    OPEN_TERMINAL, OpenedTerminal, PARSE_ERROR, PLACE_ON_FLOOR, READ_TERMINAL, RECORD_COMPACTION,
    REGISTER_AGENT, RESIZE_TERMINAL, Request, Response, TRACK_CLOUD_TASK, TerminalInput,
    TerminalOpening, TerminalOutput, TerminalReading, TerminalSize, VERSION, WRITE_TERMINAL,
};
use crate::store::Store;
use crate::terminal::{Refusal, Terminals, user_shell};

const SOCKET_MODE: u32 = 0o600;
/// How long the daemon waits between two reads of the tracked cloud branches
/// when [`serve`] runs it.
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(15);

#[derive(Debug)]
struct Shared {
    store: Mutex<Store>,
    cloud: CloudReader,
    terminals: Terminals,
}

/// Starts listening on `path`, replacing a socket file left by a daemon that
/// is no longer running. Only the current user can connect.
///
/// # Errors
///
/// Returns [`Error::AlreadyRunning`] if a daemon answers on `path`, or
/// [`Error::Io`] if the socket cannot be created.
pub fn bind(path: &Path) -> Result<UnixListener, Error> {
    if UnixStream::connect(path).is_ok() {
        return Err(Error::AlreadyRunning);
    }
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let listener = UnixListener::bind(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(SOCKET_MODE))?;
    Ok(listener)
}

/// Answers requests on `listener` until it fails, one thread per connection,
/// and reads the tracked cloud branches every [`DEFAULT_POLL_INTERVAL`].
///
/// # Errors
///
/// Returns [`Error::Io`] if accepting a connection fails.
pub fn serve(listener: &UnixListener, store: Store) -> Result<(), Error> {
    serve_polling(listener, store, DEFAULT_POLL_INTERVAL)
}

/// Does what [`serve`] does, reading the tracked cloud branches every
/// `interval`. The first read starts at once, and a branch is read once more
/// the moment it is tracked. `agent.list` answers from the latest read and
/// runs no `git` command.
///
/// # Errors
///
/// Returns [`Error::Io`] if accepting a connection fails.
pub fn serve_polling(
    listener: &UnixListener,
    store: Store,
    interval: Duration,
) -> Result<(), Error> {
    serve_with_shell(listener, store, interval, &user_shell())
}

/// Does what [`serve_polling`] does, running `shell` in every agent's
/// terminal instead of the program in the daemon's `SHELL` variable. The
/// shells are children of the process that calls this and end with it.
///
/// # Errors
///
/// Returns [`Error::Io`] if accepting a connection fails.
pub fn serve_with_shell(
    listener: &UnixListener,
    store: Store,
    interval: Duration,
    shell: &Path,
) -> Result<(), Error> {
    let shared = Arc::new(Shared {
        store: Mutex::new(store),
        cloud: CloudReader::default(),
        terminals: Terminals::new(shell.to_path_buf()),
    });
    let poller = Arc::clone(&shared);
    thread::spawn(move || poll(&poller, interval));
    for stream in listener.incoming() {
        let stream = stream?;
        let shared = Arc::clone(&shared);
        thread::spawn(move || {
            let _ = handle(stream, &shared);
        });
    }
    Ok(())
}

fn poll(shared: &Shared, interval: Duration) {
    loop {
        let tasks = lock(&shared.store).cloud_tasks();
        for task in tasks.unwrap_or_default() {
            shared.cloud.read(&task.folder, &task.branch);
        }
        thread::sleep(interval);
    }
}

fn handle(stream: UnixStream, shared: &Shared) -> Result<(), Error> {
    let mut writer = stream.try_clone()?;
    for line in BufReader::new(stream).lines() {
        let response = answer(&line?, shared);
        writeln!(writer, "{}", serde_json::to_string(&response)?)?;
    }
    Ok(())
}

fn lock(store: &Mutex<Store>) -> MutexGuard<'_, Store> {
    store.lock().unwrap_or_else(PoisonError::into_inner)
}

fn reply<T: Serialize>(id: u64, outcome: Result<T, Error>) -> Response {
    match outcome.and_then(|value| Ok(serde_json::to_value(value)?)) {
        Ok(value) => Response::success(id, value),
        Err(error) => Response::failure(id, INTERNAL_ERROR, error.to_string()),
    }
}

fn answer(line: &str, shared: &Shared) -> Response {
    let request: Request = match serde_json::from_str(line) {
        Ok(request) => request,
        Err(error) => return Response::failure(0, PARSE_ERROR, error.to_string()),
    };
    let id = request.id;
    if request.jsonrpc != VERSION {
        return Response::failure(
            id,
            METHOD_NOT_FOUND,
            format!("unknown method `{}`", request.method),
        );
    }
    match request.method.as_str() {
        RECORD_COMPACTION => with_params(id, request.params, |report: CompactionReport| {
            let stored = lock(&shared.store).record_compaction(&report);
            reply(id, stored.map(|row| json!({ "id": row })))
        }),
        REGISTER_AGENT => with_params(id, request.params, |agent: AgentRegistration| {
            let stored = lock(&shared.store).register_agent(&agent.folder, Location::Local);
            reply(id, stored.map(|row| json!({ "id": row })))
        }),
        TRACK_CLOUD_TASK => with_params(id, request.params, |tracking: CloudTracking| {
            if let Some(problem) = branch_problem(&tracking.branch) {
                return Response::failure(id, INVALID_PARAMS, problem);
            }
            let stored = lock(&shared.store).track_cloud_task(&tracking.folder, &tracking.branch);
            if stored.is_ok() {
                shared.cloud.read(&tracking.folder, &tracking.branch);
            }
            reply(id, stored.map(|row| json!({ "id": row })))
        }),
        LIST_AGENTS => {
            let (local, cloud) = {
                let store = lock(&shared.store);
                (store.agents(), store.cloud_tasks())
            };
            reply(
                id,
                local.and_then(|local| {
                    let mut agents: Vec<AgentEntry> = local.into_iter().map(entry).collect();
                    agents.extend(cloud?.into_iter().map(|task| shared.cloud.entry(task)));
                    Ok(AgentList { agents })
                }),
            )
        }
        PLACE_ON_FLOOR => with_params(id, request.params, |placement: FloorPlacement| {
            place(id, placement, shared)
        }),
        method => terminal(id, method, request.params, shared).unwrap_or_else(|| {
            Response::failure(id, METHOD_NOT_FOUND, format!("unknown method `{method}`"))
        }),
    }
}

fn settle<T: Serialize>(id: u64, outcome: Result<T, Refusal>) -> Response {
    match outcome {
        Ok(value) => reply(id, Ok(value)),
        Err(Refusal::Invalid(message)) => Response::failure(id, INVALID_PARAMS, message),
        Err(Refusal::Failed(message)) => Response::failure(id, INTERNAL_ERROR, message),
    }
}

fn terminal(id: u64, method: &str, params: Value, shared: &Shared) -> Option<Response> {
    let terminals = &shared.terminals;
    match method {
        OPEN_TERMINAL => Some(with_params(id, params, |opening: TerminalOpening| {
            settle(id, open(opening, shared))
        })),
        WRITE_TERMINAL => Some(with_params(id, params, |input: TerminalInput| {
            let written = terminals.write(input.terminal, &input.data);
            settle(id, written.map(|()| json!({})))
        })),
        RESIZE_TERMINAL => Some(with_params(id, params, |size: TerminalSize| {
            let resized = terminals.resize(size.terminal, size.cols, size.rows);
            settle(id, resized.map(|()| json!({})))
        })),
        READ_TERMINAL => Some(with_params(id, params, |reading: TerminalReading| {
            let TerminalReading {
                terminal,
                after,
                wait_ms,
            } = reading;
            settle(id, terminals.read(terminal, after, wait_ms))
        })),
        _ => None,
    }
}

fn open(opening: TerminalOpening, shared: &Shared) -> Result<OpenedTerminal, Refusal> {
    let TerminalOpening {
        location,
        id: agent,
        cols,
        rows,
    } = opening;
    if location == Location::Cloud {
        return Err(Refusal::Invalid(
            "a cloud task has no terminal: its shell would run on another machine".to_owned(),
        ));
    }
    let known = lock(&shared.store)
        .agents()
        .map_err(|error| Refusal::Failed(error.to_string()))?;
    let folder = known
        .into_iter()
        .find(|stored| stored.id == agent)
        .map(|stored| stored.folder)
        .ok_or_else(|| Refusal::Invalid(format!("there is no local agent with ID {agent}")))?;
    let terminal = shared
        .terminals
        .open(agent, Path::new(&folder), cols, rows)?;
    Ok(OpenedTerminal { terminal })
}

fn place(id: u64, placement: FloorPlacement, shared: &Shared) -> Response {
    let FloorPlacement {
        location,
        id: agent,
        x,
        y,
    } = placement;
    if x < 0 || y < 0 {
        return Response::failure(
            id,
            INVALID_PARAMS,
            format!("a floor position cannot be negative, and ({x}, {y}) is"),
        );
    }
    match lock(&shared.store).place_on_floor(location, agent, FloorPosition { x, y }) {
        Ok(true) => Response::success(id, json!({ "id": agent })),
        Ok(false) => Response::failure(
            id,
            INVALID_PARAMS,
            format!("there is no {} agent with ID {agent}", location.as_str()),
        ),
        Err(error) => Response::failure(id, INTERNAL_ERROR, error.to_string()),
    }
}

fn branch_problem(branch: &str) -> Option<String> {
    let unusable =
        |character: char| character.is_whitespace() || character.is_control() || character == ':';
    (branch.is_empty() || branch.contains(unusable))
        .then(|| format!("`{branch}` is not a branch name"))
}

fn with_params<T: DeserializeOwned>(
    id: u64,
    params: Value,
    run: impl FnOnce(T) -> Response,
) -> Response {
    match serde_json::from_value(params) {
        Ok(parsed) => run(parsed),
        Err(error) => Response::failure(id, INVALID_PARAMS, error.to_string()),
    }
}

fn call(socket: &Path, method: &str, params: &Value) -> Result<Value, Error> {
    let stream = UnixStream::connect(socket)?;
    let request = json!({
        "jsonrpc": VERSION,
        "id": 1,
        "method": method,
        "params": params,
    });
    let mut writer = stream.try_clone()?;
    writeln!(writer, "{request}")?;
    let mut line = String::new();
    if BufReader::new(stream).read_line(&mut line)? == 0 {
        return Err(Error::NoResponse);
    }
    let response: Response = serde_json::from_str(&line)?;
    if let Some(error) = response.error {
        return Err(Error::Rpc(error));
    }
    response.result.ok_or(Error::NoResponse)
}

fn row_id(result: &Value) -> Result<i64, Error> {
    result
        .get("id")
        .and_then(Value::as_i64)
        .ok_or(Error::NoResponse)
}

/// Tells the daemon at `socket` that a compaction happened and returns the
/// ID of the row it stored.
///
/// # Errors
///
/// Returns [`Error::Io`] if the daemon cannot be reached, [`Error::Rpc`] if it
/// answers with an error, and [`Error::NoResponse`] if it hangs up.
pub fn record_compaction(socket: &Path, report: &CompactionReport) -> Result<i64, Error> {
    row_id(&call(socket, RECORD_COMPACTION, &json!(report))?)
}

/// Tells the daemon at `socket` that a local agent started in `folder` and
/// returns the agent's ID. Registering the same folder again returns the same
/// ID.
///
/// # Errors
///
/// Returns [`Error::Io`] if the daemon cannot be reached, [`Error::Rpc`] if it
/// answers with an error, and [`Error::NoResponse`] if it hangs up.
pub fn register_agent(socket: &Path, folder: &str) -> Result<i64, Error> {
    let registration = AgentRegistration {
        folder: folder.to_owned(),
    };
    row_id(&call(socket, REGISTER_AGENT, &json!(registration))?)
}

/// Asks the daemon at `socket` for every agent it knows: local agents oldest
/// first, then cloud tasks oldest first. The daemon reads each local agent's
/// progress file when it answers, so a file that changed since the last call
/// shows its new content. A cloud task shows what the daemon read from its
/// branch in the latest polling round.
///
/// # Errors
///
/// Returns [`Error::Io`] if the daemon cannot be reached, [`Error::Rpc`] if it
/// answers with an error, [`Error::Json`] if the answer is not a list of
/// agents, and [`Error::NoResponse`] if it hangs up.
pub fn list_agents(socket: &Path) -> Result<Vec<AgentEntry>, Error> {
    let list: AgentList = serde_json::from_value(call(socket, LIST_AGENTS, &Value::Null)?)?;
    Ok(list.agents)
}

/// Tells the daemon at `socket` to follow the cloud task that pushes to
/// `branch` on the remote `origin` of the repository in `folder`, and returns
/// the task's ID. The daemon reads the branch before it answers. Tracking the
/// same folder and branch again returns the same ID.
///
/// # Errors
///
/// Returns [`Error::Io`] if the daemon cannot be reached, [`Error::Rpc`] if it
/// answers with an error, such as for a branch name that no branch can have,
/// and [`Error::NoResponse`] if it hangs up.
pub fn track_cloud_task(socket: &Path, folder: &str, branch: &str) -> Result<i64, Error> {
    let tracking = CloudTracking {
        folder: folder.to_owned(),
        branch: branch.to_owned(),
    };
    row_id(&call(socket, TRACK_CLOUD_TASK, &json!(tracking))?)
}

/// Tells the daemon at `socket` where an agent's tile sits on the floor and
/// returns the agent's ID. The next [`list_agents`] reports the position, and
/// it survives a daemon restart.
///
/// # Errors
///
/// Returns [`Error::Io`] if the daemon cannot be reached, [`Error::Rpc`] if it
/// answers with an error, such as for a negative coordinate or an agent it
/// does not know, and [`Error::NoResponse`] if it hangs up.
pub fn place_on_floor(socket: &Path, placement: &FloorPlacement) -> Result<i64, Error> {
    row_id(&call(socket, PLACE_ON_FLOOR, &json!(placement))?)
}

/// Asks the daemon at `socket` for a local agent's terminal and returns the
/// terminal's ID. The daemon starts the agent's shell in its folder if none
/// is running; a running shell keeps its ID and takes the new size. The shell
/// outlives the caller and ends with the daemon.
///
/// # Errors
///
/// Returns [`Error::Io`] if the daemon cannot be reached, [`Error::Rpc`] if it
/// answers with an error, such as for a cloud task, an agent it does not know
/// or a size outside 1 to 1000, [`Error::Json`] if the answer is not a
/// terminal, and [`Error::NoResponse`] if it hangs up.
pub fn open_terminal(socket: &Path, opening: &TerminalOpening) -> Result<i64, Error> {
    let opened: OpenedTerminal =
        serde_json::from_value(call(socket, OPEN_TERMINAL, &json!(opening))?)?;
    Ok(opened.terminal)
}

/// Types `input.data` into a terminal of the daemon at `socket`, as the
/// keyboard would.
///
/// # Errors
///
/// Returns [`Error::Io`] if the daemon cannot be reached, [`Error::Rpc`] if it
/// answers with an error, such as for a terminal it does not know or a shell
/// that has exited, and [`Error::NoResponse`] if it hangs up.
pub fn write_terminal(socket: &Path, input: &TerminalInput) -> Result<(), Error> {
    call(socket, WRITE_TERMINAL, &json!(input)).map(|_| ())
}

/// Tells a terminal of the daemon at `socket` its new size, so the programs
/// in it redraw to fit.
///
/// # Errors
///
/// Returns [`Error::Io`] if the daemon cannot be reached, [`Error::Rpc`] if it
/// answers with an error, such as for a terminal it does not know or a size
/// outside 1 to 1000, and [`Error::NoResponse`] if it hangs up.
pub fn resize_terminal(socket: &Path, size: &TerminalSize) -> Result<(), Error> {
    call(socket, RESIZE_TERMINAL, &json!(size)).map(|_| ())
}

/// Reads a terminal's output from the daemon at `socket`. The call returns at
/// once when there is output past `reading.after` or the shell has exited, and
/// otherwise blocks for up to `reading.wait_ms`. Call it in a loop, passing
/// each answer's `next` as the following `after`; start from `0` to replay the
/// output the daemon still keeps.
///
/// # Errors
///
/// Returns [`Error::Io`] if the daemon cannot be reached, [`Error::Rpc`] if it
/// answers with an error, such as for a terminal it does not know or an
/// `after` past the end of the output, [`Error::Json`] if the answer is not
/// terminal output, and [`Error::NoResponse`] if it hangs up.
pub fn read_terminal(socket: &Path, reading: &TerminalReading) -> Result<TerminalOutput, Error> {
    Ok(serde_json::from_value(call(
        socket,
        READ_TERMINAL,
        &json!(reading),
    )?)?)
}
