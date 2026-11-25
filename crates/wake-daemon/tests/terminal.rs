//! Agent terminals, over a real Unix socket with a real `/bin/sh` on a real
//! pseudo-terminal.

use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use wake_daemon::protocol::{
    INVALID_PARAMS, Location, TerminalInput, TerminalOpening, TerminalOutput, TerminalReading,
    TerminalSize,
};
use wake_daemon::{
    DEFAULT_POLL_INTERVAL, Error, Store, bind, list_agents, open_terminal, read_terminal,
    register_agent, resize_terminal, serve_with_shell, track_cloud_task, write_terminal,
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const SHELL: &str = "/bin/sh";
const BOUND: Duration = Duration::from_secs(20);
const STEP_MS: u64 = 200;
const COLS: u16 = 80;
const ROWS: u16 = 24;

fn scratch() -> PathBuf {
    let folder = std::env::temp_dir().join(format!(
        "wake-terminal-test-{}-{}",
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
        let _ = serve_with_shell(&listener, store, DEFAULT_POLL_INTERVAL, Path::new(SHELL));
    });
    socket
}

fn opening(id: i64, cols: u16, rows: u16) -> TerminalOpening {
    TerminalOpening {
        location: Location::Local,
        id,
        cols,
        rows,
    }
}

fn opened(socket: &Path, parent: &Path, name: &str) -> i64 {
    let agent = register_agent(socket, &workspace(parent, name)).expect("registered");
    open_terminal(socket, &opening(agent, COLS, ROWS)).expect("opened")
}

fn typed(socket: &Path, terminal: i64, data: &str) {
    let input = TerminalInput {
        terminal,
        data: data.to_owned(),
    };
    write_terminal(socket, &input).expect("written");
}

fn reading(terminal: i64, after: u64, wait_ms: u64) -> TerminalReading {
    TerminalReading {
        terminal,
        after,
        wait_ms,
    }
}

#[derive(Debug, Default)]
struct Seen {
    bytes: Vec<u8>,
    next: u64,
    exited: Option<i64>,
    truncated: bool,
    last_was_empty: bool,
}

impl Seen {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }
}

fn read_until(socket: &Path, terminal: i64, from: u64, done: impl Fn(&Seen) -> bool) -> Seen {
    let deadline = Instant::now() + BOUND;
    let mut seen = Seen {
        next: from,
        ..Seen::default()
    };
    while !done(&seen) {
        assert!(
            Instant::now() < deadline,
            "the terminal never showed what the test waits for; it showed {:?}",
            seen.text()
        );
        let output =
            read_terminal(socket, &reading(terminal, seen.next, STEP_MS)).expect("output read");
        let bytes = output.bytes().expect("the daemon sends base64");
        seen.last_was_empty = bytes.is_empty();
        seen.bytes.extend(bytes);
        seen.next = output.next;
        seen.exited = output.exited;
        seen.truncated = seen.truncated || output.truncated;
    }
    seen
}

fn until_shown(socket: &Path, terminal: i64, from: u64, needle: &str) -> Seen {
    read_until(socket, terminal, from, |seen| seen.text().contains(needle))
}

fn until_quiet(socket: &Path, terminal: i64, from: u64) -> Seen {
    read_until(socket, terminal, from, |seen| seen.last_was_empty)
}

fn refusal<T: Debug>(outcome: Result<T, Error>) -> (i64, String) {
    match outcome {
        Err(Error::Rpc(error)) => (error.code, error.message),
        other => panic!("expected the daemon to refuse, got {other:?}"),
    }
}

#[test]
fn a_command_typed_into_an_agents_terminal_runs_in_the_agents_folder() {
    let folder = scratch();
    let socket = start(&folder);
    let terminal = opened(&socket, &folder, "agent-one");

    typed(
        &socket,
        terminal,
        "echo wake-$((40 + 2))-in-$(basename \"$PWD\")\n",
    );

    let seen = until_shown(&socket, terminal, 0, "wake-42-in-agent-one");
    assert_eq!(seen.exited, None, "the shell is still running");
    assert!(!seen.truncated, "nothing was dropped from so little output");
}

#[test]
fn the_shell_sees_a_terminal_that_speaks_xterm() {
    let folder = scratch();
    let socket = start(&folder);
    let terminal = opened(&socket, &folder, "agent");

    typed(
        &socket,
        terminal,
        "test -t 0 && echo \"term-is-$TERM-on-a-tty\"\n",
    );

    until_shown(&socket, terminal, 0, "term-is-xterm-256color-on-a-tty");
}

#[test]
fn a_later_connection_replays_the_output_from_the_start() {
    let folder = scratch();
    let socket = start(&folder);
    let terminal = opened(&socket, &folder, "agent");
    typed(&socket, terminal, "echo first-$((1 + 1))\n");
    let first = until_shown(&socket, terminal, 0, "first-2");
    typed(&socket, terminal, "echo second-$((2 + 2))\n");
    until_shown(&socket, terminal, first.next, "second-4");

    let replayed = until_shown(&socket, terminal, 0, "second-4");

    assert!(
        replayed.text().contains("first-2"),
        "a read from offset 0 starts with the earlier output: {:?}",
        replayed.text()
    );
    assert!(!replayed.truncated, "the daemon still keeps all of it");
}

#[test]
fn reading_from_the_last_offset_never_repeats_or_skips_bytes() {
    let folder = scratch();
    let socket = start(&folder);
    let terminal = opened(&socket, &folder, "agent");
    typed(
        &socket,
        terminal,
        "echo one-$((1 + 0)); echo two-$((1 + 1))\n",
    );
    let whole = until_quiet(
        &socket,
        terminal,
        until_shown(&socket, terminal, 0, "two-2").next,
    );

    let from_start = until_quiet(&socket, terminal, 0);
    let half = from_start.next / 2;
    let head = read_terminal(&socket, &reading(terminal, 0, 0)).expect("read");
    let tail = until_quiet(&socket, terminal, half);

    assert_eq!(
        from_start.next, whole.next,
        "both reads end at the same offset"
    );
    assert_eq!(
        head.next, from_start.next,
        "one answer holds this little output"
    );
    let head_bytes = head.bytes().expect("base64");
    let cut = usize::try_from(half).expect("a small offset");
    assert_eq!(
        tail.bytes,
        head_bytes[cut..],
        "a read from the middle gives exactly the second half"
    );
}

#[test]
fn a_resize_reaches_the_shell_and_opening_again_keeps_the_terminal() {
    let folder = scratch();
    let socket = start(&folder);
    let agent = register_agent(&socket, &workspace(&folder, "agent")).expect("registered");
    let terminal = open_terminal(&socket, &opening(agent, COLS, ROWS)).expect("opened");

    let size = TerminalSize {
        terminal,
        cols: 100,
        rows: 30,
    };
    resize_terminal(&socket, &size).expect("resized");
    typed(&socket, terminal, "echo size-is-$(stty size | tr ' ' x)\n");
    let resized = until_shown(&socket, terminal, 0, "size-is-30x100");

    let again = open_terminal(&socket, &opening(agent, 120, 40)).expect("opened again");
    typed(
        &socket,
        terminal,
        "echo now-it-is-$(stty size | tr ' ' x)\n",
    );
    until_shown(&socket, terminal, resized.next, "now-it-is-40x120");

    assert_eq!(again, terminal, "a running shell keeps its terminal");
}

#[test]
fn an_exit_reports_its_code_and_the_next_open_starts_a_fresh_shell() {
    let folder = scratch();
    let socket = start(&folder);
    let agent = register_agent(&socket, &workspace(&folder, "agent")).expect("registered");
    let terminal = open_terminal(&socket, &opening(agent, COLS, ROWS)).expect("opened");

    typed(&socket, terminal, "exit 3\n");
    let ended = read_until(&socket, terminal, 0, |seen| seen.exited.is_some());
    let input = TerminalInput {
        terminal,
        data: "echo too-late\n".to_owned(),
    };
    let late = refusal(write_terminal(&socket, &input));
    let fresh = open_terminal(&socket, &opening(agent, COLS, ROWS)).expect("opened again");
    typed(&socket, fresh, "echo fresh-$((6 * 7))\n");
    let alive = until_shown(&socket, fresh, 0, "fresh-42");
    let old = refusal(read_terminal(&socket, &reading(terminal, 0, 0)));

    assert_eq!(ended.exited, Some(3), "the shell's own exit code");
    assert_eq!(
        late,
        (
            INVALID_PARAMS,
            format!("the shell in terminal {terminal} has exited")
        ),
        "typing into a shell that exited is refused"
    );
    assert_ne!(fresh, terminal, "a new shell is a new terminal");
    assert_eq!(alive.exited, None, "the new shell runs");
    assert_eq!(
        old,
        (
            INVALID_PARAMS,
            format!("there is no terminal with ID {terminal}")
        ),
        "the exited shell's terminal is gone once a new one started"
    );
}

#[test]
fn a_read_with_nothing_new_waits_and_comes_back_empty() {
    const WAIT_MS: u64 = 400;
    let folder = scratch();
    let socket = start(&folder);
    let terminal = opened(&socket, &folder, "agent");
    typed(&socket, terminal, "echo settled-$((2 + 3))\n");
    let settled = until_quiet(
        &socket,
        terminal,
        until_shown(&socket, terminal, 0, "settled-5").next,
    );

    let before = Instant::now();
    let output = read_terminal(&socket, &reading(terminal, settled.next, WAIT_MS)).expect("read");
    let waited = before.elapsed();

    assert_eq!(
        output,
        TerminalOutput {
            data: String::new(),
            next: settled.next,
            truncated: false,
            exited: None,
        },
        "nothing new, same offset, shell still running"
    );
    assert!(
        waited >= Duration::from_millis(WAIT_MS - 50),
        "the read waited for output before giving up: {waited:?}"
    );
    assert!(waited < BOUND, "the wait is bounded: {waited:?}");
}

#[test]
fn a_waiting_read_returns_as_soon_as_output_arrives_and_blocks_nobody_else() {
    let folder = scratch();
    let socket = start(&folder);
    let terminal = opened(&socket, &folder, "agent");
    typed(&socket, terminal, "echo ready-$((1 + 1))\n");
    let settled = until_quiet(
        &socket,
        terminal,
        until_shown(&socket, terminal, 0, "ready-2").next,
    );
    let reader_socket = socket.clone();
    let waiting = thread::spawn(move || {
        let before = Instant::now();
        let output = read_terminal(&reader_socket, &reading(terminal, settled.next, 15_000));
        (output, before.elapsed())
    });
    thread::sleep(Duration::from_millis(200));

    let before = Instant::now();
    let agents = list_agents(&socket).expect("listed while a read waits");
    let listed_in = before.elapsed();
    typed(&socket, terminal, "echo woke-$((3 + 4))\n");
    let (output, waited) = waiting.join().expect("the reading thread ends");

    assert_eq!(agents.len(), 1, "the list answers");
    assert!(
        listed_in < Duration::from_secs(5),
        "a waiting read does not hold up other requests: {listed_in:?}"
    );
    assert!(
        !output.expect("read").data.is_empty(),
        "the waiting read came back with the new output"
    );
    assert!(
        waited < Duration::from_secs(10),
        "it came back when output arrived, not when its wait ran out: {waited:?}"
    );
}

#[test]
fn two_agents_terminals_do_not_mix() {
    let folder = scratch();
    let socket = start(&folder);
    let first = opened(&socket, &folder, "first");
    let second = opened(&socket, &folder, "second");

    typed(&socket, first, "echo from-first-$((10 + 1))\n");
    typed(&socket, second, "echo from-second-$((20 + 2))\n");
    let first_seen = until_quiet(
        &socket,
        first,
        until_shown(&socket, first, 0, "from-first-11").next,
    );
    let second_seen = until_quiet(
        &socket,
        second,
        until_shown(&socket, second, 0, "from-second-22").next,
    );
    let first_all = until_quiet(&socket, first, 0).text();
    let second_all = until_quiet(&socket, second, 0).text();

    assert_ne!(first, second, "each agent has its own terminal");
    assert_eq!(first_seen.exited, None, "the first shell runs");
    assert_eq!(second_seen.exited, None, "the second shell runs");
    assert!(
        !first_all.contains("second"),
        "the first terminal shows nothing of the second: {first_all:?}"
    );
    assert!(
        !second_all.contains("first"),
        "the second terminal shows nothing of the first: {second_all:?}"
    );
}

#[test]
fn bytes_that_are_not_text_arrive_unchanged() {
    let folder = scratch();
    let socket = start(&folder);
    let terminal = opened(&socket, &folder, "agent");

    typed(&socket, terminal, "printf 'raw-\\377\\376\\200-end\\n'\n");

    let seen = read_until(&socket, terminal, 0, |seen| {
        seen.bytes
            .windows(3)
            .any(|window| window == [0xff, 0xfe, 0x80])
    });
    assert!(
        seen.bytes
            .windows(7)
            .any(|window| window == [b'r', b'a', b'w', b'-', 0xff, 0xfe, 0x80]),
        "the bytes arrive in order, untouched"
    );
}

#[test]
fn output_past_a_mebibyte_drops_its_start_and_says_so() {
    const MEBIBYTE: u64 = 1024 * 1024;
    let folder = scratch();
    let socket = start(&folder);
    let terminal = opened(&socket, &folder, "agent");

    typed(
        &socket,
        terminal,
        "head -c 1200000 /dev/zero | tr '\\0' x; echo; echo flood-$((5 + 5))-over\n",
    );
    let end = until_shown(&socket, terminal, 0, "flood-10-over").next;
    let settled = until_quiet(&socket, terminal, end);
    let replayed = until_quiet(&socket, terminal, 0);
    let kept = u64::try_from(replayed.bytes.len()).expect("fits");

    assert!(
        settled.next > 1_200_000,
        "the offsets count every byte the shell wrote: {}",
        settled.next
    );
    assert!(
        replayed.truncated,
        "a read from 0 says the start was dropped"
    );
    assert_eq!(kept, MEBIBYTE, "the daemon keeps the last mebibyte");
    assert_eq!(replayed.next, settled.next, "the replay ends at the end");
    assert!(
        replayed.text().contains("flood-10-over"),
        "the newest output is what is kept"
    );
}

#[test]
fn a_cloud_task_an_unknown_agent_and_a_missing_folder_get_no_terminal() {
    let folder = scratch();
    let socket = start(&folder);
    let cloud =
        track_cloud_task(&socket, &workspace(&folder, "repository"), "claude/task").expect("ok");
    let gone = folder.join("gone").to_string_lossy().into_owned();
    let homeless = register_agent(&socket, &gone).expect("registered");

    let cloud_opening = TerminalOpening {
        location: Location::Cloud,
        id: cloud,
        cols: COLS,
        rows: ROWS,
    };
    let refused_cloud = refusal(open_terminal(&socket, &cloud_opening));
    let refused_unknown = refusal(open_terminal(&socket, &opening(99, COLS, ROWS)));
    let refused_homeless = refusal(open_terminal(&socket, &opening(homeless, COLS, ROWS)));

    assert_eq!(
        refused_cloud,
        (
            INVALID_PARAMS,
            "a cloud task has no terminal: its shell would run on another machine".to_owned()
        ),
        "a cloud task has no local shell"
    );
    assert_eq!(
        refused_unknown,
        (
            INVALID_PARAMS,
            "there is no local agent with ID 99".to_owned()
        ),
        "an agent the daemon does not know"
    );
    assert_eq!(
        refused_homeless,
        (
            INVALID_PARAMS,
            format!("the agent's folder {gone} is not there")
        ),
        "an agent whose folder is gone"
    );
}

#[test]
fn an_unknown_terminal_a_bad_size_and_an_offset_past_the_end_are_refused() {
    let folder = scratch();
    let socket = start(&folder);
    let agent = register_agent(&socket, &workspace(&folder, "agent")).expect("registered");
    let terminal = open_terminal(&socket, &opening(agent, COLS, ROWS)).expect("opened");
    let no_terminal = (INVALID_PARAMS, "there is no terminal with ID 99".to_owned());
    let input = TerminalInput {
        terminal: 99,
        data: "ls\n".to_owned(),
    };
    let size = |terminal: i64, cols: u16, rows: u16| TerminalSize {
        terminal,
        cols,
        rows,
    };

    assert_eq!(
        refusal(read_terminal(&socket, &reading(99, 0, 0))),
        no_terminal,
        "reading a terminal that does not exist"
    );
    assert_eq!(
        refusal(write_terminal(&socket, &input)),
        no_terminal,
        "typing into a terminal that does not exist"
    );
    assert_eq!(
        refusal(resize_terminal(&socket, &size(99, COLS, ROWS))),
        no_terminal,
        "resizing a terminal that does not exist"
    );
    assert_eq!(
        refusal(resize_terminal(&socket, &size(terminal, 0, ROWS))),
        (
            INVALID_PARAMS,
            "a terminal is 1 to 1000 cells on each side, and 0 by 24 is not".to_owned()
        ),
        "a terminal with no columns"
    );
    assert_eq!(
        refusal(open_terminal(&socket, &opening(agent, COLS, 1001))),
        (
            INVALID_PARAMS,
            "a terminal is 1 to 1000 cells on each side, and 80 by 1001 is not".to_owned()
        ),
        "a terminal taller than any screen"
    );
    let far = refusal(read_terminal(&socket, &reading(terminal, 1 << 40, 0)));
    assert_eq!(far.0, INVALID_PARAMS, "an offset the shell never reached");
    assert!(
        far.1.ends_with("bytes, and 1099511627776 is past that"),
        "the refusal names the offset: {}",
        far.1
    );
}

#[test]
fn output_decodes_from_base64_and_refuses_what_is_not() {
    let output = |data: &str| TerminalOutput {
        data: data.to_owned(),
        next: 0,
        truncated: false,
        exited: None,
    };

    assert_eq!(
        output("d2FrZQ==").bytes(),
        Some(b"wake".to_vec()),
        "two padding characters"
    );
    assert_eq!(
        output("d2FrZXM=").bytes(),
        Some(b"wakes".to_vec()),
        "one padding character"
    );
    assert_eq!(
        output("//7/").bytes(),
        Some(vec![0xff, 0xfe, 0xff]),
        "no padding"
    );
    assert_eq!(output("").bytes(), Some(Vec::new()), "nothing");
    assert_eq!(
        output("d2FrZQ=").bytes(),
        None,
        "not a multiple of four characters"
    );
    assert_eq!(
        output("d2F*ZQ==").bytes(),
        None,
        "a character outside the alphabet"
    );
    assert_eq!(output("d2==ZQ==").bytes(), None, "padding before the end");
    assert_eq!(output("d===").bytes(), None, "three padding characters");
}
