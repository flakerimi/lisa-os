//! Registry of installed MCP servers + tool discovery
//! (`docs/PLAN.md` §5.4: "maintains the registry of installed servers,
//! mediates discovery").
//!
//! Manifests are JSON files (Appendix B) installed under the manifest
//! directories; invalid files are skipped with a reason, never fatal —
//! one broken app must not take the bus down. Discovery is a
//! deterministic token-overlap ranking over tool names, descriptions,
//! and app ids ("what can handle 'add a task'?"); semantic ranking via
//! embeddings is a later slice.

use crate::manifest::{Manifest, ManifestError, ToolDecl};
use crate::tier::Tier;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Discovery/listing view of one tool.
#[derive(Debug, Clone, Serialize)]
pub struct ToolRef {
    pub app_id: String,
    pub name: String,
    pub tier: Tier,
    pub description: String,
    pub undoable: bool,
    /// The tool's argument schema, verbatim from its manifest — the
    /// intent router's arg-filler grammar-constrains against it
    /// (liblisa::intent, ADR-0013).
    pub input_schema: Value,
}

#[derive(Debug, Default)]
pub struct LoadReport {
    pub loaded: Vec<String>,
    pub skipped: Vec<(PathBuf, String)>,
}

#[derive(Debug, Default)]
pub struct Registry {
    apps: BTreeMap<String, Manifest>,
}

impl Registry {
    pub fn new() -> Registry {
        Registry::default()
    }

    /// Install or update (replace) one app's manifest.
    pub fn insert(&mut self, manifest: Manifest) -> Result<(), ManifestError> {
        manifest.validate()?;
        self.apps.insert(manifest.app_id.clone(), manifest);
        Ok(())
    }

    /// Load every `*.json` in `dir`. Missing dir → empty report.
    pub fn load_dir(&mut self, dir: &Path) -> LoadReport {
        let mut report = LoadReport::default();
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return report,
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
            .collect();
        paths.sort();
        for path in paths {
            let parsed = std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|text| Manifest::from_json(&text).map_err(|e| e.to_string()));
            match parsed {
                Ok(m) => {
                    report.loaded.push(m.app_id.clone());
                    self.apps.insert(m.app_id.clone(), m);
                }
                Err(reason) => report.skipped.push((path, reason)),
            }
        }
        report
    }

    pub fn len(&self) -> usize {
        self.apps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.apps.is_empty()
    }

    pub fn manifest(&self, app_id: &str) -> Option<&Manifest> {
        self.apps.get(app_id)
    }

    pub fn tool(&self, app_id: &str, name: &str) -> Option<&ToolDecl> {
        self.apps.get(app_id).and_then(|m| m.tool(name))
    }

    /// All tools, app-then-name order.
    pub fn list(&self) -> Vec<ToolRef> {
        self.apps
            .values()
            .flat_map(|m| {
                m.tools.iter().map(|t| ToolRef {
                    app_id: m.app_id.clone(),
                    name: t.name.clone(),
                    tier: t.tier,
                    description: t.description.clone(),
                    undoable: t.undo.is_some(),
                    input_schema: t.input_schema.clone(),
                })
            })
            .collect()
    }

    /// Rank tools against a natural-language query by token overlap:
    /// name-token hits weigh 3, description and app-id hits 1. Tools
    /// with zero overlap are omitted.
    pub fn discover(&self, query: &str) -> Vec<ToolRef> {
        let query_tokens = tokens(query);
        if query_tokens.is_empty() {
            return Vec::new();
        }
        let mut scored: Vec<(i64, ToolRef)> = self
            .list()
            .into_iter()
            .filter_map(|t| {
                let name_tokens = tokens(&t.name);
                let desc_tokens = tokens(&t.description);
                let app_tokens = tokens(&t.app_id);
                let score: i64 = query_tokens
                    .iter()
                    .map(|q| {
                        if name_tokens.contains(q) {
                            3
                        } else if desc_tokens.contains(q) || app_tokens.contains(q) {
                            1
                        } else {
                            0
                        }
                    })
                    .sum();
                (score > 0).then_some((score, t))
            })
            .collect();
        scored.sort_by(|(sa, a), (sb, b)| {
            sb.cmp(sa)
                .then_with(|| a.app_id.cmp(&b.app_id))
                .then_with(|| a.name.cmp(&b.name))
        });
        scored.into_iter().map(|(_, t)| t).collect()
    }
}

fn tokens(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::fixture_calendar_json;

    fn registry() -> Registry {
        let mut r = Registry::new();
        r.insert(Manifest::from_json(&fixture_calendar_json()).unwrap())
            .unwrap();
        r
    }

    #[test]
    fn list_reports_tier_and_undoability() {
        let r = registry();
        let tools = r.list();
        assert_eq!(tools.len(), 3);
        let add = tools.iter().find(|t| t.name == "add_event").unwrap();
        assert_eq!(add.tier, Tier::Write);
        assert!(add.undoable);
        let del = tools.iter().find(|t| t.name == "delete_event").unwrap();
        assert!(!del.undoable);
    }

    #[test]
    fn discover_ranks_name_matches_first_and_omits_misses() {
        let r = registry();
        let hits = r.discover("add a calendar event");
        assert!(!hits.is_empty());
        assert_eq!(hits[0].name, "add_event", "name-token hit ranks first");
        assert!(r.discover("photosynthesis").is_empty());
        assert!(r.discover("").is_empty());
    }

    #[test]
    fn insert_replaces_on_same_app_id() {
        let mut r = registry();
        let mut m = Manifest::from_json(&fixture_calendar_json()).unwrap();
        m.tools.truncate(2);
        r.insert(m).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r.list().len(), 2, "reinstall replaces the old manifest");
    }

    #[test]
    fn load_dir_skips_invalid_files_and_keeps_valid_ones() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("calendar.json"), fixture_calendar_json()).unwrap();
        std::fs::write(dir.path().join("broken.json"), "{ not json").unwrap();
        std::fs::write(
            dir.path().join("badversion.json"),
            fixture_calendar_json().replace("\"lisa_manifest\":1", "\"lisa_manifest\":9"),
        )
        .unwrap();
        std::fs::write(dir.path().join("notes.txt"), "ignored").unwrap();

        let mut r = Registry::new();
        let report = r.load_dir(dir.path());
        assert_eq!(report.loaded, vec!["org.gnome.Calendar".to_string()]);
        assert_eq!(report.skipped.len(), 2);
        assert_eq!(r.len(), 1);

        let mut empty = Registry::new();
        let report = empty.load_dir(Path::new("/definitely/not/a/dir"));
        assert!(report.loaded.is_empty() && report.skipped.is_empty());
    }
}
