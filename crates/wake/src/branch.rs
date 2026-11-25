//! Reading a task's progress file from its branch with `git`, without
//! checking the branch out.

use std::error::Error;
use std::fmt;
use std::path::Path;
use std::process::{Command, Output};

use time::OffsetDateTime;

use crate::progress::{ProgressError, ProgressFile};

const PROGRESS_PATH: &str = ".wake/progress.md";
const LS_REMOTE_NO_MATCH: i32 = 2;
const REPOSITORY_VARIABLES: [&str; 15] = [
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CONFIG",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
    "GIT_OBJECT_DIRECTORY",
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_GRAFT_FILE",
    "GIT_INDEX_FILE",
    "GIT_NO_REPLACE_OBJECTS",
    "GIT_REPLACE_REF_BASE",
    "GIT_PREFIX",
    "GIT_SHALLOW_FILE",
    "GIT_COMMON_DIR",
];

/// What the progress file held on one branch at one commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchProgress {
    /// The full commit id the branch pointed at when it was fetched.
    pub commit: String,
    /// The agent's phase at that commit.
    pub phase: String,
    /// The agent's one-line summary at that commit.
    pub headline: String,
    /// The `updated` time written in the file at that commit.
    pub updated: OffsetDateTime,
}

/// Why a branch's progress file could not be read.
#[derive(Debug)]
#[non_exhaustive]
pub enum BranchError {
    /// The remote has no branch with this name.
    BranchMissing {
        /// The branch that was asked for.
        branch: String,
    },
    /// The branch exists but has no `.wake/progress.md`.
    FileMissing {
        /// The branch that was read.
        branch: String,
        /// The commit that was read.
        commit: String,
    },
    /// The file is there but is not a valid progress file.
    FileInvalid {
        /// The commit that was read.
        commit: String,
        /// What is wrong with the file.
        source: ProgressError,
    },
    /// The file is there but is not UTF-8 text.
    FileNotText {
        /// The commit that was read.
        commit: String,
    },
    /// `git` could not be run or reported a failure.
    GitFailed {
        /// The `git` arguments that failed.
        command: String,
        /// What `git` printed on standard error, or why it did not start.
        detail: String,
    },
}

impl fmt::Display for BranchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BranchMissing { branch } => write!(f, "the remote has no branch `{branch}`"),
            Self::FileMissing { branch, commit } => {
                write!(f, "branch `{branch}` at {commit} has no {PROGRESS_PATH}")
            }
            Self::FileInvalid { commit, source } => {
                write!(f, "{PROGRESS_PATH} at {commit} is invalid: {source}")
            }
            Self::FileNotText { commit } => {
                write!(f, "{PROGRESS_PATH} at {commit} is not UTF-8 text")
            }
            Self::GitFailed { command, detail } => write!(f, "`git {command}` failed: {detail}"),
        }
    }
}

impl Error for BranchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::FileInvalid { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn git(repo: &Path, args: &[&str]) -> Result<Output, BranchError> {
    let mut command = Command::new("git");
    for variable in REPOSITORY_VARIABLES {
        command.env_remove(variable);
    }
    command
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|error| BranchError::GitFailed {
            command: args.join(" "),
            detail: error.to_string(),
        })
}

fn failure(args: &[&str], output: &Output) -> BranchError {
    BranchError::GitFailed {
        command: args.join(" "),
        detail: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    }
}

fn run(repo: &Path, args: &[&str]) -> Result<Vec<u8>, BranchError> {
    let output = git(repo, args)?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(failure(args, &output))
    }
}

fn fetch(repo: &Path, remote: &str, branch: &str) -> Result<(), BranchError> {
    let refspec = format!("refs/heads/{branch}");
    let args = ["fetch", "--quiet", remote, refspec.as_str()];
    let output = git(repo, &args)?;
    if output.status.success() {
        return Ok(());
    }
    let probe = [
        "ls-remote",
        "--exit-code",
        "--heads",
        remote,
        refspec.as_str(),
    ];
    let listing = git(repo, &probe)?;
    if listing.status.code() == Some(LS_REMOTE_NO_MATCH) {
        return Err(BranchError::BranchMissing {
            branch: branch.to_owned(),
        });
    }
    Err(failure(&args, &output))
}

/// Fetches `branch` from `remote` into the repository at `repo` and reads
/// `.wake/progress.md` from the fetched commit. The working tree, the index
/// and every local branch stay as they were. `git` signs in with whatever
/// credentials the environment already gives it; Wake reads no tokens.
///
/// # Errors
///
/// Returns [`BranchError::BranchMissing`] when the remote has no such branch,
/// [`BranchError::FileMissing`] when the branch has no progress file,
/// [`BranchError::FileInvalid`] or [`BranchError::FileNotText`] when the file
/// cannot be read, and [`BranchError::GitFailed`] when `git` cannot be run or
/// fails for any other reason, such as an unreachable remote.
pub fn read_progress(
    repo: &Path,
    remote: &str,
    branch: &str,
) -> Result<BranchProgress, BranchError> {
    fetch(repo, remote, branch)?;
    let resolved = run(repo, &["rev-parse", "--verify", "FETCH_HEAD^{commit}"])?;
    let commit = String::from_utf8_lossy(&resolved).trim().to_owned();
    let listing = run(
        repo,
        &["ls-tree", "--name-only", &commit, "--", PROGRESS_PATH],
    )?;
    if listing.is_empty() {
        return Err(BranchError::FileMissing {
            branch: branch.to_owned(),
            commit,
        });
    }
    let object = format!("{commit}:{PROGRESS_PATH}");
    let bytes = run(repo, &["show", &object])?;
    let text = String::from_utf8(bytes).map_err(|_| BranchError::FileNotText {
        commit: commit.clone(),
    })?;
    let file = ProgressFile::parse(&text).map_err(|source| BranchError::FileInvalid {
        commit: commit.clone(),
        source,
    })?;
    Ok(BranchProgress {
        commit,
        phase: file.phase().to_owned(),
        headline: file.headline().to_owned(),
        updated: file.updated(),
    })
}
