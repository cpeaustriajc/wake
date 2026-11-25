//! The Wake daemon: JSON-RPC over a Unix socket that records compactions and
//! keeps the list of agents in SQLite (ADR-011). Nothing listens on a network port.

mod agents;
mod error;
pub mod protocol;
mod server;
mod store;

pub use error::Error;
pub use server::{bind, list_agents, record_compaction, register_agent, serve};
pub use store::{Store, StoredAgent, StoredCompaction};
