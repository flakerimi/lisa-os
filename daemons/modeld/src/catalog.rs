//! Model catalog parsing. The catalog is signed *data, not code*
//! (`docs/PLAN.md` §5.2): a TOML index describing models, licenses, and
//! hardware requirements. Signature verification (TUF-style) lands in M1.

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("catalog parse error: {0}")]
    Parse(#[from] toml::de::Error),
}

#[derive(Debug, Deserialize)]
pub struct Catalog {
    pub catalog_version: u32,
    #[serde(default, rename = "model")]
    pub models: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
pub struct ModelEntry {
    /// Store ref name, e.g. `qwen3-8b-instruct-q4`.
    pub id: String,
    /// Task slot from PLAN §7: system, vision, embeddings, reranker, asr,
    /// tts, wake-word, code, image-gen.
    pub task: String,
    /// Hardware tiers (PLAN §8) this entry is recommended for.
    pub tiers: Vec<u8>,
    pub license: String,
    /// Inference engine: llama-server, whisper-cpp, sd-cpp, onnx, piper.
    pub engine: String,
    /// Download URL — placeholder until pinned in M1; never invented.
    #[serde(default)]
    pub source: Option<String>,
    /// Pinned blake3 of the exact artifact — populated when `source` is.
    #[serde(default)]
    pub blake3: Option<String>,
    #[serde(default)]
    pub min_ram_gb: Option<u32>,
    #[serde(default)]
    pub notes: Option<String>,
    /// Revocation flag honored on catalog refresh (PLAN §5.10).
    #[serde(default)]
    pub revoked: bool,
}

pub fn parse(toml_str: &str) -> Result<Catalog, CatalogError> {
    Ok(toml::from_str(toml_str)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The seed catalog in-repo must always parse and honor the
    /// license-review policy fields.
    #[test]
    fn seed_catalog_parses() {
        let seed = include_str!("../../../models/catalog/catalog.toml");
        let catalog = parse(seed).unwrap();
        assert_eq!(catalog.catalog_version, 1);
        assert!(!catalog.models.is_empty());
        for m in &catalog.models {
            assert!(!m.id.is_empty());
            assert!(
                !m.license.is_empty(),
                "{}: license review is mandatory",
                m.id
            );
            assert!(!m.tiers.is_empty(), "{}: at least one tier", m.id);
            // A pinned source requires a pinned hash, and vice versa.
            assert_eq!(
                m.source.is_some(),
                m.blake3.is_some(),
                "{}: source and blake3 must be pinned together",
                m.id
            );
        }
    }
}
