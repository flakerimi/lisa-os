//! liblisa — the Lisa OS SDK core.
//!
//! Spec: `docs/PLAN.md` §5.6. This crate is the Rust core that the C ABI,
//! GObject Introspection, and Qt layers wrap. M0 ships the guided-generation
//! grammar module; sessions, tasks, and memory bindings land in M2.

pub mod grammar;
pub mod tasks;

/// SDK version, mirrored into the D-Bus and HTTP surfaces.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
