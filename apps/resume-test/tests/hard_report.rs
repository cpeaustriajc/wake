//! The harder task's result file and how the stream fills it.

use serde_json::Value;
use wake_claude::stream::Event;
use wake_resume_test::hard_judge::{HardStep, Snapshot};
use wake_resume_test::hard_report::{HardObserved, HardRun, build_hard};
use wake_resume_test::hard_task::Decisions;
use wake_resume_test::report::DaemonRow;

fn steps(finished: u32) -> Vec<HardStep> {
    (1..=8)
        .map(|number| {
            let done = number <= finished;
            HardStep {
                number,
                file_present: done,
                file_correct: done,
                ledger_lines: usize::from(done),
                commits: usize::from(done),
                commit_events: usize::from(done),
                reverted: false,
            }
        })
        .collect()
}

fn run() -> HardRun {
    let mut observed = HardObserved::default();
    for line in [
        r#"{"type":"system","subtype":"init","session_id":"s","cwd":"/x","model":"model-id","claude_code_version":"9.9.9","permissionMode":"acceptEdits"}"#,
        r#"{"type":"autocompact_state","value":{"enabled":true,"effective_window":980000,"threshold":901600,"enforced":true,"source":"model-default"}}"#,
        r#"{"type":"system","subtype":"compact_boundary","compact_metadata":{"trigger":"manual","pre_tokens":9000,"post_tokens":1200}}"#,
        r#"{"type":"result","subtype":"success","is_error":false,"num_turns":1}"#,
    ] {
        observed.ingest(&Event::parse(line).expect("line parses"));
    }
    HardRun {
        run_id: "20261125T120000Z-harder-control".to_owned(),
        trial: true,
        reload_hook: false,
        started_at: "2026-11-25T12:00:00Z".to_owned(),
        finished_at: "2026-11-25T12:05:00Z".to_owned(),
        observed,
        daemon_rows: vec![DaemonRow {
            source: "compact".to_owned(),
            progress: "found".to_owned(),
        }],
        snapshot: Some(Snapshot {
            progress: Some("p".to_owned()),
            decisions: Ok(Decisions::new()),
            leaks: Vec::new(),
        }),
        before: Some(steps(4)),
        after: Some(steps(8)),
        errors: Vec::new(),
    }
}

#[test]
fn the_json_names_the_task_the_trial_flag_and_the_compaction_threshold() {
    let json: Value =
        serde_json::from_str(&build_hard(run()).to_json().expect("json")).expect("parses");
    assert_eq!(json["task"], "harder");
    assert_eq!(json["trial"], true);
    assert_eq!(json["verdict"], "pass");
    assert_eq!(json["autocompact"]["threshold"], 901_600);
    assert_eq!(json["compaction"]["pre_tokens"], 9000);
    assert_eq!(json["steps"]["total"], 8);
    assert!(json["snapshot_before_compaction"]["decisions"].is_object());
}
