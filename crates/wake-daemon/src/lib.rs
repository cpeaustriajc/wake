//! The Wake daemon: JSON-RPC over a Unix socket that records compactions in
//! SQLite (ADR-011). Nothing listens on a network port.

mod error;
pub mod protocol;
mod server;
mod store;

pub use error::Error;
pub use server::{bind, record_compaction, serve};
pub use store::{Store, StoredCompaction};
