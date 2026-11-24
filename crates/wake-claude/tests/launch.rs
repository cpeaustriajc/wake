//! Starting an agent with the hook left out or extra arguments added.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use wake_claude::agent::{AgentConfig, Launch, SOCKET_ENV, command, command_with};

fn config() -> AgentConfig {
    AgentConfig {
        folder: PathBuf::from("/work/repo"),
        claude: PathBuf::from("claude"),
        wake: PathBuf::from("/opt/wake"),
        socket: PathBuf::from("/work/wake.sock"),
    }
}

fn arguments(launch: &Launch) -> Vec<String> {
    command_with(&config(), launch)
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn the_default_launch_is_the_command_with_the_hook() {
    let plain: Vec<String> = command(&config())
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect();
    assert_eq!(plain, arguments(&Launch::default()), "same arguments");
    assert!(
        plain.contains(&"--settings".to_owned()),
        "the hook is attached"
    );
}

#[test]
fn a_launch_without_the_hook_passes_no_settings_but_keeps_the_rest() {
    let launch = Launch {
        reload_hook: false,
        extra_arguments: Vec::new(),
    };
    let arguments = arguments(&launch);
    assert!(
        !arguments.contains(&"--settings".to_owned()),
        "no settings: {arguments:?}"
    );
    assert!(
        arguments.contains(&"--append-system-prompt".to_owned()),
        "the progress instructions stay"
    );
    assert!(arguments.contains(&"-p".to_owned()), "print mode");
}

#[test]
fn extra_arguments_come_last_and_the_environment_is_unchanged() {
    let launch = Launch {
        reload_hook: true,
        extra_arguments: vec![
            OsString::from("--permission-mode"),
            OsString::from("acceptEdits"),
        ],
    };
    let arguments = arguments(&launch);
    assert_eq!(
        &arguments[arguments.len() - 2..],
        ["--permission-mode", "acceptEdits"],
        "extras last"
    );
    let command = command_with(&config(), &launch);
    let socket = command
        .get_envs()
        .find(|(name, _)| *name == SOCKET_ENV)
        .and_then(|(_, value)| value)
        .expect("the socket is passed");
    assert_eq!(socket, Path::new("/work/wake.sock"), "socket");
}
