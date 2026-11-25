//! The harder task's pass or fail rule against hand-made repository states.

use wake_resume_test::hard_judge::{
    HardObservation, HardStep, HookState, Snapshot, Verdict, judge_hard, leaks,
};
use wake_resume_test::hard_task::Decisions;

fn step(number: u32) -> HardStep {
    HardStep {
        number,
        file_present: true,
        file_correct: true,
        ledger_lines: 1,
        commits: 1,
        commit_events: 1,
        reverted: false,
    }
}

fn blank(number: u32) -> HardStep {
    HardStep {
        number,
        file_present: false,
        file_correct: false,
        ledger_lines: 0,
        commits: 0,
        commit_events: 0,
        reverted: false,
    }
}

fn steps(finished: u32) -> Vec<HardStep> {
    (1..=8)
        .map(|number| {
            if number <= finished {
                step(number)
            } else {
                blank(number)
            }
        })
        .collect()
}

fn clean() -> Snapshot {
    Snapshot {
        progress: Some("decision codename: marzipan".to_owned()),
        decisions: Ok(Decisions::new()),
        leaks: Vec::new(),
    }
}

fn judge(
    before: &[HardStep],
    after: &[HardStep],
    snapshot: Option<&Snapshot>,
    hook: HookState,
) -> wake_resume_test::hard_judge::HardJudgement {
    judge_hard(&HardObservation {
        before,
        after,
        snapshot,
        compaction_happened: true,
        hook,
        errors: &[],
    })
}

#[test]
fn all_eight_finished_once_with_the_right_choices_passes() {
    let result = judge(&steps(4), &steps(8), Some(&clean()), HookState::Reloaded);
    assert_eq!(result.verdict, Verdict::Pass, "{:?}", result.reasons);
}

#[test]
fn a_wrong_choice_after_compaction_fails_and_is_named() {
    let mut after = steps(8);
    after[5].file_correct = false;
    let result = judge(&steps(4), &after, Some(&clean()), HookState::NotAttached);
    assert_eq!(result.verdict, Verdict::Fail);
    assert_eq!(result.wrong_choice, vec![6]);
    assert!(
        result.reasons.iter().any(|r| r.contains("[6]")),
        "{:?}",
        result.reasons
    );
}

#[test]
fn an_unfinished_step_fails() {
    let result = judge(&steps(4), &steps(6), Some(&clean()), HookState::NotAttached);
    assert_eq!(result.verdict, Verdict::Fail);
    assert!(
        result.reasons.iter().any(|r| r.contains("[7, 8]")),
        "{:?}",
        result.reasons
    );
}

#[test]
fn a_redone_step_fails() {
    let mut after = steps(8);
    after[1].commit_events = 2;
    let result = judge(&steps(4), &after, Some(&clean()), HookState::NotAttached);
    assert_eq!(result.verdict, Verdict::Fail);
    assert_eq!(result.redone, vec![2]);
}

#[test]
fn an_undone_step_fails() {
    let mut after = steps(8);
    after[2].reverted = true;
    let result = judge(&steps(4), &after, Some(&clean()), HookState::NotAttached);
    assert_eq!(result.verdict, Verdict::Fail);
    assert_eq!(result.undone, vec![3]);
}

#[test]
fn finishing_more_or_fewer_than_steps_one_to_four_before_compaction_is_errored() {
    for before in [steps(3), steps(5)] {
        let result = judge(&before, &steps(8), Some(&clean()), HookState::NotAttached);
        assert_eq!(result.verdict, Verdict::Errored, "{:?}", result.reasons);
    }
}

#[test]
fn unreadable_choices_or_a_missing_snapshot_is_errored() {
    let unreadable = Snapshot {
        progress: None,
        decisions: Err(vec![
            "no `decision port:` line in the progress file".to_owned(),
        ]),
        leaks: Vec::new(),
    };
    let result = judge(
        &steps(4),
        &steps(8),
        Some(&unreadable),
        HookState::NotAttached,
    );
    assert_eq!(result.verdict, Verdict::Errored);
    let result = judge(&steps(4), &steps(8), None, HookState::NotAttached);
    assert_eq!(result.verdict, Verdict::Errored);
}

#[test]
fn a_choice_visible_outside_the_progress_file_is_errored() {
    let leaked = Snapshot {
        leaks: vec!["port in steps/2.txt".to_owned()],
        ..clean()
    };
    let result = judge(&steps(4), &steps(8), Some(&leaked), HookState::NotAttached);
    assert_eq!(result.verdict, Verdict::Errored);
    assert!(
        result.reasons.iter().any(|r| r.contains("steps/2.txt")),
        "{:?}",
        result.reasons
    );
}

#[test]
fn an_attached_hook_that_never_fired_is_errored() {
    let result = judge(&steps(4), &steps(8), Some(&clean()), HookState::NotReloaded);
    assert_eq!(result.verdict, Verdict::Errored);
}

#[test]
fn leaks_name_each_choice_and_where_it_appeared() {
    let mut decisions = Decisions::new();
    decisions.insert("port".to_owned(), "24817".to_owned());
    decisions.insert("codename".to_owned(), "marzipan".to_owned());
    let places = vec![
        ("steps/2.txt".to_owned(), "step 2 24817".to_owned()),
        ("a commit message".to_owned(), "step 1".to_owned()),
    ];
    assert_eq!(
        leaks(&decisions, &places),
        vec!["port in steps/2.txt".to_owned()]
    );
}
