//! Replays recorded `claude` stream-json output through the adapter.

use wake_claude::stream::{Event, ParseError};

const SESSION: &str = include_str!("fixtures/session.jsonl");
const COMPACTION: &str = include_str!("fixtures/compaction.jsonl");

fn events(recording: &str) -> Vec<Event> {
    recording
        .lines()
        .map(|line| Event::parse(line).expect("every recorded line is readable"))
        .collect()
}

#[test]
fn reads_the_init_event() {
    let init = events(SESSION)
        .into_iter()
        .find_map(|event| match event {
            Event::Init(init) => Some(init),
            _ => None,
        })
        .expect("the recording has an init event");
    assert_eq!(init.cwd, "/work/project", "cwd");
    assert_eq!(init.claude_code_version, "2.1.331", "claude version");
    assert_eq!(init.permission_mode, "default", "permission mode");
    assert!(
        init.session_id.starts_with("00000000-"),
        "session id is a placeholder"
    );
    assert!(!init.model.is_empty(), "model");
}

#[test]
fn reads_the_rate_limit_event() {
    let limit = events(SESSION)
        .into_iter()
        .find_map(|event| match event {
            Event::RateLimit(limit) => Some(limit),
            _ => None,
        })
        .expect("the recording has a rate limit event");
    assert_eq!(limit.status, "allowed", "status");
    assert_eq!(limit.rate_limit_type, "seven_day", "window type");
    let window = limit
        .windows
        .get("seven_day")
        .expect("the seven day window is reported");
    assert!(window.utilization >= 0.0, "utilization is a share of usage");
    assert!(window.resets_at > 0, "reset time");
    assert_eq!(window.status, "allowed", "window status");
}

#[test]
fn reads_the_autocompact_state_event() {
    let state = events(SESSION)
        .into_iter()
        .find_map(|event| match event {
            Event::AutocompactState(state) => Some(state),
            _ => None,
        })
        .expect("the recording has an autocompact_state event");
    assert!(state.enabled, "autocompact is on at the default");
    assert!(state.enforced, "threshold is enforced");
    assert_eq!(state.source, "model-default", "the harness default applies");
    assert!(
        state.threshold < state.effective_window,
        "compaction starts before the window is full"
    );
}

#[test]
fn reads_the_events_around_a_real_compaction() {
    let events = events(COMPACTION);
    let hook = events
        .iter()
        .find_map(|event| match event {
            Event::HookResponse(hook) => Some(hook),
            _ => None,
        })
        .expect("the reload hook ran");
    assert_eq!(hook.hook_name, "SessionStart:compact", "hook name");
    assert_eq!(hook.outcome, "success", "hook outcome");
    assert_eq!(hook.exit_code, 0, "hook exit code");
    assert!(
        hook.stdout.contains("headline: Adding the parser"),
        "the hook printed the progress file"
    );
    let compaction = events
        .iter()
        .find_map(|event| match event {
            Event::CompactBoundary(compaction) => Some(compaction),
            _ => None,
        })
        .expect("claude reported the compaction");
    assert_eq!(compaction.trigger, "manual", "trigger");
    assert!(
        compaction.post_tokens < compaction.pre_tokens,
        "context shrank"
    );
    let turns = events
        .iter()
        .filter(|event| matches!(event, Event::Result(_)))
        .count();
    assert_eq!(turns, 3, "three messages were sent");
}

#[test]
fn messages_wake_does_not_read_are_kept_as_other() {
    let event = Event::parse(r#"{"type":"stream_event","event":{}}"#).expect("readable");
    assert_eq!(
        event,
        Event::Other {
            kind: "stream_event".to_owned(),
            subtype: None
        },
        "unread message"
    );
}

#[test]
fn a_changed_shape_is_an_error() {
    let line = r#"{"type":"autocompact_state","value":{"enabled":true}}"#;
    assert!(
        matches!(Event::parse(line), Err(ParseError::Json(_))),
        "a missing field must fail loudly"
    );
}

#[test]
fn rejects_lines_without_a_type_or_json() {
    assert!(
        matches!(Event::parse("{}"), Err(ParseError::MissingType)),
        "no type"
    );
    assert!(
        matches!(Event::parse("not json"), Err(ParseError::Json(_))),
        "not json"
    );
}
