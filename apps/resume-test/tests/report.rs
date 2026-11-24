//! The result file's shape and how stream messages fill it.

use serde_json::Value;
use wake_claude::stream::Event;
use wake_resume_test::judge::StepEvidence;
use wake_resume_test::report::{DaemonRow, Observed, Run, build};

fn step(number: u32, finished: u32) -> StepEvidence {
    let done = number <= finished;
    StepEvidence {
        number,
        file_committed: done,
        ledger_lines: usize::from(done),
        commits: usize::from(done),
        commit_events: usize::from(done),
        reverted: false,
    }
}

fn steps(finished: u32) -> Vec<StepEvidence> {
    (1..=5).map(|number| step(number, finished)).collect()
}

fn observed() -> Observed {
    let mut observed = Observed::default();
    for line in [
        r#"{"type":"system","subtype":"init","session_id":"s","cwd":"/x","model":"model-id","claude_code_version":"9.9.9","permissionMode":"acceptEdits"}"#,
        r#"{"type":"system","subtype":"hook_response","hook_name":"SessionStart:compact","stdout":"---\nphase: build\n---\n","exit_code":0,"outcome":"success"}"#,
        r#"{"type":"system","subtype":"compact_boundary","compact_metadata":{"trigger":"manual","pre_tokens":9000,"post_tokens":1200}}"#,
        r#"{"type":"result","subtype":"success","is_error":false,"num_turns":1}"#,
    ] {
        observed.ingest(&Event::parse(line).expect("line parses"));
    }
    observed
}

fn run(rows: Vec<DaemonRow>) -> Run {
    Run {
        run_id: "20261124T120000Z-hook".to_owned(),
        reload_hook: true,
        started_at: "2026-11-24T12:00:00Z".to_owned(),
        finished_at: "2026-11-24T12:05:00Z".to_owned(),
        observed: observed(),
        daemon_rows: rows,
        before: Some(steps(3)),
        after: Some(steps(5)),
        errors: Vec::new(),
    }
}

fn row() -> DaemonRow {
    DaemonRow {
        source: "compact".to_owned(),
        progress: "found".to_owned(),
    }
}

#[test]
fn the_json_carries_every_field_the_recommendation_reads() {
    let result = build(run(vec![row()]));
    let json: Value = serde_json::from_str(&result.to_json().expect("json")).expect("parses");
    assert_eq!(json["verdict"], "pass", "verdict");
    assert_eq!(json["reload_hook"], true, "hook flag");
    assert_eq!(json["claude_version"], "9.9.9", "version");
    assert_eq!(json["model"], "model-id", "model");
    assert_eq!(json["compaction"]["happened"], true, "compaction");
    assert_eq!(json["compaction"]["pre_tokens"], 9000, "pre tokens");
    assert_eq!(json["compaction"]["post_tokens"], 1200, "post tokens");
    assert_eq!(json["hook"]["ran"], true, "hook ran");
    assert_eq!(json["hook"]["printed_progress_file"], true, "printed");
    assert_eq!(json["hook"]["daemon_rows"][0]["progress"], "found", "row");
    assert_eq!(
        json["steps"]["finished_before_compaction"],
        serde_json::json!([1, 2, 3]),
        "before"
    );
    assert_eq!(
        json["steps"]["finished_at_end"],
        serde_json::json!([1, 2, 3, 4, 5]),
        "end"
    );
    assert_eq!(json["steps"]["redone"], serde_json::json!([]), "redone");
    assert_eq!(json["steps"]["undone"], serde_json::json!([]), "undone");
    assert_eq!(json["started_at"], "2026-11-24T12:00:00Z", "start");
}

#[test]
fn an_attached_hook_with_no_daemon_row_is_errored() {
    let result = build(run(Vec::new()));
    assert_eq!(
        serde_json::to_value(&result).expect("value")["verdict"],
        "errored",
        "verdict"
    );
}

#[test]
fn a_control_run_is_labelled_and_needs_no_row() {
    let mut input = run(Vec::new());
    input.reload_hook = false;
    let result = build(input);
    let json = serde_json::to_value(&result).expect("value");
    assert_eq!(json["reload_hook"], false, "label");
    assert_eq!(json["hook"]["attached"], false, "attached");
    assert_eq!(json["verdict"], "pass", "verdict");
}

#[test]
fn unreadable_repository_state_is_errored() {
    let mut input = run(vec![row()]);
    input.after = None;
    let result = build(input);
    assert_eq!(
        serde_json::to_value(&result).expect("value")["verdict"],
        "errored",
        "verdict"
    );
}

#[test]
fn an_errored_turn_is_errored() {
    let mut input = run(vec![row()]);
    input.observed.ingest(
        &Event::parse(r#"{"type":"result","subtype":"error","is_error":true,"num_turns":0}"#)
            .expect("parses"),
    );
    let result = build(input);
    assert_eq!(
        serde_json::to_value(&result).expect("value")["verdict"],
        "errored",
        "verdict"
    );
}
