//! The `wake` command: the daemon, the reload hook, starting an agent, tracking a cloud task and listing agents.

mod agent;
mod agents;
mod cloud;
mod daemon;
mod hook;
mod paths;

use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

const USAGE: &str = "usage:
  wake daemon
  wake hook reload
  wake agent start <folder> [message...]
  wake cloud track <branch>
  wake agents";

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let names: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let outcome = match names.as_slice() {
        ["daemon"] => daemon::run(),
        ["hook", "reload"] => Ok(hook::reload()),
        ["agent", "start", folder, messages @ ..] => agent::start(folder, messages),
        ["cloud", "track", branch] => cloud::track(branch),
        ["agents"] => agents::run(),
        _ => Err(USAGE.to_owned()),
    };
    match outcome {
        Ok(code) => code,
        Err(message) => {
            let _ = writeln!(io::stderr().lock(), "wake: {message}");
            ExitCode::FAILURE
        }
    }
}
