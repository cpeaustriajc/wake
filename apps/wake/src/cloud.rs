use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use wake_daemon::track_cloud_task;

use crate::agents;
use crate::paths;

pub(crate) fn track(branch: &str) -> Result<ExitCode, String> {
    let folder = env::current_dir()
        .and_then(|folder| folder.canonicalize())
        .map_err(|error| error.to_string())?;
    let socket = paths::socket()?;
    let id = track_cloud_task(&socket, &folder.to_string_lossy(), branch)
        .map_err(|error| agents::daemon_down(&socket, &error))?;
    writeln!(
        io::stdout().lock(),
        "tracking cloud task {id}: branch {branch} of {}",
        folder.display()
    )
    .map_err(|error| error.to_string())?;
    Ok(ExitCode::SUCCESS)
}
