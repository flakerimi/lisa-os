//! lisa-inferenced — model runtime & scheduler (`docs/PLAN.md` §5.1).
//! Library surface exists so integration tests and (later) the e2e harness
//! can drive the router without going through a spawned process.

pub mod api;
pub mod config;
pub mod dbus;
pub mod engine;
pub mod llama;
pub mod openai;
