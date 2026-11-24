//! The fixed task the agent is given, and the evidence each step leaves.

/// How many numbered steps the task has.
pub const STEP_COUNT: u32 = 5;
/// How many steps the agent is asked to finish before `/compact` is sent.
pub const STEPS_BEFORE_COMPACTION: u32 = 3;
/// The user turn that makes `claude` compact. No compaction setting is touched.
pub const COMPACT: &str = "/compact";
/// The only thing the agent is told after compaction.
pub const CONTINUE: &str = "continue";
/// The file each finished step appends one line to.
pub const LEDGER: &str = "LEDGER.md";

/// The path of the file step `number` creates.
#[must_use]
pub fn step_file(number: u32) -> String {
    format!("steps/{number}.txt")
}

/// What step `number`'s file holds, ignoring surrounding whitespace.
#[must_use]
pub fn step_content(number: u32) -> String {
    format!("step {number}")
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

/// The first message: the whole task, with a stop after the first steps.
#[must_use]
pub fn task_prompt() -> String {
    format!(
        "This folder is a scratch git repository. Work through {STEP_COUNT} numbered steps in \
         order. For step N: write the file steps/N.txt containing exactly the text `step N`, \
         append the line `step N done` to {LEDGER}, then commit both with git add and git commit \
         -m \"step N\". Make exactly one commit per step, and never redo, undo or amend a \
         finished step. Keep the list of steps and which are finished in your progress file, \
         .wake/progress.md, and commit it with each step. Do steps 1 to {STEPS_BEFORE_COMPACTION} \
         now, then stop and wait: I will message you again."
    )
}

/// The three user turns of a run, in order.
#[must_use]
pub fn messages() -> [String; 3] {
    [task_prompt(), COMPACT.to_owned(), CONTINUE.to_owned()]
}
