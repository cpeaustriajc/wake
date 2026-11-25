//! The daemon's Unix socket and the client that talks to it.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::agents::entry;
use crate::error::Error;
use crate::protocol::{
    AgentEntry, AgentList, AgentRegistration, CompactionReport, INTERNAL_ERROR, INVALID_PARAMS,
    LIST_AGENTS, Location, METHOD_NOT_FOUND, PARSE_ERROR, RECORD_COMPACTION, REGISTER_AGENT,
    Request, Response, VERSION,
};
use crate::store::Store;

const SOCKET_MODE: u32 = 0o600;

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

/// Answers requests on `listener` until it fails, one thread per connection.
///
/// # Errors
///
/// Returns [`Error::Io`] if accepting a connection fails.
pub fn serve(listener: &UnixListener, store: Store) -> Result<(), Error> {
    let store = Arc::new(Mutex::new(store));
    for stream in listener.incoming() {
        let stream = stream?;
        let store = Arc::clone(&store);
        thread::spawn(move || {
            let _ = handle(stream, &store);
        });
    }
    Ok(())
}

fn handle(stream: UnixStream, store: &Mutex<Store>) -> Result<(), Error> {
    let mut writer = stream.try_clone()?;
    for line in BufReader::new(stream).lines() {
        let response = answer(&line?, store);
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

fn answer(line: &str, store: &Mutex<Store>) -> Response {
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
            let stored = lock(store).record_compaction(&report);
            reply(id, stored.map(|row| json!({ "id": row })))
        }),
        REGISTER_AGENT => with_params(id, request.params, |agent: AgentRegistration| {
            let stored = lock(store).register_agent(&agent.folder, Location::Local);
            reply(id, stored.map(|row| json!({ "id": row })))
        }),
        LIST_AGENTS => {
            let stored = lock(store).agents();
            reply(
                id,
                stored.map(|agents| AgentList {
                    agents: agents.into_iter().map(entry).collect(),
                }),
            )
        }
        method => Response::failure(id, METHOD_NOT_FOUND, format!("unknown method `{method}`")),
    }
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

/// Asks the daemon at `socket` for every agent it knows, oldest first. The
/// daemon reads each agent's progress file when it answers, so a file that
/// changed since the last call shows its new content.
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
