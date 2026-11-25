//! The harder task: eight steps, four choices the agent makes and records only
//! in its progress file, and four later steps that must use them.

use std::collections::BTreeMap;

use crate::task::{COMPACT, CONTINUE, LEDGER};

/// How many numbered steps the harder task has.
pub const STEP_COUNT: u32 = 8;
/// How many steps the agent finishes before `/compact` is sent.
pub const STEPS_BEFORE_COMPACTION: u32 = 4;
/// The progress file, relative to the repository.
pub const PROGRESS_FILE: &str = ".wake/progress.md";
/// The choices in the order steps 1 to 4 make them, and steps 5 to 8 use them.
pub const DECISION_KEYS: [&str; 4] = ["codename", "port", "colour", "motto"];

/// What the agent chose, by key.
pub type Decisions = BTreeMap<String, String>;

/// The path of the file step `number` writes.
#[must_use]
pub fn step_file(number: u32) -> String {
    format!("steps/{number}.txt")
}

/// The ledger line step `number` appends.
#[must_use]
pub fn ledger_line(number: u32) -> String {
    format!("step {number} done")
}

/// The subject of the one commit step `number` makes.
#[must_use]
pub fn commit_subject(number: u32) -> String {
    format!("step {number}")
}

/// The choice step `number` uses, if it is one of steps 5 to 8.
#[must_use]
pub fn decision_used_by(number: u32) -> Option<&'static str> {
    let index = number.checked_sub(STEPS_BEFORE_COMPACTION + 1)?;
    DECISION_KEYS.get(usize::try_from(index).ok()?).copied()
}

/// What step `number`'s file must hold, ignoring surrounding whitespace:
/// `step N` alone for steps 1 to 4, and `step N` then `key=value` after.
#[must_use]
pub fn expected_content(number: u32, decisions: &Decisions) -> Option<String> {
    let head = format!("step {number}");
    match decision_used_by(number) {
        None => Some(head),
        Some(key) => decisions
            .get(key)
            .map(|value| format!("{head}\n{key}={value}")),
    }
}

/// Whether `text` is what step `number` must hold, given `decisions`.
#[must_use]
pub fn content_matches(number: u32, text: &str, decisions: &Decisions) -> bool {
    let normal = |text: &str| {
        text.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    };
    expected_content(number, decisions).is_some_and(|expected| normal(text) == expected)
}

fn valid(key: &str, value: &str) -> bool {
    let lower = |text: &str| !text.is_empty() && text.chars().all(|c| c.is_ascii_lowercase());
    match key {
        "codename" => (6..=10).contains(&value.len()) && lower(value),
        "port" => {
            value.chars().all(|c| c.is_ascii_digit())
                && value
                    .parse::<u32>()
                    .is_ok_and(|port| (20000..=29999).contains(&port))
        }
        "colour" => {
            value.len() == 6
                && value
                    .chars()
                    .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
        }
        "motto" => {
            let words: Vec<&str> = value.split(' ').collect();
            words.len() == 3 && words.iter().all(|word| lower(word))
        }
        _ => false,
    }
}

/// Reads the four choices from a progress file's text.
///
/// A choice is a line `decision <key>: <value>`; a leading list marker and
/// backticks around the value are ignored, and the last line for a key wins.
///
/// # Errors
///
/// Returns every key that is missing or whose value breaks its format.
pub fn parse_decisions(progress: &str) -> Result<Decisions, Vec<String>> {
    let mut found = Decisions::new();
    for line in progress.lines() {
        let line = line.trim().trim_start_matches(['-', '*', ' ']);
        let Some(rest) = line.strip_prefix("decision ") else {
            continue;
        };
        if let Some((key, value)) = rest.split_once(':') {
            found.insert(
                key.trim().to_ascii_lowercase(),
                value.trim().trim_matches('`').trim().to_owned(),
            );
        }
    }
    let problems: Vec<String> = DECISION_KEYS
        .iter()
        .filter_map(|key| match found.get(*key) {
            None => Some(format!("no `decision {key}:` line in the progress file")),
            Some(value) if !valid(key, value) => {
                Some(format!("`decision {key}` has an invalid value `{value}`"))
            }
            Some(_) => None,
        })
        .collect();
    if problems.is_empty() {
        found.retain(|key, _| DECISION_KEYS.contains(&key.as_str()));
        Ok(found)
    } else {
        Err(problems)
    }
}

/// The first message: the whole task, with a stop after step 4.
#[must_use]
pub fn task_prompt() -> String {
    format!(
        "This folder is a scratch git repository. Work through {STEP_COUNT} numbered steps in \
         order. Every step N does three things: write the file steps/N.txt, append the line \
         `step N done` to {LEDGER}, then commit the file, the ledger and {PROGRESS_FILE} with git \
         add and git commit -m \"step N\". Make exactly one commit per step, and never redo, undo \
         or amend a finished step.\n\n\
         Steps 1 to {STEPS_BEFORE_COMPACTION} each need you to make one choice, in this order. \
         Step 1, codename: one unusual lowercase word of 6 to 10 letters. Step 2, port: a number \
         from 20000 to 29999 that is not round. Step 3, colour: six lowercase hex digits with no \
         repeated pair. Step 4, motto: three unrelated lowercase words separated by single \
         spaces. Record each choice in your progress file, {PROGRESS_FILE}, on its own line in the \
         body, written exactly like `decision codename: <value>`, `decision port: <value>`, \
         `decision colour: <value>` and `decision motto: <value>`. Write each choice only in that \
         file: in no other file, in no commit message and not in the ledger. The file steps/N.txt \
         for steps 1 to {STEPS_BEFORE_COMPACTION} holds only the text `step N`.\n\n\
         Steps 5 to {STEP_COUNT} each use one of your choices. The file steps/5.txt holds two \
         lines: `step 5` and `codename=<your codename>`. Steps 6, 7 and 8 do the same with \
         `port=`, `colour=` and `motto=`.\n\n\
         Keep the list of steps and which are finished in the progress file too, and commit it \
         with each step. Do steps 1 to {STEPS_BEFORE_COMPACTION} now, then stop and wait: I will \
         message you again."
    )
}

/// The three user turns of a run, in order.
#[must_use]
pub fn messages() -> [String; 3] {
    [task_prompt(), COMPACT.to_owned(), CONTINUE.to_owned()]
}
