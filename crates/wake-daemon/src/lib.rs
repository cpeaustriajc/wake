//! The Wake daemon: JSON-RPC over a Unix socket that records compactions and
//! keeps the list of agents and tracked cloud tasks, and where each sits on the
//! floor, in SQLite (ADR-011). It also runs one shell per local agent for the
//! window's terminal. Nothing listens on a network port.

mod agents;
mod base64;
mod cloud;
mod error;
pub mod protocol;
mod server;
mod store;
mod terminal;

pub use error::Error;
pub use server::{
    DEFAULT_POLL_INTERVAL, bind, list_agents, open_terminal, place_on_floor, read_terminal,
    record_compaction, register_agent, resize_terminal, serve, serve_polling, serve_with_shell,
    track_cloud_task, write_terminal,
};
pub use store::{Store, StoredAgent, StoredCloudTask, StoredCompaction};
