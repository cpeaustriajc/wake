//! The harder task's result file, one per run.

use serde::Serialize;
use wake_claude::stream::{AutocompactState, Event};

use crate::hard_judge::{HardObservation, HardStep, HookState, Snapshot, Verdict, judge_hard};
use crate::hard_task::STEP_COUNT;
use crate::report::{CompactionReport, DaemonRow, HookReport, Observed, StepsReport, TurnSeen};

/// The name the result file carries in its `task` field.
pub const TASK_NAME: &str = "harder";

/// The context window and compaction threshold the stream reported, which show
/// how far a run stayed from automatic compaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AutocompactSeen {
    /// Whether automatic compaction is on.
    pub enabled: bool,
    /// The context window in tokens.
    pub effective_window: u64,
    /// The token count at which automatic compaction starts.
    pub threshold: u64,
    /// Where the setting comes from.
    pub source: String,
}

impl From<&AutocompactState> for AutocompactSeen {
    fn from(state: &AutocompactState) -> Self {
        Self {
            enabled: state.enabled,
            effective_window: state.effective_window,
            threshold: state.threshold,
            source: state.source.clone(),
        }
    }
}

/// What the stream showed during a harder-task run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HardObserved {
    /// The general stream facts.
    pub stream: Observed,
    /// The last `autocompact_state` the stream reported.
    pub autocompact: Option<AutocompactSeen>,
}

impl HardObserved {
    /// Takes in one message from the stream.
    pub fn ingest(&mut self, event: &Event) {
        if let Event::AutocompactState(state) = event {
            self.autocompact = Some(AutocompactSeen::from(state));
        }
        self.stream.ingest(event);
    }
}

/// What one harder-task run produced, ready to be judged and written.
#[derive(Debug, Clone)]
pub struct HardRun {
    /// The run's ID, also its file's name.
    pub run_id: String,
    /// Whether this run only shook out the harness.
    pub trial: bool,
    /// Whether the reload hook was attached.
    pub reload_hook: bool,
    /// When the run started.
    pub started_at: String,
    /// When the run ended.
    pub finished_at: String,
    /// What the stream showed.
    pub observed: HardObserved,
    /// What the daemon stored.
    pub daemon_rows: Vec<DaemonRow>,
    /// The repository just before `/compact`.
    pub snapshot: Option<Snapshot>,
    /// The steps' evidence just before `/compact`.
    pub before: Option<Vec<HardStep>>,
    /// The steps' evidence at the end.
    pub after: Option<Vec<HardStep>>,
    /// Anything that went wrong while running.
    pub errors: Vec<String>,
}

/// One harder-task run's result file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HardResult {
    /// The run's ID, also the file's name.
    pub run_id: String,
    /// Always `harder`.
    pub task: String,
    /// True for a run that only shook out the harness; the comparison leaves
    /// it out.
    pub trial: bool,
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
    /// The window and threshold for automatic compaction.
    pub autocompact: Option<AutocompactSeen>,
    /// The reload hook.
    pub hook: HookReport,
    /// The progress file, the choices and any leaks just before `/compact`.
    pub snapshot_before_compaction: Option<Snapshot>,
    /// The steps.
    pub steps: StepsReport,
    /// Steps 5 to 8 that wrote a wrong choice.
    pub wrong_choice: Vec<u32>,
    /// Each step's evidence just before `/compact`.
    pub evidence_before_compaction: Option<Vec<HardStep>>,
    /// Each step's evidence at the end.
    pub evidence_at_end: Option<Vec<HardStep>>,
    /// How each turn ended.
    pub turns: Vec<TurnSeen>,
    /// Pass, fail or errored.
    pub verdict: Verdict,
    /// Why.
    pub reasons: Vec<String>,
}

/// Judges a run and shapes its result file.
#[must_use]
pub fn build_hard(run: HardRun) -> HardResult {
    let HardRun {
        run_id,
        trial,
        reload_hook,
        started_at,
        finished_at,
        observed,
        daemon_rows,
        snapshot,
        before,
        after,
        mut errors,
    } = run;
    let HardObserved {
        stream,
        autocompact,
    } = observed;
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
        stream
            .turns
            .iter()
            .enumerate()
            .filter(|(_, turn)| turn.is_error)
            .map(|(index, _)| format!("turn {} ended in an error", index + 1)),
    );
    let judgement = judge_hard(&HardObservation {
        before: before.as_deref().unwrap_or_default(),
        after: after.as_deref().unwrap_or_default(),
        snapshot: snapshot.as_ref(),
        compaction_happened: !stream.compactions.is_empty(),
        hook: hook_state,
        errors: &errors,
    });
    let first = stream.compactions.first();
    HardResult {
        run_id,
        task: TASK_NAME.to_owned(),
        trial,
        reload_hook,
        started_at,
        finished_at,
        claude_version: stream.claude_version,
        model: stream.model,
        permission_mode: stream.permission_mode,
        compaction: CompactionReport {
            happened: first.is_some(),
            count: stream.compactions.len(),
            trigger: first.map(|seen| seen.trigger.clone()),
            pre_tokens: first.map(|seen| seen.pre_tokens),
            post_tokens: first.map(|seen| seen.post_tokens),
        },
        autocompact,
        hook: HookReport {
            attached: reload_hook,
            ran: reloaded,
            printed_progress_file: stream.hooks.iter().any(|seen| seen.printed_progress_file),
            in_stream: stream.hooks,
            daemon_rows,
        },
        snapshot_before_compaction: snapshot,
        steps: StepsReport {
            total: STEP_COUNT,
            finished_before_compaction: judgement.finished_before_compaction,
            finished_at_end: judgement.finished_at_end,
            redone: judgement.redone,
            undone: judgement.undone,
        },
        wrong_choice: judgement.wrong_choice,
        evidence_before_compaction: before,
        evidence_at_end: after,
        turns: stream.turns,
        verdict: judgement.verdict,
        reasons: judgement.reasons,
    }
}

impl HardResult {
    /// The result file's text.
    ///
    /// # Errors
    ///
    /// Returns the serialization error, which does not happen for these types.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
