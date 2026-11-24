use std::env;
use std::path::PathBuf;

use wake_claude::agent::SOCKET_ENV;

const DATABASE_ENV: &str = "WAKE_DB";
const STATE_FOLDER: &str = ".wake";
const SOCKET_FILE: &str = "wake.sock";
const DATABASE_FILE: &str = "wake.db";

fn state_folder() -> Result<PathBuf, String> {
    let home = env::var_os("HOME").ok_or("HOME is not set")?;
    Ok(PathBuf::from(home).join(STATE_FOLDER))
}

pub(crate) fn socket() -> Result<PathBuf, String> {
    match env::var_os(SOCKET_ENV) {
        Some(path) => Ok(PathBuf::from(path)),
        None => Ok(state_folder()?.join(SOCKET_FILE)),
    }
}

pub(crate) fn database() -> Result<PathBuf, String> {
    match env::var_os(DATABASE_ENV) {
        Some(path) => Ok(PathBuf::from(path)),
        None => Ok(state_folder()?.join(DATABASE_FILE)),
    }
}
