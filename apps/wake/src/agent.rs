use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use wake_claude::agent::{Agent, AgentConfig, Message};
use wake_claude::stream::Event;

use crate::paths;

const CLAUDE_ENV: &str = "WAKE_CLAUDE";
const DEFAULT_CLAUDE: &str = "claude";

fn describe(event: &Event) -> Option<String> {
    match event {
        Event::Init(init) => Some(format!(
            "init session={} model={} claude={} cwd={}",
            init.session_id, init.model, init.claude_code_version, init.cwd
        )),
        Event::RateLimit(limit) => {
            let windows: Vec<String> = limit
                .windows
                .iter()
                .map(|(name, window)| format!("{name}={}", window.utilization))
                .collect();
            Some(format!(
                "rate_limit {} {} {}",
                limit.rate_limit_type,
                limit.status,
                windows.join(" ")
            ))
        }
        Event::AutocompactState(state) => Some(format!(
            "autocompact enabled={} window={} threshold={} source={}",
            state.enabled, state.effective_window, state.threshold, state.source
        )),
        Event::HookResponse(hook) => Some(format!(
            "hook {} outcome={} exit={}",
            hook.hook_name, hook.outcome, hook.exit_code
        )),
        Event::CompactBoundary(compaction) => Some(format!(
            "compacted trigger={} before={} after={}",
            compaction.trigger, compaction.pre_tokens, compaction.post_tokens
        )),
        Event::Result(result) => Some(format!(
            "turn ended is_error={} turns={}",
            result.is_error, result.num_turns
        )),
        _ => None,
    }
}

fn show(messages: &[Message]) {
    let mut out = io::stdout().lock();
    for message in messages {
        if let Some(line) = describe(&message.event) {
            let _ = writeln!(out, "{line}");
        }
    }
}

pub(crate) fn start(folder: &str, messages: &[&str]) -> Result<ExitCode, String> {
    let folder = PathBuf::from(folder)
        .canonicalize()
        .map_err(|error| format!("{folder}: {error}"))?;
    let config = AgentConfig {
        folder,
        claude: env::var_os(CLAUDE_ENV).map_or_else(|| DEFAULT_CLAUDE.into(), PathBuf::from),
        wake: env::current_exe().map_err(|error| error.to_string())?,
        socket: paths::socket()?,
    };
    let mut agent =
        Agent::start(&config).map_err(|error| format!("cannot start claude: {error}"))?;
    for text in messages {
        agent.send(text).map_err(|error| error.to_string())?;
        show(&agent.read_turn().map_err(|error| error.to_string())?);
    }
    agent.close_input();
    while let Some(message) = agent.read().map_err(|error| error.to_string())? {
        show(&[message]);
    }
    let succeeded = agent.finish().map_err(|error| error.to_string())?;
    Ok(if succeeded {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}
