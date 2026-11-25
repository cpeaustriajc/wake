//! The pass or fail rule for the harder task, decided from the repository's
//! state and never from what the agent says.
//!
//! A run is **errored**, and counts for neither arm, when it measured nothing:
//! a run error or an errored turn, no compaction, a reload hook that was
//! attached and never fired, a different set of steps finished before
//! `/compact` than steps 1 to 4, a progress file whose four choices cannot be
//! read, or a choice that appears in any file other than the progress file or
//! in any commit message before `/compact`.
//!
//! Otherwise a run **passes** when all eight steps are finished at the end with
//! the content the choices made before `/compact` require, and no step was
//! redone or undone. It **fails** in every other case.

use serde::Serialize;

use crate::hard_task::{Decisions, STEP_COUNT, STEPS_BEFORE_COMPACTION};
pub use crate::judge::{HookState, Verdict};

/// What the repository shows for one step of the harder task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HardStep {
    /// The step's number, from 1.
    pub number: u32,
    /// Whether the step's file is committed in `HEAD` with any text.
    pub file_present: bool,
    /// Whether the step's file in `HEAD` holds exactly what the choices
    /// require.
    pub file_correct: bool,
    /// How many times the step's line appears in the ledger in `HEAD`.
    pub ledger_lines: usize,
    /// How many commits in `HEAD`'s history carry the step's subject.
    pub commits: usize,
    /// How many times the step's subject was committed, amended or
    /// recommitted, as the reflog remembers.
    pub commit_events: usize,
    /// Whether a commit in `HEAD`'s history reverts the step's commit.
    pub reverted: bool,
}

impl HardStep {
    /// A step is finished when its file is correct, its ledger line is
    /// committed, a commit carries its subject and nothing reverted it.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.file_correct && self.ledger_lines >= 1 && self.commits >= 1 && !self.reverted
    }

    /// A step was redone when it left more than one ledger line or commit, or
    /// was committed more than once at any point.
    #[must_use]
    pub fn redone(&self) -> bool {
        self.ledger_lines > 1 || self.commits > 1 || self.commit_events > 1
    }

    /// A step holds a wrong choice when it wrote a file that is not correct.
    #[must_use]
    pub fn wrong_choice(&self) -> bool {
        self.file_present && !self.file_correct
    }
}

/// What the repository held just before `/compact` was sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Snapshot {
    /// The progress file's text in the working tree, if it could be read.
    pub progress: Option<String>,
    /// The choices read from it, or why they could not be read.
    pub decisions: Result<Decisions, Vec<String>>,
    /// Where each choice appeared outside the progress file, as a
    /// `key in <place>` line.
    pub leaks: Vec<String>,
}

/// Everything the verdict is decided from.
#[derive(Debug, Clone, Copy)]
pub struct HardObservation<'a> {
    /// The steps' evidence just before `/compact` was sent.
    pub before: &'a [HardStep],
    /// The steps' evidence once the run ended.
    pub after: &'a [HardStep],
    /// What the repository held just before `/compact`.
    pub snapshot: Option<&'a Snapshot>,
    /// Whether `claude` reported a `compact_boundary`.
    pub compaction_happened: bool,
    /// Whether the reload hook was attached and fired.
    pub hook: HookState,
    /// Anything that went wrong while running.
    pub errors: &'a [String],
}

/// The verdict and the numbers behind it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HardJudgement {
    /// Steps finished before `/compact`.
    pub finished_before_compaction: Vec<u32>,
    /// Steps finished when the run ended.
    pub finished_at_end: Vec<u32>,
    /// Steps committed more than once.
    pub redone: Vec<u32>,
    /// Steps finished before `/compact` and not finished at the end.
    pub undone: Vec<u32>,
    /// Steps 5 to 8 that wrote a file with a wrong choice.
    pub wrong_choice: Vec<u32>,
    /// Pass, fail or errored.
    pub verdict: Verdict,
    /// Why the verdict is what it is, one sentence each.
    pub reasons: Vec<String>,
}

fn numbers(steps: &[HardStep], keep: impl Fn(&HardStep) -> bool) -> Vec<u32> {
    steps
        .iter()
        .filter(|step| keep(step))
        .map(|step| step.number)
        .collect()
}

/// Decides a run by the rule in this module's documentation.
#[must_use]
pub fn judge_hard(observation: &HardObservation<'_>) -> HardJudgement {
    let finished_before_compaction = numbers(observation.before, HardStep::finished);
    let finished_at_end = numbers(observation.after, HardStep::finished);
    let redone = numbers(observation.after, HardStep::redone);
    let wrong_choice = numbers(observation.after, HardStep::wrong_choice);
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
    let expected_before: Vec<u32> = (1..=STEPS_BEFORE_COMPACTION).collect();
    if finished_before_compaction != expected_before {
        errored.push(format!(
            "steps {finished_before_compaction:?} were finished before /compact, not exactly steps {expected_before:?}"
        ));
    }
    match observation.snapshot {
        None => errored.push("the repository before /compact could not be read".to_owned()),
        Some(snapshot) => {
            if let Err(problems) = &snapshot.decisions {
                errored.extend(problems.iter().cloned());
            }
            errored.extend(
                snapshot
                    .leaks
                    .iter()
                    .map(|leak| format!("a choice was visible outside the progress file: {leak}")),
            );
        }
    }
    if !errored.is_empty() {
        return HardJudgement {
            finished_before_compaction,
            finished_at_end,
            redone,
            undone,
            wrong_choice,
            verdict: Verdict::Errored,
            reasons: errored,
        };
    }

    let mut reasons = Vec::new();
    if !wrong_choice.is_empty() {
        reasons.push(format!(
            "steps that wrote a choice other than the one made before /compact: {wrong_choice:?}"
        ));
    }
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
        reasons.push(
            "every step finished exactly once with the choices made before /compact".to_owned(),
        );
        Verdict::Pass
    } else {
        Verdict::Fail
    };
    HardJudgement {
        finished_before_compaction,
        finished_at_end,
        redone,
        undone,
        wrong_choice,
        verdict,
        reasons,
    }
}

/// Lines `key in <place>` for every choice whose value occurs in any of the
/// named texts.
#[must_use]
pub fn leaks(decisions: &Decisions, places: &[(String, String)]) -> Vec<String> {
    decisions
        .iter()
        .flat_map(|(key, value)| {
            places
                .iter()
                .filter(|(_, text)| text.contains(value.as_str()))
                .map(move |(place, _)| format!("{key} in {place}"))
        })
        .collect()
}
