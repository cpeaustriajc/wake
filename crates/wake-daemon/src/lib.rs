//! The Wake daemon: JSON-RPC over a Unix socket that records compactions and
//! keeps the list of agents and tracked cloud tasks in SQLite (ADR-011). Nothing listens on a network port.

mod agents;
mod cloud;
mod error;
pub mod protocol;
mod server;
mod store;

pub use error::Error;
pub use server::{
    DEFAULT_POLL_INTERVAL, bind, list_agents, record_compaction, register_agent, serve,
    serve_polling, track_cloud_task,
};
pub use store::{Store, StoredAgent, StoredCloudTask, StoredCompaction};
