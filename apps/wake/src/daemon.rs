use std::fs;
use std::io::{self, Write};
use std::process::ExitCode;

use wake_daemon::{Store, bind, serve_polling};

use crate::paths;

pub(crate) fn run() -> Result<ExitCode, String> {
    let socket = paths::socket()?;
    let database = paths::database()?;
    for path in [&socket, &database] {
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder).map_err(|error| error.to_string())?;
        }
    }
    let store = Store::open(&database).map_err(|error| error.to_string())?;
    let listener = bind(&socket).map_err(|error| error.to_string())?;
    let _ = writeln!(
        io::stdout().lock(),
        "wake daemon listening on {} and storing to {}",
        socket.display(),
        database.display()
    );
    serve_polling(&listener, store, paths::cloud_poll_interval()?)
        .map_err(|error| error.to_string())?;
    Ok(ExitCode::SUCCESS)
}
