//! The pass or fail rule against hand-made repository states.

use wake_resume_test::judge::{HookState, Observation, StepEvidence, Verdict, judge};

fn step(number: u32) -> StepEvidence {
    StepEvidence {
        number,
        file_committed: true,
        ledger_lines: 1,
        commits: 1,
        commit_events: 1,
        reverted: false,
    }
}

fn blank(number: u32) -> StepEvidence {
    StepEvidence {
        number,
        file_committed: false,
        ledger_lines: 0,
        commits: 0,
        commit_events: 0,
        reverted: false,
    }
}

fn steps(finished: u32) -> Vec<StepEvidence> {
    (1..=5)
        .map(|number| {
            if number <= finished {
                step(number)
            } else {
                blank(number)
            }
        })
        .collect()
}

fn judged(before: &[StepEvidence], after: &[StepEvidence]) -> wake_resume_test::judge::Judgement {
    judge(&Observation {
        before,
        after,
        compaction_happened: true,
        hook: HookState::Reloaded,
        errors: &[],
    })
}

#[test]
fn passes_when_every_step_finished_once() {
    let judgement = judged(&steps(3), &steps(5));
    assert_eq!(judgement.verdict, Verdict::Pass, "{:?}", judgement.reasons);
    assert_eq!(judgement.finished_before_compaction, [1, 2, 3], "before");
    assert_eq!(judgement.finished_at_end, [1, 2, 3, 4, 5], "end");
    assert!(judgement.redone.is_empty(), "nothing redone");
    assert!(judgement.undone.is_empty(), "nothing undone");
}

#[test]
fn a_second_ledger_line_means_the_step_was_redone() {
    let mut after = steps(5);
    after[1].ledger_lines = 2;
    let judgement = judged(&steps(3), &after);
    assert_eq!(judgement.verdict, Verdict::Fail, "verdict");
    assert_eq!(judgement.redone, [2], "redone");
}

#[test]
fn a_second_commit_event_means_the_step_was_redone_even_if_discarded() {
    let mut after = steps(5);
    after[0].commit_events = 2;
    let judgement = judged(&steps(3), &after);
    assert_eq!(judgement.verdict, Verdict::Fail, "verdict");
    assert_eq!(judgement.redone, [1], "redone");
}

#[test]
fn a_step_lost_after_compaction_is_undone() {
    let mut after = steps(5);
    after[2].file_committed = false;
    let judgement = judged(&steps(3), &after);
    assert_eq!(judgement.verdict, Verdict::Fail, "verdict");
    assert_eq!(judgement.undone, [3], "undone");
}

#[test]
fn a_reverted_step_is_undone() {
    let mut after = steps(5);
    after[0].reverted = true;
    let judgement = judged(&steps(3), &after);
    assert_eq!(judgement.verdict, Verdict::Fail, "verdict");
    assert_eq!(judgement.undone, [1], "undone");
}

#[test]
fn stopping_early_fails() {
    let judgement = judged(&steps(3), &steps(4));
    assert_eq!(judgement.verdict, Verdict::Fail, "verdict");
    assert!(
        judgement
            .reasons
            .iter()
            .any(|reason| reason.contains("[5]")),
        "{:?}",
        judgement.reasons
    );
}

#[test]
fn no_compaction_is_errored_not_a_pass() {
    let judgement = judge(&Observation {
        before: &steps(3),
        after: &steps(5),
        compaction_happened: false,
        hook: HookState::Reloaded,
        errors: &[],
    });
    assert_eq!(judgement.verdict, Verdict::Errored, "verdict");
}

#[test]
fn an_attached_hook_that_never_fired_is_errored() {
    let judgement = judge(&Observation {
        before: &steps(3),
        after: &steps(5),
        compaction_happened: true,
        hook: HookState::NotReloaded,
        errors: &[],
    });
    assert_eq!(judgement.verdict, Verdict::Errored, "verdict");
}

#[test]
fn a_control_run_needs_no_hook() {
    let judgement = judge(&Observation {
        before: &steps(3),
        after: &steps(5),
        compaction_happened: true,
        hook: HookState::NotAttached,
        errors: &[],
    });
    assert_eq!(judgement.verdict, Verdict::Pass, "verdict");
}

#[test]
fn finishing_none_or_all_steps_before_compact_leaves_nothing_to_resume() {
    for finished in [0, 5] {
        let judgement = judged(&steps(finished), &steps(5));
        assert_eq!(
            judgement.verdict,
            Verdict::Errored,
            "{finished} steps before"
        );
    }
}

#[test]
fn a_run_error_is_errored_with_its_reason() {
    let errors = ["claude exited with a failure status".to_owned()];
    let judgement = judge(&Observation {
        before: &steps(3),
        after: &steps(5),
        compaction_happened: true,
        hook: HookState::Reloaded,
        errors: &errors,
    });
    assert_eq!(judgement.verdict, Verdict::Errored, "verdict");
    assert!(
        judgement.reasons.contains(&errors[0]),
        "{:?}",
        judgement.reasons
    );
}
