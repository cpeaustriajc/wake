//! How the two arms of the harder test are compared.
//!
//! Only runs of the harder task that are not trials and not errored take part.
//! An errored run measured nothing, so it counts for neither arm, and an errored
//! run may be replaced by one more run of its arm, at most two per arm; every
//! run that started stays in the results folder. A run that passed or failed
//! is never replaced.
//!
//! The arms alternate in the fixed order hook, control, hook, control, and so
//! on, so a change in the service over time falls on both.
//!
//! The answer is one of three:
//! - **cannot tell** when an arm has fewer than 5 runs that passed or failed;
//! - **the hook changed the outcome** when the two-sided Fisher exact test on
//!   the passes and fails gives a p-value below 0.05, and the answer says which
//!   arm passed more;
//! - **the hook made no difference** when the pass rates of the two arms differ
//!   by 0.2 or less, and otherwise **cannot tell**, because a gap that large is
//!   too small to show with so few runs.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::hard_report::TASK_NAME;

/// The fewest runs that passed or failed an arm needs for an answer.
pub const MIN_VALID_RUNS: usize = 5;
/// The p-value below which the arms count as different.
pub const SIGNIFICANCE: f64 = 0.05;
/// The largest gap in pass rate that still counts as no difference.
pub const NO_DIFFERENCE_GAP: f64 = 0.2;
/// The most errored runs one arm may replace.
pub const MAX_REPLACEMENTS: usize = 2;

/// What one result file says about its run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRow {
    /// The run's ID.
    pub run_id: String,
    /// Whether the reload hook was attached.
    pub reload_hook: bool,
    /// Whether the run only shook out the harness.
    pub trial: bool,
    /// `pass`, `fail` or `errored`.
    pub verdict: String,
}

/// How many runs of one arm ended each way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tally {
    /// Runs that passed.
    pub passed: usize,
    /// Runs that failed.
    pub failed: usize,
    /// Runs that errored.
    pub errored: usize,
}

impl Tally {
    /// Runs that passed or failed.
    #[must_use]
    pub fn valid(self) -> usize {
        self.passed + self.failed
    }
}

/// The answer to whether the hook changed the outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Answer {
    /// The arms differ, and the hook arm passed more.
    HookBetter,
    /// The arms differ, and the control arm passed more.
    ControlBetter,
    /// The pass rates are close enough to call the same.
    NoDifference,
    /// There are too few runs, or the gap is too small for these runs to show.
    CannotTell,
}

/// The tallies, the p-value and the answer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Comparison {
    /// The arm with the reload hook.
    pub hook: Tally,
    /// The arm without it.
    pub control: Tally,
    /// The two-sided Fisher exact p-value, if both arms have runs.
    pub p_value: Option<f64>,
    /// The answer.
    pub answer: Answer,
}

fn tally<'a>(arm: impl Iterator<Item = &'a RunRow>) -> Tally {
    let rows: Vec<&RunRow> = arm.filter(|row| !row.trial).collect();
    let count = |verdict: &str| rows.iter().filter(|row| row.verdict == verdict).count();
    Tally {
        passed: count("pass"),
        failed: count("fail"),
        errored: count("errored"),
    }
}

fn choose(n: usize, k: usize) -> f64 {
    (0..k).fold(1.0, |acc, i| {
        acc * f64::from(u32::try_from(n - i).unwrap_or(u32::MAX))
            / f64::from(u32::try_from(i + 1).unwrap_or(u32::MAX))
    })
}

/// The two-sided Fisher exact p-value for `a_pass` of `a_total` against
/// `b_pass` of `b_total`.
#[must_use]
pub fn fisher_two_sided(a_pass: usize, a_total: usize, b_pass: usize, b_total: usize) -> f64 {
    let passes = a_pass + b_pass;
    let all = a_total + b_total;
    let probability =
        |x: usize| choose(a_total, x) * choose(b_total, passes - x) / choose(all, passes);
    let observed = probability(a_pass);
    let low = passes.saturating_sub(b_total);
    let high = passes.min(a_total);
    (low..=high)
        .map(probability)
        .filter(|chance| *chance <= observed * (1.0 + 1e-9))
        .sum::<f64>()
        .min(1.0)
}

fn rate(tally: Tally) -> f64 {
    f64::from(u32::try_from(tally.passed).unwrap_or(u32::MAX))
        / f64::from(u32::try_from(tally.valid().max(1)).unwrap_or(u32::MAX))
}

/// Compares the two arms by the rule in this module's documentation.
#[must_use]
pub fn compare(rows: &[RunRow]) -> Comparison {
    let hook = tally(rows.iter().filter(|row| row.reload_hook));
    let control = tally(rows.iter().filter(|row| !row.reload_hook));
    let p_value = (hook.valid() > 0 && control.valid() > 0)
        .then(|| fisher_two_sided(hook.passed, hook.valid(), control.passed, control.valid()));
    let answer = if hook.valid() < MIN_VALID_RUNS || control.valid() < MIN_VALID_RUNS {
        Answer::CannotTell
    } else if p_value.is_some_and(|p| p < SIGNIFICANCE) {
        if rate(hook) > rate(control) {
            Answer::HookBetter
        } else {
            Answer::ControlBetter
        }
    } else if (rate(hook) - rate(control)).abs() <= NO_DIFFERENCE_GAP + 1e-9 {
        Answer::NoDifference
    } else {
        Answer::CannotTell
    };
    Comparison {
        hook,
        control,
        p_value,
        answer,
    }
}

impl Answer {
    /// The answer in one plain sentence.
    #[must_use]
    pub fn sentence(self) -> &'static str {
        match self {
            Self::HookBetter => "The hook changed the outcome: the hook arm passed more.",
            Self::ControlBetter => "The hook changed the outcome: the arm without it passed more.",
            Self::NoDifference => "The hook made no difference to the outcome on this task.",
            Self::CannotTell => "This test cannot tell whether the hook changed the outcome.",
        }
    }
}

/// Reads one result file's row, or `None` for a file of another task.
#[must_use]
pub fn row_from(text: &str) -> Option<RunRow> {
    let value: Value = serde_json::from_str(text).ok()?;
    if value.get("task")?.as_str()? != TASK_NAME {
        return None;
    }
    Some(RunRow {
        run_id: value.get("run_id")?.as_str()?.to_owned(),
        reload_hook: value.get("reload_hook")?.as_bool()?,
        trial: value.get("trial")?.as_bool()?,
        verdict: value.get("verdict")?.as_str()?.to_owned(),
    })
}

/// Reads every harder-task result file in `folder`, ordered by run ID.
///
/// # Errors
///
/// Returns what went wrong if the folder cannot be read.
pub fn read_rows(folder: &Path) -> Result<Vec<RunRow>, String> {
    let mut rows: Vec<RunRow> = fs::read_dir(folder)
        .map_err(|error| format!("{}: {error}", folder.display()))?
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|entry| fs::read_to_string(entry.path()).ok())
        .filter_map(|text| row_from(&text))
        .collect();
    rows.sort_by(|a, b| a.run_id.cmp(&b.run_id));
    Ok(rows)
}

/// The comparison as text: every run, the tallies, the p-value and the answer.
#[must_use]
pub fn render(rows: &[RunRow]) -> String {
    let comparison = compare(rows);
    let mut text = String::from("run | hook | trial | verdict\n");
    for row in rows {
        let hook = if row.reload_hook { "on" } else { "off" };
        let trial = if row.trial { "yes" } else { "no" };
        let _ = writeln!(text, "{} | {hook} | {trial} | {}", row.run_id, row.verdict);
    }
    for (name, tally) in [("hook", comparison.hook), ("control", comparison.control)] {
        let _ = writeln!(
            text,
            "{name}: {} passed, {} failed, {} errored",
            tally.passed, tally.failed, tally.errored
        );
    }
    if let Some(p) = comparison.p_value {
        let _ = writeln!(text, "Fisher exact, two-sided: p = {p:.4}");
    }
    let _ = writeln!(text, "{}", comparison.answer.sentence());
    text
}
