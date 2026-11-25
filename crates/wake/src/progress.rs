//! The progress file, `.wake/progress.md`: front matter the app reads and a
//! Markdown body the agent owns.

use std::error::Error;
use std::fmt;

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// Where an agent's progress file sits, relative to the folder it works in.
pub const PATH: &str = ".wake/progress.md";

const DELIMITER: &str = "---";
const PHASE: &str = "phase";
const HEADLINE: &str = "headline";
const UPDATED: &str = "updated";

/// Why a progress file could not be read or updated.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProgressError {
    /// The text does not start with a `---` line.
    MissingFrontMatter,
    /// The front matter has no closing `---` line.
    UnterminatedFrontMatter,
    /// A front matter line is not `key: value`.
    MalformedLine {
        /// Line number in the file, starting at 1.
        line: usize,
    },
    /// A key appears twice in the front matter.
    DuplicateField {
        /// The repeated key.
        field: String,
    },
    /// A required field is absent.
    MissingField {
        /// The absent field.
        field: &'static str,
    },
    /// A required field has no value.
    EmptyField {
        /// The empty field.
        field: &'static str,
    },
    /// A value spans more than one line.
    MultilineValue {
        /// The field that was given the value.
        field: &'static str,
    },
    /// `updated` is not an RFC 3339 timestamp.
    InvalidUpdated {
        /// The text found after `updated:`.
        value: String,
    },
    /// A timestamp cannot be written as RFC 3339.
    UnformattableTimestamp,
}

impl fmt::Display for ProgressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFrontMatter => write!(f, "the file does not start with a --- line"),
            Self::UnterminatedFrontMatter => write!(f, "the front matter has no closing --- line"),
            Self::MalformedLine { line } => {
                write!(f, "line {line} of the file is not `key: value`")
            }
            Self::DuplicateField { field } => write!(f, "the field `{field}` appears twice"),
            Self::MissingField { field } => write!(f, "the field `{field}` is missing"),
            Self::EmptyField { field } => write!(f, "the field `{field}` is empty"),
            Self::MultilineValue { field } => {
                write!(f, "the field `{field}` must be a single line")
            }
            Self::InvalidUpdated { value } => {
                write!(f, "`{value}` is not an RFC 3339 timestamp")
            }
            Self::UnformattableTimestamp => {
                write!(f, "the timestamp cannot be written as RFC 3339")
            }
        }
    }
}

impl Error for ProgressError {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Line {
    raw: String,
    key: String,
    value: String,
    terminator: String,
}

impl Line {
    fn set(&mut self, value: String) {
        self.raw = format!("{}: {}{}", self.key, value, self.terminator);
        self.value = value;
    }
}

/// A parsed progress file that writes back exactly what it read, except for
/// the fields an update changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressFile {
    opening: String,
    lines: Vec<Line>,
    closing: String,
    body: String,
    phase: usize,
    headline: usize,
    updated: usize,
    updated_at: OffsetDateTime,
}

fn split_terminator(line: &str) -> (&str, &str) {
    let content = line.strip_suffix('\n').unwrap_or(line);
    let content = content.strip_suffix('\r').unwrap_or(content);
    (content, &line[content.len()..])
}

fn single_line(field: &'static str, value: &str) -> Result<String, ProgressError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ProgressError::EmptyField { field });
    }
    if value.contains(['\n', '\r']) {
        return Err(ProgressError::MultilineValue { field });
    }
    Ok(value.to_owned())
}

impl ProgressFile {
    /// Reads a progress file from its text.
    ///
    /// # Errors
    ///
    /// Returns the [`ProgressError`] for missing or unterminated front matter,
    /// a malformed or repeated line, a missing or empty `phase`, `headline` or
    /// `updated`, or an `updated` that is not RFC 3339.
    pub fn parse(text: &str) -> Result<Self, ProgressError> {
        let mut rest = text.split_inclusive('\n');
        let opening = rest.next().ok_or(ProgressError::MissingFrontMatter)?;
        if split_terminator(opening).0 != DELIMITER {
            return Err(ProgressError::MissingFrontMatter);
        }
        let mut lines: Vec<Line> = Vec::new();
        let mut closing = None;
        for (index, raw) in rest.by_ref().enumerate() {
            let (content, terminator) = split_terminator(raw);
            if content == DELIMITER {
                closing = Some(raw);
                break;
            }
            let malformed = ProgressError::MalformedLine { line: index + 2 };
            let (key, value) = content.split_once(':').ok_or_else(|| malformed.clone())?;
            let key = key.trim();
            if key.is_empty() || key.contains(char::is_whitespace) {
                return Err(malformed);
            }
            if lines.iter().any(|line| line.key == key) {
                return Err(ProgressError::DuplicateField {
                    field: key.to_owned(),
                });
            }
            lines.push(Line {
                raw: raw.to_owned(),
                key: key.to_owned(),
                value: value.trim().to_owned(),
                terminator: terminator.to_owned(),
            });
        }
        let closing = closing.ok_or(ProgressError::UnterminatedFrontMatter)?;
        let body = rest.collect::<String>();

        let find = |field: &'static str| {
            lines
                .iter()
                .position(|line| line.key == field)
                .ok_or(ProgressError::MissingField { field })
        };
        let phase = find(PHASE)?;
        let headline = find(HEADLINE)?;
        let updated = find(UPDATED)?;
        for (index, field) in [(phase, PHASE), (headline, HEADLINE), (updated, UPDATED)] {
            if lines[index].value.is_empty() {
                return Err(ProgressError::EmptyField { field });
            }
        }
        let updated_at = OffsetDateTime::parse(&lines[updated].value, &Rfc3339).map_err(|_| {
            ProgressError::InvalidUpdated {
                value: lines[updated].value.clone(),
            }
        })?;

        Ok(Self {
            opening: opening.to_owned(),
            lines,
            closing: closing.to_owned(),
            body,
            phase,
            headline,
            updated,
            updated_at,
        })
    }

    /// The agent's current phase, free-form.
    #[must_use]
    pub fn phase(&self) -> &str {
        &self.lines[self.phase].value
    }

    /// The one-line summary the fleet view shows.
    #[must_use]
    pub fn headline(&self) -> &str {
        &self.lines[self.headline].value
    }

    /// When the file was last updated.
    #[must_use]
    pub fn updated(&self) -> OffsetDateTime {
        self.updated_at
    }

    /// The Markdown after the front matter, exactly as it was read.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Changes `phase` and/or `headline` and stamps `updated` with `now`.
    /// Every other field, the field order and the body stay byte for byte as
    /// they were. Nothing changes if an error is returned.
    ///
    /// # Errors
    ///
    /// Returns [`ProgressError::EmptyField`] or [`ProgressError::MultilineValue`]
    /// for a value that is blank or spans lines, and
    /// [`ProgressError::UnformattableTimestamp`] if `now` cannot be written as
    /// RFC 3339.
    pub fn update(
        &mut self,
        phase: Option<&str>,
        headline: Option<&str>,
        now: OffsetDateTime,
    ) -> Result<(), ProgressError> {
        let phase = phase.map(|value| single_line(PHASE, value)).transpose()?;
        let headline = headline
            .map(|value| single_line(HEADLINE, value))
            .transpose()?;
        let stamp = now
            .format(&Rfc3339)
            .map_err(|_| ProgressError::UnformattableTimestamp)?;
        if let Some(value) = phase {
            self.lines[self.phase].set(value);
        }
        if let Some(value) = headline {
            self.lines[self.headline].set(value);
        }
        self.lines[self.updated].set(stamp);
        self.updated_at = now;
        Ok(())
    }
}

impl fmt::Display for ProgressFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.opening)?;
        for line in &self.lines {
            f.write_str(&line.raw)?;
        }
        f.write_str(&self.closing)?;
        f.write_str(&self.body)
    }
}
