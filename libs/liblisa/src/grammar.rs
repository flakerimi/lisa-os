//! JSON Schema → GBNF grammar conversion for guided generation.
//!
//! `lisa-inferenced` compiles a caller-supplied JSON Schema into a GBNF
//! grammar that llama.cpp enforces during sampling, so structured output is
//! guaranteed-valid by construction (`docs/PLAN.md` §5.1, §5.6).
//!
//! M0 subset: `object` (all declared properties emitted, in declaration
//! order), `array`, `string`, `number`, `integer`, `boolean`, `null`,
//! `enum`/`const` of scalars. Optional-property elision, `pattern`,
//! `format`, and composition keywords (`oneOf` et al.) land with the M1
//! 1,000-sample validation gate (§5.1 acceptance).

use serde_json::Value;
use std::fmt::Write as _;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GrammarError {
    #[error("invalid schema: {0}")]
    Invalid(String),
    #[error("unsupported schema construct (M0 subset): {0}")]
    Unsupported(String),
}

/// Convert a JSON Schema value into a GBNF grammar string whose start rule
/// is `root`.
pub fn json_schema_to_gbnf(schema: &Value) -> Result<String, GrammarError> {
    let mut g = Generator::default();
    let root_body = g.visit(schema, "root")?;
    g.define("root", &root_body);

    let mut out = String::new();
    // Emit root first, then the rest in definition order.
    for (name, body) in g
        .rules
        .iter()
        .filter(|(n, _)| n == "root")
        .chain(g.rules.iter().filter(|(n, _)| n != "root"))
    {
        writeln!(out, "{name} ::= {body}").expect("writing to String cannot fail");
    }
    Ok(out)
}

#[derive(Default)]
struct Generator {
    rules: Vec<(String, String)>,
}

impl Generator {
    fn define(&mut self, name: &str, body: &str) -> String {
        if !self.rules.iter().any(|(n, _)| n == name) {
            self.rules.push((name.to_string(), body.to_string()));
        }
        name.to_string()
    }

    fn primitive(&mut self, name: &str) -> String {
        let body = match name {
            "space" => r#"" "?"#,
            "string" => {
                r#""\"" ( [^"\\] | "\\" (["\\bfnrt] | "u" [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F]) )* "\"" space"#
            }
            "number" => r#""-"? ([0-9] | [1-9] [0-9]*) ("." [0-9]+)? ([eE] [-+]? [0-9]+)? space"#,
            "integer" => r#""-"? ([0-9] | [1-9] [0-9]*) space"#,
            "boolean" => r#"("true" | "false") space"#,
            "null" => r#""null" space"#,
            other => unreachable!("unknown primitive {other}"),
        };
        // Primitives reference `space`; make sure it exists.
        if name != "space" {
            self.primitive("space");
        }
        self.define(name, body)
    }

    /// Returns the *body* of the rule for `schema`; the caller decides
    /// whether to inline it or bind it to a named rule.
    fn visit(&mut self, schema: &Value, name: &str) -> Result<String, GrammarError> {
        let obj = schema
            .as_object()
            .ok_or_else(|| GrammarError::Invalid("schema must be a JSON object".into()))?;

        if let Some(values) = obj.get("enum") {
            return self.literal_alternatives(
                values
                    .as_array()
                    .ok_or_else(|| GrammarError::Invalid("enum must be an array".into()))?,
            );
        }
        if let Some(value) = obj.get("const") {
            return self.literal_alternatives(std::slice::from_ref(value));
        }

        let ty = obj
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| GrammarError::Unsupported("schema without a `type`".into()))?;

        match ty {
            "string" => {
                let min = obj.get("minLength").and_then(Value::as_u64);
                let max = obj.get("maxLength").and_then(Value::as_u64);
                if min.is_none() && max.is_none() {
                    return Ok(self.primitive("string"));
                }
                // Bounded string: repeat the character class {min,max} so
                // the grammar itself terminates (structural loop bound).
                self.primitive("space");
                let char_rule = self.define(
                    "string-char",
                    r#"( [^"\\] | "\\" (["\\bfnrt] | "u" [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F]) )"#,
                );
                Ok(format!(
                    r#""\"" {char_rule}{} "\"" space"#,
                    repetition(min.unwrap_or(0), max)
                ))
            }
            "number" => Ok(self.primitive("number")),
            "integer" => Ok(self.primitive("integer")),
            "boolean" => Ok(self.primitive("boolean")),
            "null" => Ok(self.primitive("null")),
            "array" => {
                let items = obj
                    .get("items")
                    .ok_or_else(|| GrammarError::Unsupported("array without `items`".into()))?;
                let item_rule = self.named(items, &format!("{name}-item"))?;
                self.primitive("space");
                let min = obj.get("minItems").and_then(Value::as_u64).unwrap_or(0);
                let max = obj.get("maxItems").and_then(Value::as_u64);
                if let Some(max) = max
                    && max < min
                {
                    return Err(GrammarError::Invalid("maxItems < minItems".into()));
                }
                // Bound the repetition structurally when limits are given —
                // unbounded loops are where small models spiral until the
                // token cap truncates mid-structure.
                let body = if min == 0 {
                    let inner_max = max.map(|m| m.saturating_sub(1));
                    if max == Some(0) {
                        String::new()
                    } else {
                        format!(
                            r#"( {item_rule} ( "," space {item_rule} ){} )? "#,
                            repetition(0, inner_max)
                        )
                    }
                } else {
                    format!(
                        r#"{item_rule} ( "," space {item_rule} ){} "#,
                        repetition(min - 1, max.map(|m| m - 1))
                    )
                };
                Ok(format!(r#""[" space {body}"]" space"#))
            }
            "object" => {
                let props = obj
                    .get("properties")
                    .and_then(Value::as_object)
                    .ok_or_else(|| {
                        GrammarError::Unsupported("object without `properties`".into())
                    })?;
                if props.is_empty() {
                    return Err(GrammarError::Unsupported(
                        "object with no properties".into(),
                    ));
                }
                self.primitive("space");
                let mut parts = vec![r#""{" space"#.to_string()];
                for (i, (key, prop_schema)) in props.iter().enumerate() {
                    let prop_rule =
                        self.named(prop_schema, &format!("{name}-{}", sanitize(key)))?;
                    if i > 0 {
                        parts.push(r#""," space"#.to_string());
                    }
                    parts.push(format!(
                        r#"{} space ":" space {prop_rule}"#,
                        gbnf_string_literal(&format!("\"{key}\""))
                    ));
                }
                parts.push(r#""}" space"#.to_string());
                Ok(parts.join(" "))
            }
            other => Err(GrammarError::Unsupported(format!("type `{other}`"))),
        }
    }

    /// Visit `schema` and bind non-primitive results to a named rule so the
    /// grammar stays readable and shared subtrees aren't duplicated inline.
    fn named(&mut self, schema: &Value, name: &str) -> Result<String, GrammarError> {
        let body = self.visit(schema, name)?;
        // Primitive visits return a bare rule name; reuse it directly.
        if self.rules.iter().any(|(n, _)| *n == body) {
            Ok(body)
        } else {
            Ok(self.define(name, &body))
        }
    }

    fn literal_alternatives(&mut self, values: &[Value]) -> Result<String, GrammarError> {
        self.primitive("space");
        let mut alts = Vec::new();
        for v in values {
            let lit = match v {
                Value::String(s) => gbnf_string_literal(&format!("\"{}\"", escape_json(s))),
                Value::Number(n) => gbnf_string_literal(&n.to_string()),
                Value::Bool(b) => gbnf_string_literal(&b.to_string()),
                Value::Null => gbnf_string_literal("null"),
                _ => {
                    return Err(GrammarError::Unsupported(
                        "non-scalar enum/const values".into(),
                    ));
                }
            };
            alts.push(lit);
        }
        Ok(format!("({}) space", alts.join(" | ")))
    }
}

/// Escape a string for embedding inside a JSON string literal.
fn escape_json(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect(),
            c => vec![c],
        })
        .collect()
}

/// Quote a chunk of literal output text as a GBNF terminal.
fn gbnf_string_literal(s: &str) -> String {
    let escaped: String = s
        .chars()
        .flat_map(|c| match c {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect(),
            c => vec![c],
        })
        .collect();
    format!("\"{escaped}\"")
}

fn sanitize(key: &str) -> String {
    key.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// GBNF repetition suffix for {min,max}: `*` when unbounded from zero,
/// `{n,}` / `{n,m}` otherwise (llama.cpp supports bounded repetition).
fn repetition(min: u64, max: Option<u64>) -> String {
    match (min, max) {
        (0, None) => "*".to_string(),
        (n, None) => format!("{{{n},}}"),
        (n, Some(m)) => format!("{{{n},{m}}}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn recipe_schema_produces_root_and_property_rules() {
        let schema = json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "servings": { "type": "integer" },
                "ingredients": { "type": "array", "items": { "type": "string" } }
            },
            "required": ["title", "servings", "ingredients"]
        });
        let g = json_schema_to_gbnf(&schema).unwrap();
        assert!(
            g.starts_with("root ::="),
            "grammar must start with root: {g}"
        );
        assert!(
            g.contains(r#""\"title\"""#),
            "title key literal missing: {g}"
        );
        assert!(g.contains("integer ::="), "integer primitive missing: {g}");
        assert!(g.contains(r#""[" space"#), "array rule missing: {g}");
    }

    #[test]
    fn shared_primitives_are_defined_once() {
        let schema = json!({
            "type": "object",
            "properties": {
                "a": { "type": "string" },
                "b": { "type": "string" }
            }
        });
        let g = json_schema_to_gbnf(&schema).unwrap();
        assert_eq!(g.matches("string ::=").count(), 1, "grammar: {g}");
    }

    #[test]
    fn string_enum_becomes_literal_alternatives() {
        let schema = json!({ "enum": ["red", "green", "blue"] });
        let g = json_schema_to_gbnf(&schema).unwrap();
        assert!(
            g.contains(r#""\"red\"" | "\"green\"" | "\"blue\"""#),
            "grammar: {g}"
        );
    }

    #[test]
    fn nested_objects_get_named_rules() {
        let schema = json!({
            "type": "object",
            "properties": {
                "author": {
                    "type": "object",
                    "properties": { "name": { "type": "string" } }
                }
            }
        });
        let g = json_schema_to_gbnf(&schema).unwrap();
        assert!(g.contains("root-author ::="), "grammar: {g}");
    }

    #[test]
    fn bounded_array_emits_finite_repetition() {
        let schema = json!({
            "type": "array",
            "items": { "type": "string" },
            "minItems": 1,
            "maxItems": 12
        });
        let g = json_schema_to_gbnf(&schema).unwrap();
        assert!(g.contains("{0,11}"), "grammar: {g}");
        assert!(
            !g.contains(r#"( "," space string )*"#),
            "unbounded item loop left in grammar: {g}"
        );
    }

    #[test]
    fn bounded_string_emits_char_repetition() {
        let schema = json!({ "type": "string", "maxLength": 80 });
        let g = json_schema_to_gbnf(&schema).unwrap();
        assert!(g.contains("string-char"), "grammar: {g}");
        assert!(g.contains("{0,80}"), "grammar: {g}");
    }

    #[test]
    #[ignore = "debug helper: cargo test -p liblisa dump_ -- --ignored --nocapture"]
    fn dump_harness_grammar() {
        let schema = json!({
            "type":"object",
            "properties":{
                "title":{"type":"string","maxLength":80},
                "servings":{"type":"integer"},
                "vegetarian":{"type":"boolean"},
                "ingredients":{"type":"array","items":{"type":"string","maxLength":60},"minItems":1,"maxItems":12}
            },
            "required":["title","servings","vegetarian","ingredients"]
        });
        println!("{}", json_schema_to_gbnf(&schema).unwrap());
    }

    #[test]
    fn max_items_below_min_items_is_invalid() {
        let schema = json!({
            "type": "array",
            "items": { "type": "string" },
            "minItems": 3,
            "maxItems": 1
        });
        assert!(matches!(
            json_schema_to_gbnf(&schema),
            Err(GrammarError::Invalid(_))
        ));
    }

    #[test]
    fn schema_without_type_is_unsupported() {
        let schema = json!({ "description": "anything" });
        assert!(matches!(
            json_schema_to_gbnf(&schema),
            Err(GrammarError::Unsupported(_))
        ));
    }

    #[test]
    fn generated_grammar_lines_are_well_formed() {
        let schema = json!({
            "type": "object",
            "properties": {
                "done": { "type": "boolean" },
                "score": { "type": "number" }
            }
        });
        let g = json_schema_to_gbnf(&schema).unwrap();
        for line in g.lines() {
            assert!(line.contains("::="), "malformed rule line: {line}");
        }
    }
}
