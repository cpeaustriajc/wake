//! How the two arms are compared, against the answers the rule must give.

use wake_resume_test::compare::{Answer, RunRow, compare, fisher_two_sided, render, row_from};

fn rows(hook: &[&str], control: &[&str]) -> Vec<RunRow> {
    let arm = |verdicts: &[&str], reload_hook: bool| {
        verdicts
            .iter()
            .enumerate()
            .map(|(index, verdict)| RunRow {
                run_id: format!("{reload_hook}-{index}"),
                reload_hook,
                trial: false,
                verdict: (*verdict).to_owned(),
            })
            .collect::<Vec<_>>()
    };
    let mut all = arm(hook, true);
    all.extend(arm(control, false));
    all
}

#[test]
fn fisher_matches_known_values() {
    let close = |left: f64, right: f64| (left - right).abs() < 1e-4;
    assert!(
        close(fisher_two_sided(5, 5, 0, 5), 0.0079),
        "5/5 against 0/5"
    );
    assert!(
        close(fisher_two_sided(5, 5, 1, 5), 0.0476),
        "5/5 against 1/5"
    );
    assert!(
        close(fisher_two_sided(4, 5, 0, 5), 0.0476),
        "4/5 against 0/5"
    );
    assert!(close(fisher_two_sided(5, 5, 5, 5), 1.0), "equal arms");
    assert!(
        close(fisher_two_sided(5, 5, 3, 5), 0.4444),
        "5/5 against 3/5"
    );
}

#[test]
fn equal_arms_with_five_runs_each_made_no_difference() {
    let all = ["pass"; 5];
    assert_eq!(compare(&rows(&all, &all)).answer, Answer::NoDifference);
}

#[test]
fn one_run_apart_is_still_no_difference() {
    let result = compare(&rows(
        &["pass"; 5],
        &["pass", "pass", "pass", "pass", "fail"],
    ));
    assert_eq!(result.answer, Answer::NoDifference);
}

#[test]
fn a_wide_gap_names_the_better_arm() {
    let fails = ["fail"; 5];
    let passes = ["pass"; 5];
    assert_eq!(compare(&rows(&passes, &fails)).answer, Answer::HookBetter);
    assert_eq!(
        compare(&rows(&fails, &passes)).answer,
        Answer::ControlBetter
    );
}

#[test]
fn a_gap_that_five_runs_cannot_show_cannot_be_told() {
    let result = compare(&rows(
        &["pass"; 5],
        &["pass", "pass", "pass", "fail", "fail"],
    ));
    assert_eq!(result.answer, Answer::CannotTell);
}

#[test]
fn fewer_than_five_valid_runs_in_an_arm_cannot_be_told() {
    let errored = ["pass", "pass", "pass", "pass", "errored"];
    let result = compare(&rows(&errored, &["pass"; 5]));
    assert_eq!(result.answer, Answer::CannotTell);
    assert_eq!(result.hook.errored, 1);
}

#[test]
fn trials_and_other_tasks_are_left_out() {
    let mut all = rows(&["pass"; 5], &["pass"; 5]);
    all.push(RunRow {
        run_id: "trial".to_owned(),
        reload_hook: false,
        trial: true,
        verdict: "fail".to_owned(),
    });
    assert_eq!(compare(&all).control.failed, 0);
    let first = r#"{"run_id":"a","reload_hook":true,"verdict":"pass"}"#;
    assert_eq!(
        row_from(first),
        None,
        "a first-test result has no task field"
    );
    let hard =
        r#"{"run_id":"b","task":"harder","trial":false,"reload_hook":false,"verdict":"fail"}"#;
    assert_eq!(
        row_from(hard).map(|row| row.verdict),
        Some("fail".to_owned())
    );
}

#[test]
fn the_rendered_text_lists_every_run_and_ends_with_the_answer() {
    let text = render(&rows(&["pass"; 5], &["pass"; 5]));
    assert_eq!(
        text.lines().filter(|line| line.contains("| pass")).count(),
        10
    );
    assert!(
        text.ends_with("The hook made no difference to the outcome on this task.\n"),
        "{text}"
    );
}
