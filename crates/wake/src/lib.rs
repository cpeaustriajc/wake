//! Wake core library. No feature code yet; this crate exists so the checks
//! have something to run against.

pub mod branch;
pub mod progress;

/// The name Wake presents itself with.
#[must_use]
pub fn name() -> &'static str {
    "Wake"
}
