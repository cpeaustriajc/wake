//! Starting an agent: the command it runs and reading its output.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use wake_claude::agent::{Agent, AgentConfig, SOCKET_ENV, command, hook_settings};
use wake_claude::stream::Event;

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn scratch() -> PathBuf {
    let folder = std::env::temp_dir().join(format!(
        "wake-claude-test-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    fs::create_dir_all(&folder).expect("scratch folder");
    folder
}

fn config(folder: &Path, claude: &Path) -> AgentConfig {
    AgentConfig {
        folder: folder.to_owned(),
        claude: claude.to_owned(),
        wake: PathBuf::from("/opt/it's/wake"),
        socket: folder.join("wake.sock"),
    }
}

#[test]
fn the_hook_runs_wake_hook_reload_after_compaction() {
    let settings = hook_settings(Path::new("/opt/it's/wake"));
    let entry = &settings["hooks"]["SessionStart"][0];
    assert_eq!(entry["matcher"], "compact", "matcher");
    assert_eq!(entry["hooks"][0]["type"], "command", "hook type");
    assert_eq!(
        entry["hooks"][0]["command"], "'/opt/it'\\''s/wake' hook reload",
        "the binary is quoted for the shell"
    );
}

#[test]
fn drives_claude_through_stream_json_with_the_hook_for_this_run_only() {
    let folder = scratch();
    let command = command(&config(&folder, Path::new("claude")));
    let arguments: Vec<String> = command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect();
    let position = |flag: &str| {
        arguments
            .iter()
            .position(|argument| argument == flag)
            .unwrap_or_else(|| panic!("{flag} is passed"))
    };
    assert_eq!(
        arguments[position("--input-format") + 1],
        "stream-json",
        "input"
    );
    assert_eq!(
        arguments[position("--output-format") + 1],
        "stream-json",
        "output"
    );
    assert!(arguments.contains(&"-p".to_owned()), "print mode");
    assert!(
        arguments[position("--settings") + 1].contains("hook reload"),
        "settings carry the hook"
    );
    assert_eq!(command.get_current_dir(), Some(folder.as_path()), "folder");
    let socket = command
        .get_envs()
        .find(|(name, _)| *name == SOCKET_ENV)
        .and_then(|(_, value)| value)
        .expect("the hook is told the socket");
    assert_eq!(socket, folder.join("wake.sock"), "socket");
}

#[test]
fn changes_no_compaction_setting() {
    let folder = scratch();
    let command = command(&config(&folder, Path::new("claude")));
    let mut settings = command
        .get_args()
        .map(|argument| argument.to_string_lossy().to_lowercase())
        .collect::<Vec<_>>();
    settings.extend(
        command
            .get_envs()
            .map(|(name, _)| name.to_string_lossy().to_lowercase()),
    );
    let touching: Vec<&String> = settings
        .iter()
        .filter(|word| word.contains("autocompact") || word.contains("compact_"))
        .collect();
    assert!(
        touching.is_empty(),
        "compaction settings passed: {touching:?}"
    );
}

#[test]
fn replays_a_recording_through_a_started_agent() {
    let folder = scratch();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/session.jsonl");
    let script = folder.join("claude");
    fs::write(&script, format!("#!/bin/sh\ncat '{}'\n", fixture.display())).expect("script");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).expect("executable");

    let mut agent = Agent::start(&config(&folder, &script)).expect("starts");
    let messages = agent.read_turn().expect("readable");
    assert!(
        messages
            .iter()
            .any(|message| matches!(message.event, Event::Init(_))),
        "init was read"
    );
    assert!(
        messages
            .iter()
            .any(|message| matches!(message.event, Event::RateLimit(_))),
        "rate limit was read"
    );
    assert!(
        messages
            .iter()
            .any(|message| matches!(message.event, Event::AutocompactState(_))),
        "autocompact state was read"
    );
    assert!(
        matches!(
            messages.last().map(|message| &message.event),
            Some(Event::Result(_))
        ),
        "the turn ended with a result"
    );
    assert!(agent.finish().expect("exits"), "the process exited cleanly");
}
