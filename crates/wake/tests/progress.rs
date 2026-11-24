//! Tests for the progress file.

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use wake::progress::{ProgressError, ProgressFile};

const VALID: &str =
    "---\nphase: build\nheadline: Adding the parser\nupdated: 2026-11-24T14:00:00Z\n---\nNotes\n";

fn at(text: &str) -> OffsetDateTime {
    OffsetDateTime::parse(text, &Rfc3339).expect("test timestamp is valid")
}

#[test]
fn reads_a_valid_file() {
    let file = ProgressFile::parse(VALID).expect("valid file");
    assert_eq!(file.phase(), "build", "phase");
    assert_eq!(file.headline(), "Adding the parser", "headline");
    assert_eq!(file.updated(), at("2026-11-24T14:00:00Z"), "updated");
    assert_eq!(file.body(), "Notes\n", "body");
    assert_eq!(
        file.to_string(),
        VALID,
        "unchanged file writes back as read"
    );
}

#[test]
fn rejects_missing_phase() {
    let text = "---\nheadline: h\nupdated: 2026-11-24T14:00:00Z\n---\n";
    assert_eq!(
        ProgressFile::parse(text),
        Err(ProgressError::MissingField { field: "phase" }),
        "missing phase"
    );
}

#[test]
fn rejects_missing_headline() {
    let text = "---\nphase: p\nupdated: 2026-11-24T14:00:00Z\n---\n";
    assert_eq!(
        ProgressFile::parse(text),
        Err(ProgressError::MissingField { field: "headline" }),
        "missing headline"
    );
}

#[test]
fn rejects_empty_headline() {
    let text = "---\nphase: p\nheadline:   \nupdated: 2026-11-24T14:00:00Z\n---\n";
    assert_eq!(
        ProgressFile::parse(text),
        Err(ProgressError::EmptyField { field: "headline" }),
        "empty headline"
    );
}

#[test]
fn rejects_unparsable_updated() {
    let text = "---\nphase: p\nheadline: h\nupdated: yesterday\n---\n";
    assert_eq!(
        ProgressFile::parse(text),
        Err(ProgressError::InvalidUpdated {
            value: "yesterday".to_owned()
        }),
        "unparsable updated"
    );
}

#[test]
fn rejects_missing_updated() {
    let text = "---\nphase: p\nheadline: h\n---\n";
    assert_eq!(
        ProgressFile::parse(text),
        Err(ProgressError::MissingField { field: "updated" }),
        "missing updated"
    );
}

#[test]
fn rejects_missing_front_matter() {
    assert_eq!(
        ProgressFile::parse("just notes\n"),
        Err(ProgressError::MissingFrontMatter),
        "no front matter"
    );
    assert_eq!(
        ProgressFile::parse(""),
        Err(ProgressError::MissingFrontMatter),
        "empty file"
    );
}

#[test]
fn rejects_unterminated_front_matter() {
    assert_eq!(
        ProgressFile::parse("---\nphase: p\n"),
        Err(ProgressError::UnterminatedFrontMatter),
        "no closing line"
    );
}

#[test]
fn rejects_malformed_and_duplicate_lines() {
    assert_eq!(
        ProgressFile::parse("---\nphase p\n---\n"),
        Err(ProgressError::MalformedLine { line: 2 }),
        "line without a colon"
    );
    assert_eq!(
        ProgressFile::parse("---\nphase: a\nphase: b\n---\n"),
        Err(ProgressError::DuplicateField {
            field: "phase".to_owned()
        }),
        "repeated key"
    );
}

#[test]
fn update_keeps_untouched_fields_order_and_body() {
    let text = "---\nowner: me: here\nphase: build\nextra:   spaced  \nheadline: old\nupdated: 2026-11-24T14:00:00Z\nlast: z\n---\n\n# Notes\n\n---\n\ntrailing  \n\n\n";
    let mut file = ProgressFile::parse(text).expect("valid file");
    file.update(None, Some("new headline"), at("2026-11-24T15:30:00+02:00"))
        .expect("update");
    let expected = "---\nowner: me: here\nphase: build\nextra:   spaced  \nheadline: new headline\nupdated: 2026-11-24T15:30:00+02:00\nlast: z\n---\n\n# Notes\n\n---\n\ntrailing  \n\n\n";
    assert_eq!(
        file.to_string(),
        expected,
        "only headline and updated change"
    );
    assert_eq!(file.headline(), "new headline", "headline accessor");
    assert_eq!(file.phase(), "build", "phase accessor");
    assert_eq!(
        file.updated(),
        at("2026-11-24T13:30:00Z"),
        "updated accessor"
    );

    let reread = ProgressFile::parse(&file.to_string()).expect("round trip");
    assert_eq!(reread, file, "reading the output gives the same file");
}

#[test]
fn update_changes_phase_and_keeps_crlf_and_missing_final_newline() {
    let text = "---\r\nphase: a\r\nheadline: h\r\nupdated: 2026-11-24T14:00:00Z\r\n---\r\nbody without newline";
    let mut file = ProgressFile::parse(text).expect("valid file");
    file.update(Some("review"), None, at("2026-11-25T00:00:00Z"))
        .expect("update");
    assert_eq!(
        file.to_string(),
        "---\r\nphase: review\r\nheadline: h\r\nupdated: 2026-11-25T00:00:00Z\r\n---\r\nbody without newline",
        "line endings and body kept"
    );
}

#[test]
fn update_with_a_bad_value_changes_nothing() {
    let mut file = ProgressFile::parse(VALID).expect("valid file");
    assert_eq!(
        file.update(Some("ok"), Some("two\nlines"), at("2026-11-25T00:00:00Z")),
        Err(ProgressError::MultilineValue { field: "headline" }),
        "multi-line headline"
    );
    assert_eq!(
        file.update(Some(" "), None, at("2026-11-25T00:00:00Z")),
        Err(ProgressError::EmptyField { field: "phase" }),
        "blank phase"
    );
    assert_eq!(file.to_string(), VALID, "file untouched");
}

#[test]
fn empty_body_with_closing_line_at_end_of_file() {
    let text = "---\nphase: p\nheadline: h\nupdated: 2026-11-24T14:00:00Z\n---";
    let file = ProgressFile::parse(text).expect("valid file");
    assert_eq!(file.body(), "", "empty body");
    assert_eq!(file.to_string(), text, "written back as read");
}
