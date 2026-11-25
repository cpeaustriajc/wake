//! Reading tracked cloud tasks from their branches, one `git` command at a time.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use time::format_description::well_known::Rfc3339;
use wake::branch::{BranchError, BranchProgress, read_progress};

use crate::protocol::{AgentEntry, AgentProgress, Location};
use crate::store::StoredCloudTask;

const REMOTE: &str = "origin";

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn found(progress: &BranchProgress) -> AgentProgress {
    match progress.updated.format(&Rfc3339) {
        Ok(updated) => AgentProgress::Found {
            headline: progress.headline.clone(),
            phase: progress.phase.clone(),
            updated,
        },
        Err(error) => AgentProgress::Invalid {
            reason: error.to_string(),
        },
    }
}

fn progress(result: &Result<BranchProgress, BranchError>) -> AgentProgress {
    match result {
        Ok(read) => found(read),
        Err(BranchError::BranchMissing { .. }) => AgentProgress::BranchMissing,
        Err(BranchError::FileMissing { .. }) => AgentProgress::Missing,
        Err(error @ (BranchError::FileInvalid { .. } | BranchError::FileNotText { .. })) => {
            AgentProgress::Invalid {
                reason: error.to_string(),
            }
        }
        Err(error) => AgentProgress::Unreachable {
            reason: error.to_string(),
        },
    }
}

#[derive(Debug, Default)]
pub(crate) struct CloudReader {
    reading: Mutex<()>,
    latest: Mutex<HashMap<(String, String), AgentProgress>>,
}

impl CloudReader {
    pub(crate) fn read(&self, folder: &str, branch: &str) {
        let _one_at_a_time = lock(&self.reading);
        let outcome = progress(&read_progress(Path::new(folder), REMOTE, branch));
        lock(&self.latest).insert((folder.to_owned(), branch.to_owned()), outcome);
    }

    pub(crate) fn entry(&self, task: StoredCloudTask) -> AgentEntry {
        let progress = lock(&self.latest)
            .get(&(task.folder.clone(), task.branch.clone()))
            .cloned()
            .unwrap_or(AgentProgress::Pending);
        AgentEntry {
            id: task.id,
            folder: task.folder,
            location: Location::Cloud,
            branch: Some(task.branch),
            started_at: task.tracked_at,
            progress,
        }
    }
}
