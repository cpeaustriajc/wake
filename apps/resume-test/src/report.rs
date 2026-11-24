//! What a run saw in the stream and the daemon, and the JSON file it becomes.

use serde::Serialize;
use wake_claude::stream::Event;
use wake_daemon::StoredCompaction;

use crate::judge::{HookState, Judgement, Observation, StepEvidence, Verdict, judge};
use crate::task::STEP_COUNT;

/// A `hook_response` the stream reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HookSeen {
    /// The hook as `claude` names it, for example `SessionStart:compact`.
    pub name: String,
    /// `success` or the failure `claude` reports.
    pub outcome: String,
    /// The hook's exit code.
    pub exit_code: i64,
    /// Whether the hook printed a progress file for the agent.
    pub printed_progress_file: bool,
}

/// A `compact_boundary` the stream reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompactionSeen {
    /// `manual` or `auto`.
    pub trigger: String,
    /// Tokens in the context before compaction.
    pub pre_tokens: u64,
    /// Tokens in the context after compaction.
    pub post_tokens: u64,
}

/// How a turn ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TurnSeen {
    /// Whether `claude` ended the turn in an error.
    pub is_error: bool,
    /// The number of model turns the request took.
    pub num_turns: u64,
}

/// A compaction as the daemon stored it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DaemonRow {
    /// What the hook said started the session.
    pub source: String,
    /// Whether the progress file was `found`, `missing` or `invalid`.
    pub progress: String,
}

impl From<&StoredCompaction> for DaemonRow {
    fn from(stored: &StoredCompaction) -> Self {
        Self {
            source: stored.report.source.clone(),
            progress: stored.report.progress.as_str().to_owned(),
        }
    }
}

/// Everything read from the stream while a run went on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Observed {
    /// The model the session reported.
    pub model: Option<String>,
    /// The `claude` version the session reported.
    pub claude_version: Option<String>,
    /// The permission mode the session reported.
    pub permission_mode: Option<String>,
    /// Hooks that finished, in order.
    pub hooks: Vec<HookSeen>,
    /// Compactions that finished, in order.
    pub compactions: Vec<CompactionSeen>,
    /// Turns that ended, in order.
    pub turns: Vec<TurnSeen>,
}

impl Observed {
    /// Takes in one message from the stream.
    pub fn ingest(&mut self, event: &Event) {
        match event {
            Event::Init(init) => {
                self.model.get_or_insert_with(|| init.model.clone());
                self.claude_version
                    .get_or_insert_with(|| init.claude_code_version.clone());
                self.permission_mode
                    .get_or_insert_with(|| init.permission_mode.clone());
            }
            Event::HookResponse(hook) => self.hooks.push(HookSeen {
                name: hook.hook_name.clone(),
                outcome: hook.outcome.clone(),
                exit_code: hook.exit_code,
                printed_progress_file: hook.stdout.contains("phase:"),
            }),
            Event::CompactBoundary(compaction) => self.compactions.push(CompactionSeen {
                trigger: compaction.trigger.clone(),
                pre_tokens: compaction.pre_tokens,
                post_tokens: compaction.post_tokens,
            }),
            Event::Result(result) => self.turns.push(TurnSeen {
                is_error: result.is_error,
                num_turns: result.num_turns,
            }),
            _ => {}
        }
    }
}

/// Compaction as one run saw it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompactionReport {
    /// Whether `claude` reported a `compact_boundary`.
    pub happened: bool,
    /// How many it reported.
    pub count: usize,
    /// What started the first, `manual` for `/compact`.
    pub trigger: Option<String>,
    /// Tokens in the context before the first compaction.
    pub pre_tokens: Option<u64>,
    /// Tokens in the context after the first compaction.
    pub post_tokens: Option<u64>,
}

/// The reload hook as one run saw it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HookReport {
    /// Whether the hook was attached. False for a control run.
    pub attached: bool,
    /// Whether the daemon recorded a reload.
    pub ran: bool,
    /// Whether the hook printed a progress file into the stream.
    pub printed_progress_file: bool,
    /// The hooks the stream reported.
    pub in_stream: Vec<HookSeen>,
    /// The compactions the daemon stored.
    pub daemon_rows: Vec<DaemonRow>,
}

/// The steps as one run left them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StepsReport {
    /// How many steps the task has.
    pub total: u32,
    /// Steps finished before `/compact`.
    pub finished_before_compaction: Vec<u32>,
    /// Steps finished when the run ended.
    pub finished_at_end: Vec<u32>,
    /// Steps done more than once.
    pub redone: Vec<u32>,
    /// Steps finished before `/compact` and not finished at the end.
    pub undone: Vec<u32>,
}

/// One run's result file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunResult {
    /// The run's ID, also the file's name.
    pub run_id: String,
    /// False for a control run without the reload hook.
    pub reload_hook: bool,
    /// When the run started, RFC 3339 in UTC.
    pub started_at: String,
    /// When the run ended, RFC 3339 in UTC.
    pub finished_at: String,
    /// The `claude` version the session reported.
    pub claude_version: Option<String>,
    /// The model the stream reported.
    pub model: Option<String>,
    /// The permission mode the session ran under.
    pub permission_mode: Option<String>,
    /// Compaction.
    pub compaction: CompactionReport,
    /// The reload hook.
    pub hook: HookReport,
    /// The steps.
    pub steps: StepsReport,
    /// Each step's evidence just before `/compact`, if it could be read.
    pub evidence_before_compaction: Option<Vec<StepEvidence>>,
    /// Each step's evidence at the end, if it could be read.
    pub evidence_at_end: Option<Vec<StepEvidence>>,
    /// How each turn ended.
    pub turns: Vec<TurnSeen>,
    /// Pass, fail or errored.
    pub verdict: Verdict,
    /// Why.
    pub reasons: Vec<String>,
}

/// What one run produced, ready to be judged and written.
#[derive(Debug, Clone)]
pub struct Run {
    /// The run's ID.
    pub run_id: String,
    /// Whether the reload hook was attached.
    pub reload_hook: bool,
    /// When the run started.
    pub started_at: String,
    /// When the run ended.
    pub finished_at: String,
    /// What the stream showed.
    pub observed: Observed,
    /// What the daemon stored.
    pub daemon_rows: Vec<DaemonRow>,
    /// The steps' evidence just before `/compact`.
    pub before: Option<Vec<StepEvidence>>,
    /// The steps' evidence at the end.
    pub after: Option<Vec<StepEvidence>>,
    /// Anything that went wrong while running.
    pub errors: Vec<String>,
}

/// Judges a run and shapes its result file.
#[must_use]
pub fn build(run: Run) -> RunResult {
    let Run {
        run_id,
        reload_hook,
        started_at,
        finished_at,
        observed,
        daemon_rows,
        before,
        after,
        mut errors,
    } = run;
    let reloaded = daemon_rows.iter().any(|row| row.source == "compact");
    let hook_state = match (reload_hook, reloaded) {
        (false, _) => HookState::NotAttached,
        (true, true) => HookState::Reloaded,
        (true, false) => HookState::NotReloaded,
    };
    if before.is_none() || after.is_none() {
        errors.push("the repository's state could not be read".to_owned());
    }
    errors.extend(
        observed
            .turns
            .iter()
            .enumerate()
            .filter(|(_, turn)| turn.is_error)
            .map(|(index, _)| format!("turn {} ended in an error", index + 1)),
    );
    let judgement: Judgement = judge(&Observation {
        before: before.as_deref().unwrap_or_default(),
        after: after.as_deref().unwrap_or_default(),
        compaction_happened: !observed.compactions.is_empty(),
        hook: hook_state,
        errors: &errors,
    });
    let first = observed.compactions.first();
    RunResult {
        run_id,
        reload_hook,
        started_at,
        finished_at,
        claude_version: observed.claude_version,
        model: observed.model,
        permission_mode: observed.permission_mode,
        compaction: CompactionReport {
            happened: first.is_some(),
            count: observed.compactions.len(),
            trigger: first.map(|seen| seen.trigger.clone()),
            pre_tokens: first.map(|seen| seen.pre_tokens),
            post_tokens: first.map(|seen| seen.post_tokens),
        },
        hook: HookReport {
            attached: reload_hook,
            ran: reloaded,
            printed_progress_file: observed.hooks.iter().any(|seen| seen.printed_progress_file),
            in_stream: observed.hooks,
            daemon_rows,
        },
        steps: StepsReport {
            total: STEP_COUNT,
            finished_before_compaction: judgement.finished_before_compaction,
            finished_at_end: judgement.finished_at_end,
            redone: judgement.redone,
            undone: judgement.undone,
        },
        evidence_before_compaction: before,
        evidence_at_end: after,
        turns: observed.turns,
        verdict: judgement.verdict,
        reasons: judgement.reasons,
    }
}

impl RunResult {
    /// The result file's text.
    ///
    /// # Errors
    ///
    /// Returns the serialization error, which does not happen for these types.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
