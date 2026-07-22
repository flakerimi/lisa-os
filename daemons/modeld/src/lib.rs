//! lisa-modeld — model catalog & content-addressed store.
//!
//! Spec: `docs/PLAN.md` §5.2. This crate is both the daemon (see `main.rs`)
//! and the library the `lisa` CLI uses for local store operations until the
//! D-Bus surface lands in M1.

pub mod catalog;
pub mod fetch;
pub mod profile;
pub mod recommend;
pub mod store;

pub use catalog::Catalog;
pub use store::{ModelStore, RefEntry, StoreError, VerifyReport};

/// The in-repo seed catalog (data, not law — verify at build time, §0.4).
pub fn seed_catalog() -> Catalog {
    catalog::parse(include_str!("../../../models/catalog/catalog.toml"))
        .expect("seed catalog is valid (enforced by tests)")
}
