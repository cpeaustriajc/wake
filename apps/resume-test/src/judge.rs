//! The pass or fail rule, decided from the repository's state and never from
//! what the agent says.

use serde::Serialize;

use crate::task::STEP_COUNT;

/// What the repository shows for one step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StepEvidence {
    /// The step's number, from 1.
    pub number: u32,
    /// Whether the step's file is committed in `HEAD` with the right text.
    pub file_committed: bool,
    /// How many times the step's line appears in the ledger committed in
    /// `HEAD`.
    pub ledger_lines: usize,
    /// How many commits in `HEAD`'s history carry the step's subject.
    pub commits: usize,
    /// How many times the step's subject was committed, amended or
    /// recommitted, as the reflog remembers, including work since discarded.
    pub commit_events: usize,
    /// Whether a commit in `HEAD`'s history reverts the step's commit.
    pub reverted: bool,
}

impl StepEvidence {
    /// A step is finished when its file and its ledger line are committed,
    /// and a commit carries its subject, and nothing reverted it.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.file_committed && self.ledger_lines >= 1 && self.commits >= 1 && !self.reverted
    }

    /// A step was redone when it left more than one ledger line, more than one
    /// commit, or was committed more than once at any point.
    #[must_use]
    pub fn redone(&self) -> bool {
        self.ledger_lines > 1 || self.commits > 1 || self.commit_events > 1
    }
}

/// Whether the run had the reload hook, and whether it fired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum HookState {
    /// The run is a control: the hook was left out.
    NotAttached,
    /// The hook was attached and the daemon recorded a reload.
    Reloaded,
    /// The hook was attached and the daemon recorded nothing.
    NotReloaded,
}

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Verdict {
    /// Compaction happened and every step finished exactly once.
    Pass,
    /// Compaction happened and a step was redone, undone or left unfinished.
    Fail,
    /// The run could not measure resumption, so it counts as neither.
    Errored,
}

/// Everything the verdict is decided from.
#[derive(Debug, Clone, Copy)]
pub struct Observation<'a> {
    /// The steps' evidence just before `/compact` was sent.
    pub before: &'a [StepEvidence],
    /// The steps' evidence once the run ended.
    pub after: &'a [StepEvidence],
    /// Whether `claude` reported a `compact_boundary`.
    pub compaction_happened: bool,
    /// Whether the reload hook was attached and fired.
    pub hook: HookState,
    /// Anything that went wrong while running, such as a failed turn.
    pub errors: &'a [String],
}

/// The verdict and the numbers behind it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Judgement {
    /// Steps finished before `/compact`.
    pub finished_before_compaction: Vec<u32>,
    /// Steps finished when the run ended.
    pub finished_at_end: Vec<u32>,
    /// Steps committed or written more than once.
    pub redone: Vec<u32>,
    /// Steps finished before `/compact` that are not finished at the end.
    pub undone: Vec<u32>,
    /// Pass, fail or errored.
    pub verdict: Verdict,
    /// Why the verdict is what it is, one sentence each.
    pub reasons: Vec<String>,
}

fn numbers(steps: &[StepEvidence], keep: impl Fn(&StepEvidence) -> bool) -> Vec<u32> {
    steps
        .iter()
        .filter(|step| keep(step))
        .map(|step| step.number)
        .collect()
}

/// Decides a run.
///
/// Errored when something went wrong while running, no compaction happened,
/// the reload hook was attached but never fired, or the agent finished none or
/// all of the steps before `/compact` (so nothing was left to resume).
/// Otherwise a pass needs every step finished at the end and none redone or
/// undone.
#[must_use]
pub fn judge(observation: &Observation<'_>) -> Judgement {
    let finished_before_compaction = numbers(observation.before, StepEvidence::finished);
    let finished_at_end = numbers(observation.after, StepEvidence::finished);
    let redone = numbers(observation.after, StepEvidence::redone);
    let undone: Vec<u32> = finished_before_compaction
        .iter()
        .copied()
        .filter(|number| !finished_at_end.contains(number))
        .collect();

    let mut errored: Vec<String> = observation.errors.to_vec();
    if !observation.compaction_happened {
        errored.push("claude reported no compaction".to_owned());
    }
    if observation.hook == HookState::NotReloaded {
        errored.push("the reload hook was attached but the daemon recorded no reload".to_owned());
    }
    let before = finished_before_compaction.len();
    if before == 0 || before >= usize::try_from(STEP_COUNT).unwrap_or(usize::MAX) {
        errored.push(format!(
            "{before} of {STEP_COUNT} steps were finished before /compact, so nothing was left to resume"
        ));
    }
    if !errored.is_empty() {
        return Judgement {
            finished_before_compaction,
            finished_at_end,
            redone,
            undone,
            verdict: Verdict::Errored,
            reasons: errored,
        };
    }

    let mut reasons = Vec::new();
    if !redone.is_empty() {
        reasons.push(format!("steps redone: {redone:?}"));
    }
    if !undone.is_empty() {
        reasons.push(format!("steps undone: {undone:?}"));
    }
    let unfinished: Vec<u32> = (1..=STEP_COUNT)
        .filter(|number| !finished_at_end.contains(number))
        .collect();
    if !unfinished.is_empty() {
        reasons.push(format!("steps not finished at the end: {unfinished:?}"));
    }
    let verdict = if reasons.is_empty() {
        reasons.push("every step finished exactly once and none was undone".to_owned());
        Verdict::Pass
    } else {
        Verdict::Fail
    };
    Judgement {
        finished_before_compaction,
        finished_at_end,
        redone,
        undone,
        verdict,
        reasons,
    }
}
