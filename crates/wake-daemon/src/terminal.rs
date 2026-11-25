//! The shells the daemon runs for local agents, each on its own
//! pseudo-terminal with a bounded copy of its output.

use std::collections::{HashMap, VecDeque};
use std::env;
use std::fmt;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};

use crate::base64;
use crate::protocol::{MAX_TERMINAL_WAIT_MS, TerminalOutput};

const SCROLLBACK_BYTES: usize = 1024 * 1024;
const ANSWER_BYTES: usize = 64 * 1024;
const READ_BUFFER_BYTES: usize = 8 * 1024;
const LARGEST_SIDE: u16 = 1000;
const TERM_VARIABLE: &str = "TERM";
const TERM: &str = "xterm-256color";
const SHELL_VARIABLE: &str = "SHELL";
const FALLBACK_SHELL: &str = "/bin/sh";
const UNREADABLE_STATUS: i64 = -1;
const LAST_OUTPUT_GRACE: Duration = Duration::from_millis(250);

pub(crate) enum Refusal {
    Invalid(String),
    Failed(String),
}

fn failed(error: impl fmt::Display) -> Refusal {
    Refusal::Failed(error.to_string())
}

fn wide(count: usize) -> u64 {
    u64::try_from(count).unwrap_or(u64::MAX)
}

pub(crate) fn user_shell() -> PathBuf {
    env::var_os(SHELL_VARIABLE)
        .filter(|shell| !shell.is_empty())
        .map_or_else(|| PathBuf::from(FALLBACK_SHELL), PathBuf::from)
}

fn size(cols: u16, rows: u16) -> Result<PtySize, Refusal> {
    let fits = |side: u16| (1..=LARGEST_SIDE).contains(&side);
    if fits(cols) && fits(rows) {
        Ok(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
    } else {
        Err(Refusal::Invalid(format!(
            "a terminal is 1 to {LARGEST_SIDE} cells on each side, and {cols} by {rows} is not"
        )))
    }
}

#[derive(Default)]
struct Scrollback {
    kept: VecDeque<u8>,
    dropped: u64,
    closed: bool,
    exited: Option<i64>,
}

impl Scrollback {
    fn end(&self) -> u64 {
        self.dropped.saturating_add(wide(self.kept.len()))
    }

    fn answer(&self, after: u64) -> TerminalOutput {
        let from = after.max(self.dropped);
        let skipped = usize::try_from(from - self.dropped).unwrap_or(usize::MAX);
        let bytes: Vec<u8> = self
            .kept
            .iter()
            .skip(skipped)
            .take(ANSWER_BYTES)
            .copied()
            .collect();
        let next = from.saturating_add(wide(bytes.len()));
        TerminalOutput {
            data: base64::encode(&bytes),
            next,
            truncated: after < self.dropped,
            exited: self.exited.filter(|_| next == self.end()),
        }
    }
}

#[derive(Default)]
struct Output {
    scrollback: Mutex<Scrollback>,
    changed: Condvar,
}

impl Output {
    fn lock(&self) -> MutexGuard<'_, Scrollback> {
        self.scrollback
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn append(&self, bytes: &[u8]) {
        let mut scrollback = self.lock();
        scrollback.kept.extend(bytes);
        let excess = scrollback.kept.len().saturating_sub(SCROLLBACK_BYTES);
        scrollback.kept.drain(..excess);
        scrollback.dropped = scrollback.dropped.saturating_add(wide(excess));
        drop(scrollback);
        self.changed.notify_all();
    }

    fn close(&self) {
        self.lock().closed = true;
        self.changed.notify_all();
    }

    fn exit(&self, code: i64) {
        let deadline = Instant::now() + LAST_OUTPUT_GRACE;
        let mut scrollback = self.lock();
        while !scrollback.closed {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            scrollback = self
                .changed
                .wait_timeout(scrollback, remaining)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        scrollback.exited = Some(code);
        drop(scrollback);
        self.changed.notify_all();
    }

    fn has_exited(&self) -> bool {
        self.lock().exited.is_some()
    }

    fn read(&self, after: u64, wait: Duration) -> Result<TerminalOutput, Refusal> {
        let deadline = Instant::now() + wait;
        let mut scrollback = self.lock();
        loop {
            let end = scrollback.end();
            if after > end {
                return Err(Refusal::Invalid(format!(
                    "the terminal has written {end} bytes, and {after} is past that"
                )));
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if after < end || scrollback.exited.is_some() || remaining.is_zero() {
                return Ok(scrollback.answer(after));
            }
            scrollback = self
                .changed
                .wait_timeout(scrollback, remaining)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }
}

fn copy(mut reader: Box<dyn Read + Send>, output: &Output) {
    let mut buffer = vec![0_u8; READ_BUFFER_BYTES];
    while let Ok(count) = reader.read(&mut buffer) {
        if count == 0 {
            break;
        }
        output.append(buffer.get(..count).unwrap_or(&[]));
    }
    output.close();
}

fn wait(mut child: Box<dyn Child + Send + Sync>, output: &Output) {
    let code = child
        .wait()
        .map_or(UNREADABLE_STATUS, |status| i64::from(status.exit_code()));
    output.exit(code);
}

struct Session {
    agent: i64,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    output: Arc<Output>,
}

fn start(shell: &Path, agent: i64, folder: &Path, size: PtySize) -> Result<Session, Refusal> {
    let pair = native_pty_system().openpty(size).map_err(failed)?;
    let mut command = CommandBuilder::new(shell);
    command.cwd(folder);
    command.env(TERM_VARIABLE, TERM);
    let child = pair.slave.spawn_command(command).map_err(failed)?;
    drop(pair.slave);
    let reader = pair.master.try_clone_reader().map_err(failed)?;
    let writer = pair.master.take_writer().map_err(failed)?;
    let output = Arc::new(Output::default());
    let copied = Arc::clone(&output);
    thread::spawn(move || copy(reader, &copied));
    let waited = Arc::clone(&output);
    thread::spawn(move || wait(child, &waited));
    Ok(Session {
        agent,
        master: pair.master,
        writer,
        output,
    })
}

#[derive(Default)]
struct Sessions {
    last: i64,
    by_terminal: HashMap<i64, Session>,
}

fn unknown(terminal: i64) -> Refusal {
    Refusal::Invalid(format!("there is no terminal with ID {terminal}"))
}

pub(crate) struct Terminals {
    shell: PathBuf,
    sessions: Mutex<Sessions>,
}

impl fmt::Debug for Terminals {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Terminals")
            .field("shell", &self.shell)
            .field("sessions", &self.lock().by_terminal.len())
            .finish()
    }
}

impl Terminals {
    pub(crate) fn new(shell: PathBuf) -> Self {
        Self {
            shell,
            sessions: Mutex::new(Sessions::default()),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Sessions> {
        self.sessions.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn open(
        &self,
        agent: i64,
        folder: &Path,
        cols: u16,
        rows: u16,
    ) -> Result<i64, Refusal> {
        let size = size(cols, rows)?;
        let mut sessions = self.lock();
        let running = sessions
            .by_terminal
            .iter()
            .find(|(_, session)| session.agent == agent && !session.output.has_exited());
        if let Some((&terminal, session)) = running {
            session.master.resize(size).map_err(failed)?;
            return Ok(terminal);
        }
        if !folder.is_dir() {
            return Err(Refusal::Invalid(format!(
                "the agent's folder {} is not there",
                folder.display()
            )));
        }
        sessions
            .by_terminal
            .retain(|_, session| session.agent != agent);
        let session = start(&self.shell, agent, folder, size)?;
        sessions.last += 1;
        let terminal = sessions.last;
        sessions.by_terminal.insert(terminal, session);
        Ok(terminal)
    }

    pub(crate) fn write(&self, terminal: i64, data: &str) -> Result<(), Refusal> {
        let mut sessions = self.lock();
        let session = sessions
            .by_terminal
            .get_mut(&terminal)
            .ok_or_else(|| unknown(terminal))?;
        if session.output.has_exited() {
            return Err(Refusal::Invalid(format!(
                "the shell in terminal {terminal} has exited"
            )));
        }
        session.writer.write_all(data.as_bytes()).map_err(failed)?;
        session.writer.flush().map_err(failed)
    }

    pub(crate) fn resize(&self, terminal: i64, cols: u16, rows: u16) -> Result<(), Refusal> {
        let size = size(cols, rows)?;
        let sessions = self.lock();
        let session = sessions
            .by_terminal
            .get(&terminal)
            .ok_or_else(|| unknown(terminal))?;
        session.master.resize(size).map_err(failed)
    }

    pub(crate) fn read(
        &self,
        terminal: i64,
        after: u64,
        wait_ms: u64,
    ) -> Result<TerminalOutput, Refusal> {
        let output = self
            .lock()
            .by_terminal
            .get(&terminal)
            .map(|session| Arc::clone(&session.output))
            .ok_or_else(|| unknown(terminal))?;
        output.read(
            after,
            Duration::from_millis(wait_ms.min(MAX_TERMINAL_WAIT_MS)),
        )
    }
}
