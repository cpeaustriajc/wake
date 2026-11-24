//! The daemon's Unix socket and the client that talks to it.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use serde_json::{Value, json};

use crate::error::Error;
use crate::protocol::{
    CompactionReport, INTERNAL_ERROR, INVALID_PARAMS, METHOD_NOT_FOUND, PARSE_ERROR,
    RECORD_COMPACTION, Request, Response, VERSION,
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

fn answer(line: &str, store: &Mutex<Store>) -> Response {
    let request: Request = match serde_json::from_str(line) {
        Ok(request) => request,
        Err(error) => return Response::failure(0, PARSE_ERROR, error.to_string()),
    };
    if request.jsonrpc != VERSION || request.method != RECORD_COMPACTION {
        return Response::failure(
            request.id,
            METHOD_NOT_FOUND,
            format!("unknown method `{}`", request.method),
        );
    }
    let report: CompactionReport = match serde_json::from_value(request.params) {
        Ok(report) => report,
        Err(error) => return Response::failure(request.id, INVALID_PARAMS, error.to_string()),
    };
    let stored = store
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .record_compaction(&report);
    match stored {
        Ok(id) => Response::success(request.id, json!({ "id": id })),
        Err(error) => Response::failure(request.id, INTERNAL_ERROR, error.to_string()),
    }
}

/// Tells the daemon at `socket` that a compaction happened and returns the
/// ID of the row it stored.
///
/// # Errors
///
/// Returns [`Error::Io`] if the daemon cannot be reached, [`Error::Rpc`] if it
/// answers with an error, and [`Error::NoResponse`] if it hangs up.
pub fn record_compaction(socket: &Path, report: &CompactionReport) -> Result<i64, Error> {
    let stream = UnixStream::connect(socket)?;
    let request = json!({
        "jsonrpc": VERSION,
        "id": 1,
        "method": RECORD_COMPACTION,
        "params": report,
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
    let result: Value = response.result.ok_or(Error::NoResponse)?;
    result
        .get("id")
        .and_then(Value::as_i64)
        .ok_or(Error::NoResponse)
}
