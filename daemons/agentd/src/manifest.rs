//! App intent manifests — the MCP declaration (`docs/PLAN.md` §5.4,
//! Appendix B).
//!
//! An app declares capability in its manifest: tools (typed actions
//! with a tier and an optional undo compensation), resources, and how
//! its MCP server is reached. Manifests are JSON files installed under
//! the manifest directories (see `main.rs`); this module parses and
//! validates them. Unknown top-level fields are allowed (forward
//! compatibility); everything the bus *enforces* — tiers, undo
//! declarations, schemas — is validated strictly.

use crate::tier::Tier;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use thiserror::Error;

pub const MANIFEST_VERSION: u64 = 1;

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("not valid manifest JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported lisa_manifest version {0} (expected {MANIFEST_VERSION})")]
    Version(u64),
    #[error("app_id {0:?} is not a reverse-DNS id")]
    AppId(String),
    #[error("unsupported mcp.transport {0:?} (v1 supports \"unix\")")]
    Transport(String),
    #[error("tool name {0:?} is invalid (want [a-z][a-z0-9_-]*)")]
    ToolName(String),
    #[error("duplicate tool {0:?}")]
    DuplicateTool(String),
    #[error("tool {0:?}: input_schema must be a JSON Schema with \"type\": \"object\"")]
    InputSchema(String),
    #[error("tool {0:?}: undo declared on a read-tier tool (nothing to compensate)")]
    UndoOnRead(String),
    #[error("tool {0:?}: undo tool {1:?} is not declared in this manifest")]
    UndoUnknownTool(String, String),
    #[error("tool {0:?}: undo map value {1:?} is not a literal or a $input/$result path")]
    UndoMapRef(String, String),
    #[error("resource uri {0:?} is empty")]
    ResourceUri(String),
}

/// Appendix B, top level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub lisa_manifest: u64,
    pub app_id: String,
    #[serde(default)]
    pub mcp: McpDecl,
    #[serde(default)]
    pub tools: Vec<ToolDecl>,
    #[serde(default)]
    pub resources: Vec<ResourceDecl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpDecl {
    #[serde(default = "default_transport")]
    pub transport: String,
    #[serde(default)]
    pub activatable: bool,
}

fn default_transport() -> String {
    "unix".to_string()
}

impl Default for McpDecl {
    fn default() -> Self {
        McpDecl {
            transport: default_transport(),
            activatable: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDecl {
    pub name: String,
    pub tier: Tier,
    #[serde(default)]
    pub description: String,
    pub input_schema: Value,
    #[serde(default)]
    pub undo: Option<UndoDecl>,
}

/// The app-provided inverse call: `tool` in the same manifest, args
/// built from `map` — values are literals or `$input`/`$result` paths
/// resolved against the executed call (PLAN §5.4 undo journal).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoDecl {
    pub tool: String,
    #[serde(default)]
    pub map: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDecl {
    pub uri: String,
    #[serde(default)]
    pub description: String,
}

impl Manifest {
    /// Parse and validate one manifest document.
    pub fn from_json(json: &str) -> Result<Manifest, ManifestError> {
        let m: Manifest = serde_json::from_str(json)?;
        m.validate()?;
        Ok(m)
    }

    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.lisa_manifest != MANIFEST_VERSION {
            return Err(ManifestError::Version(self.lisa_manifest));
        }
        if !is_reverse_dns(&self.app_id) {
            return Err(ManifestError::AppId(self.app_id.clone()));
        }
        if self.mcp.transport != "unix" {
            return Err(ManifestError::Transport(self.mcp.transport.clone()));
        }
        let names: HashSet<&str> = self.tools.iter().map(|t| t.name.as_str()).collect();
        let mut seen: HashSet<&str> = HashSet::new();
        for tool in &self.tools {
            if !is_tool_name(&tool.name) {
                return Err(ManifestError::ToolName(tool.name.clone()));
            }
            if !seen.insert(&tool.name) {
                return Err(ManifestError::DuplicateTool(tool.name.clone()));
            }
            if tool.input_schema.get("type").and_then(Value::as_str) != Some("object") {
                return Err(ManifestError::InputSchema(tool.name.clone()));
            }
            if let Some(undo) = &tool.undo {
                if tool.tier == Tier::Read {
                    return Err(ManifestError::UndoOnRead(tool.name.clone()));
                }
                if !names.contains(undo.tool.as_str()) {
                    return Err(ManifestError::UndoUnknownTool(
                        tool.name.clone(),
                        undo.tool.clone(),
                    ));
                }
                for value in undo.map.values() {
                    if value.starts_with('$') && !is_undo_ref(value) {
                        return Err(ManifestError::UndoMapRef(tool.name.clone(), value.clone()));
                    }
                }
            }
        }
        for res in &self.resources {
            if res.uri.trim().is_empty() {
                return Err(ManifestError::ResourceUri(res.uri.clone()));
            }
        }
        Ok(())
    }

    pub fn tool(&self, name: &str) -> Option<&ToolDecl> {
        self.tools.iter().find(|t| t.name == name)
    }
}

fn is_reverse_dns(id: &str) -> bool {
    let segments: Vec<&str> = id.split('.').collect();
    segments.len() >= 2
        && segments.iter().all(|s| {
            !s.is_empty()
                && s.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
}

fn is_tool_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// `$input`, `$result`, or a dotted path into either.
fn is_undo_ref(value: &str) -> bool {
    let rest = match value
        .strip_prefix("$result")
        .or_else(|| value.strip_prefix("$input"))
    {
        Some(rest) => rest,
        None => return false,
    };
    rest.is_empty()
        || (rest.starts_with('.')
            && rest[1..].split('.').all(|seg| {
                !seg.is_empty() && seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            }))
}

/// Minimal structural validation of call args against a tool's
/// `input_schema` — enough to reject malformed calls at the bus without
/// a full JSON Schema engine (guided *generation* against the schema is
/// inferenced's job): type checks, `required`, and
/// `additionalProperties: false`, recursively for nested objects.
pub fn validate_args(schema: &Value, args: &Value) -> Result<(), String> {
    check_value(schema, args, "$")
}

fn check_value(schema: &Value, value: &Value, path: &str) -> Result<(), String> {
    let Some(ty) = schema.get("type").and_then(Value::as_str) else {
        return Ok(()); // Untyped schema node: accept.
    };
    let ok = match ty {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "number" => value.is_number(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        _ => true, // Unknown type keyword: accept.
    };
    if !ok {
        return Err(format!("{path}: expected {ty}"));
    }
    if ty == "object" {
        let obj = value.as_object().expect("checked above");
        let props = schema.get("properties").and_then(Value::as_object);
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for name in required.iter().filter_map(Value::as_str) {
                if !obj.contains_key(name) {
                    return Err(format!("{path}: missing required field {name:?}"));
                }
            }
        }
        let closed = schema.get("additionalProperties") == Some(&Value::Bool(false));
        for (key, val) in obj {
            match props.and_then(|p| p.get(key)) {
                Some(sub) => check_value(sub, val, &format!("{path}.{key}"))?,
                None if closed => {
                    return Err(format!("{path}: unexpected field {key:?}"));
                }
                None => {}
            }
        }
    }
    if ty == "array"
        && let Some(items) = schema.get("items")
    {
        for (i, item) in value.as_array().expect("checked above").iter().enumerate() {
            check_value(items, item, &format!("{path}[{i}]"))?;
        }
    }
    Ok(())
}

/// The Appendix B example, verbatim in shape — shared test fixture.
#[cfg(test)]
pub(crate) fn fixture_calendar_json() -> String {
    serde_json::json!({
        "lisa_manifest": 1,
        "app_id": "org.gnome.Calendar",
        "mcp": { "transport": "unix", "activatable": true },
        "tools": [
            {
                "name": "add_event",
                "tier": "write",
                "description": "Create a calendar event",
                "input_schema": { "type": "object", "required": ["title", "start"],
                    "properties": { "title": {"type": "string"},
                                    "start": {"type": "string", "format": "date-time"},
                                    "end": {"type": "string", "format": "date-time"} } },
                "undo": { "tool": "delete_event", "map": { "event_id": "$result.event_id" } }
            },
            {
                "name": "delete_event",
                "tier": "destructive",
                "description": "Delete a calendar event",
                "input_schema": { "type": "object", "required": ["event_id"],
                    "properties": { "event_id": {"type": "string"} } }
            },
            {
                "name": "list_events",
                "tier": "read",
                "description": "List events in a date range",
                "input_schema": { "type": "object", "properties": {} }
            }
        ],
        "resources": [{ "uri": "selection://current", "description": "Currently selected event" }]
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn calendar_json() -> String {
        fixture_calendar_json()
    }

    #[test]
    fn appendix_b_example_parses() {
        let m = Manifest::from_json(&calendar_json()).unwrap();
        assert_eq!(m.app_id, "org.gnome.Calendar");
        assert_eq!(m.tools.len(), 3);
        let add = m.tool("add_event").unwrap();
        assert_eq!(add.tier, Tier::Write);
        assert_eq!(add.undo.as_ref().unwrap().tool, "delete_event");
        assert!(m.mcp.activatable);
    }

    #[test]
    fn rejects_wrong_version() {
        let json = calendar_json().replace("\"lisa_manifest\":1", "\"lisa_manifest\":2");
        assert!(matches!(
            Manifest::from_json(&json),
            Err(ManifestError::Version(2))
        ));
    }

    #[test]
    fn rejects_bad_app_id() {
        for bad in ["", "noreversedns", ".leading", "a..b", "spaces here.app"] {
            let m = Manifest {
                app_id: bad.into(),
                ..Manifest::from_json(&calendar_json()).unwrap()
            };
            assert!(
                matches!(m.validate(), Err(ManifestError::AppId(_))),
                "{bad:?} should be rejected"
            );
        }
    }

    #[test]
    fn rejects_non_unix_transport() {
        let mut m = Manifest::from_json(&calendar_json()).unwrap();
        m.mcp.transport = "tcp".into();
        assert!(matches!(m.validate(), Err(ManifestError::Transport(_))));
    }

    #[test]
    fn rejects_duplicate_and_invalid_tool_names() {
        let mut m = Manifest::from_json(&calendar_json()).unwrap();
        m.tools.push(m.tools[0].clone());
        assert!(matches!(m.validate(), Err(ManifestError::DuplicateTool(_))));

        let mut m = Manifest::from_json(&calendar_json()).unwrap();
        m.tools[0].name = "Add Event!".into();
        assert!(matches!(m.validate(), Err(ManifestError::ToolName(_))));
    }

    #[test]
    fn rejects_non_object_input_schema() {
        let mut m = Manifest::from_json(&calendar_json()).unwrap();
        m.tools[0].input_schema = json!({"type": "string"});
        assert!(matches!(m.validate(), Err(ManifestError::InputSchema(_))));
    }

    #[test]
    fn rejects_undo_on_read_tier_and_unknown_undo_tool() {
        let mut m = Manifest::from_json(&calendar_json()).unwrap();
        m.tools[2].undo = Some(UndoDecl {
            tool: "delete_event".into(),
            map: BTreeMap::new(),
        });
        assert!(matches!(m.validate(), Err(ManifestError::UndoOnRead(_))));

        let mut m = Manifest::from_json(&calendar_json()).unwrap();
        m.tools[0].undo.as_mut().unwrap().tool = "not_a_tool".into();
        assert!(matches!(
            m.validate(),
            Err(ManifestError::UndoUnknownTool(_, _))
        ));
    }

    #[test]
    fn rejects_malformed_undo_refs_but_allows_literals() {
        let mut m = Manifest::from_json(&calendar_json()).unwrap();
        let undo = m.tools[0].undo.as_mut().unwrap();
        undo.map.insert("mode".into(), "hard".into()); // literal: fine
        undo.map.insert("whole".into(), "$result".into()); // whole-result ref: fine
        m.validate().unwrap();

        for bad in ["$results.id", "$result..id", "$input.", "$foo.bar"] {
            let mut m = Manifest::from_json(&calendar_json()).unwrap();
            m.tools[0]
                .undo
                .as_mut()
                .unwrap()
                .map
                .insert("x".into(), bad.into());
            assert!(
                matches!(m.validate(), Err(ManifestError::UndoMapRef(_, _))),
                "{bad:?} should be rejected"
            );
        }
    }

    #[test]
    fn args_validator_enforces_required_types_and_closed_objects() {
        let schema = json!({
            "type": "object",
            "required": ["title"],
            "additionalProperties": false,
            "properties": {
                "title": {"type": "string"},
                "count": {"type": "integer"},
                "nested": {"type": "object", "required": ["id"],
                           "properties": {"id": {"type": "string"}}},
                "tags": {"type": "array", "items": {"type": "string"}}
            }
        });
        validate_args(&schema, &json!({"title": "x"})).unwrap();
        validate_args(
            &schema,
            &json!({"title": "x", "count": 3, "nested": {"id": "a"}, "tags": ["t"]}),
        )
        .unwrap();
        assert!(
            validate_args(&schema, &json!({})).is_err(),
            "missing required"
        );
        assert!(
            validate_args(&schema, &json!({"title": 7})).is_err(),
            "wrong type"
        );
        assert!(
            validate_args(&schema, &json!({"title": "x", "zzz": 1})).is_err(),
            "closed object"
        );
        assert!(
            validate_args(&schema, &json!({"title": "x", "nested": {}})).is_err(),
            "nested required"
        );
        assert!(
            validate_args(&schema, &json!({"title": "x", "tags": [1]})).is_err(),
            "array item type"
        );
        assert!(validate_args(&schema, &json!("not an object")).is_err());
    }
}
