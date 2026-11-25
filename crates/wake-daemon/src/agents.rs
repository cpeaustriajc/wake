//! Turning stored agents into list entries by reading their progress files.

use std::fs;
use std::io;
use std::path::Path;

use time::format_description::well_known::Rfc3339;
use wake::progress::{PATH, ProgressFile};

use crate::protocol::{AgentEntry, AgentProgress};
use crate::store::StoredAgent;

fn progress(folder: &Path) -> AgentProgress {
    let text = match fs::read_to_string(folder.join(PATH)) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return AgentProgress::Missing,
        Err(error) => {
            return AgentProgress::Invalid {
                reason: format!("cannot be read: {error}"),
            };
        }
    };
    let file = match ProgressFile::parse(&text) {
        Ok(file) => file,
        Err(error) => {
            return AgentProgress::Invalid {
                reason: error.to_string(),
            };
        }
    };
    match file.updated().format(&Rfc3339) {
        Ok(updated) => AgentProgress::Found {
            headline: file.headline().to_owned(),
            phase: file.phase().to_owned(),
            updated,
        },
        Err(error) => AgentProgress::Invalid {
            reason: error.to_string(),
        },
    }
}

pub(crate) fn entry(agent: StoredAgent) -> AgentEntry {
    let progress = progress(Path::new(&agent.folder));
    AgentEntry {
        id: agent.id,
        folder: agent.folder,
        location: agent.location,
        branch: None,
        started_at: agent.started_at,
        progress,
        position: agent.position,
    }
}
