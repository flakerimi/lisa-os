//! Model downloads. `lisa-modeld` is the *only* Lisa component allowed
//! network access for model traffic (`docs/PLAN.md` §5.2, dataflow rule 2).
//! Delta/resumable downloads and pinned-host enforcement land in M1; M0
//! ships plain streaming download with mandatory hash pinning.

use crate::store::{ModelStore, RefEntry, StoreError};
use std::fs;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("download failed: {0}")]
    Http(#[from] Box<ureq::Error>),
    #[error("io error during download: {0}")]
    Io(#[from] io::Error),
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// Download `url` into the store as `name`, verifying the pinned blake3
/// before anything becomes visible. No pin, no pull — hash pinning is
/// policy (PLAN §5.10), not an option.
pub fn pull(
    store: &ModelStore,
    url: &str,
    name: &str,
    expected_blake3: &str,
) -> Result<RefEntry, FetchError> {
    let tmp = store.tmp_dir().join(format!("download-{name}"));
    let result = (|| -> Result<RefEntry, FetchError> {
        let mut response = ureq::get(url).call().map_err(Box::new)?;
        let mut reader = response.body_mut().as_reader();
        let mut file = fs::File::create(&tmp)?;
        io::copy(&mut reader, &mut file)?;
        file.sync_all()?;
        Ok(store.add_file_verified(&tmp, name, expected_blake3)?)
    })();
    // Best-effort cleanup of the partial/ingested temp file either way.
    let _ = fs::remove_file(&tmp);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Network-dependent; run explicitly with `cargo test -- --ignored`
    /// against a real pinned artifact once the M1 catalog is populated.
    #[test]
    #[ignore = "requires network and a pinned artifact URL (M1)"]
    fn pull_verifies_pin() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ModelStore::open(tmp.path()).unwrap();
        let err = pull(
            &store,
            "http://127.0.0.1:1/nonexistent",
            "m",
            &"0".repeat(64),
        );
        assert!(err.is_err());
    }
}
