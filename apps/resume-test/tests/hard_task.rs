//! The harder task's choices, their formats and what later steps must hold.

use wake_resume_test::hard_task::{
    Decisions, content_matches, decision_used_by, expected_content, messages, parse_decisions,
};

fn progress(lines: &[&str]) -> String {
    format!(
        "---\nphase: build\nheadline: working\nupdated: 2026-11-25T10:00:00Z\n---\n\n{}\n",
        lines.join("\n")
    )
}

fn good() -> Vec<&'static str> {
    vec![
        "decision codename: marzipan",
        "decision port: 24817",
        "decision colour: 3fa9c2",
        "decision motto: lantern quiet orchard",
    ]
}

#[test]
fn four_valid_lines_are_read() {
    let decisions = parse_decisions(&progress(&good())).expect("valid choices");
    assert_eq!(decisions["codename"], "marzipan");
    assert_eq!(decisions["motto"], "lantern quiet orchard");
    assert_eq!(decisions.len(), 4, "one value per key");
}

#[test]
fn list_markers_and_backticks_are_ignored_and_the_last_line_wins() {
    let text = progress(&[
        "- decision codename: `firstone`",
        "decision codename: marzipan",
        "* decision port: 24817",
        "decision colour: 3fa9c2",
        "decision motto: lantern quiet orchard",
    ]);
    let decisions = parse_decisions(&text).expect("valid choices");
    assert_eq!(decisions["codename"], "marzipan", "the last line wins");
}

#[test]
fn a_missing_or_malformed_choice_is_named() {
    let text = progress(&[
        "decision codename: Marzipan",
        "decision port: 99999",
        "decision colour: 3fa9c2",
    ]);
    let problems = parse_decisions(&text).expect_err("three problems");
    assert_eq!(problems.len(), 3, "{problems:?}");
    assert!(
        problems.iter().any(|p| p.contains("codename")),
        "{problems:?}"
    );
    assert!(problems.iter().any(|p| p.contains("port")), "{problems:?}");
    assert!(problems.iter().any(|p| p.contains("motto")), "{problems:?}");
}

#[test]
fn steps_one_to_four_hold_only_their_label() {
    let decisions = Decisions::new();
    assert_eq!(expected_content(2, &decisions).as_deref(), Some("step 2"));
    assert!(
        content_matches(2, "step 2\n", &decisions),
        "trailing newline"
    );
    assert!(
        !content_matches(2, "step 2\ncodename=x", &decisions),
        "extra line"
    );
}

#[test]
fn steps_five_to_eight_hold_their_label_and_one_choice_in_order() {
    let decisions = parse_decisions(&progress(&good())).expect("valid choices");
    assert_eq!(decision_used_by(4), None);
    assert_eq!(decision_used_by(5), Some("codename"));
    assert_eq!(decision_used_by(8), Some("motto"));
    assert!(content_matches(6, "step 6\nport=24817\n", &decisions));
    assert!(
        !content_matches(6, "step 6\nport=24818\n", &decisions),
        "wrong value"
    );
    assert!(
        !content_matches(6, "step 6\n", &decisions),
        "no choice line"
    );
    assert_eq!(
        expected_content(5, &Decisions::new()),
        None,
        "no choices known"
    );
}

#[test]
fn the_three_messages_are_fixed_and_the_task_names_the_progress_file_only() {
    let [first, second, third] = messages();
    assert_eq!(second, "/compact");
    assert_eq!(third, "continue");
    assert!(first.contains(".wake/progress.md"), "{first}");
    assert!(first.contains("8 numbered steps"), "{first}");
    assert!(
        first.contains("in no other file, in no commit message"),
        "{first}"
    );
}
