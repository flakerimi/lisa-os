//! Task definitions — the reusable, grammar-constrained building blocks
//! the assistant and apps share (`docs/PLAN.md` §5.6 tasks API). Each
//! task is *data*: a system prompt + a JSON Schema. The caller runs it
//! through any OpenAI-compatible endpoint with `response_format:
//! json_schema` (guided generation, §5.1); liblisa stays HTTP-free.
//!
//! First task: **addressed-intent** — the brain of Lisa Ambient
//! (ADR-0011). Given a transcribed utterance, decide whether the speaker
//! was talking *to Lisa*. This is what replaces the wake word: speaking
//! near Lisa is not speaking to Lisa.

use serde_json::{Value, json};

/// A guided-generation task: a system instruction + a result schema.
/// Owned strings so parameterized tasks (extract, classify) work.
pub struct Task {
    pub name: String,
    pub system: String,
    pub schema: Value,
}

impl Task {
    /// Build an OpenAI chat-completions request body for this task over
    /// `input`. Deterministic-friendly (max_tokens capped; the grammar
    /// bounds the shape). Feed to `/v1/chat/completions`.
    pub fn request(&self, input: &str) -> Value {
        json!({
            "messages": [
                {"role": "system", "content": self.system},
                {"role": "user", "content": input},
            ],
            "max_tokens": 200,
            "response_format": {
                "type": "json_schema",
                "json_schema": {"name": self.name, "schema": self.schema},
            },
        })
    }

    /// The GBNF grammar this task constrains generation to — the same one
    /// the server compiles. Exposed for testing and offline validation.
    pub fn grammar(&self) -> Result<String, crate::grammar::GrammarError> {
        crate::grammar::json_schema_to_gbnf(&self.schema)
    }
}

/// Addressed-intent: was this utterance directed at Lisa? (ADR-0011.)
pub fn addressed_intent() -> Task {
    Task {
        name: "addressed_intent".into(),
        system: "You are the ear of Lisa, an on-device assistant. You are given a \
                 transcript of something a person said out loud near the computer. \
                 Decide whether they were addressing Lisa (asking it to do or answer \
                 something) versus talking to another person, thinking aloud, or \
                 background speech. Be conservative: when unsure, addressed is false — \
                 responding when not addressed is worse than missing a request. \
                 Reply ONLY as the JSON object: addressed (bool), confidence (0..1), \
                 intent (a short imperative summary of the request, or empty when not \
                 addressed)."
            .into(),
        schema: json!({
            "type": "object",
            "properties": {
                "addressed": {"type": "boolean"},
                "confidence": {"type": "number"},
                "intent": {"type": "string", "maxLength": 200}
            },
            "required": ["addressed", "confidence", "intent"]
        }),
    }
}

/// Extract structured data matching `schema` from free text — the
/// flagship task (§5.6): "paste text → typed struct". `instruction`
/// tailors it (e.g. "Extract the recipe.").
pub fn extract(name: &str, instruction: &str, schema: Value) -> Task {
    Task {
        name: name.to_string(),
        system: format!(
            "{instruction} Read the text and return ONLY a JSON object matching the \
             required schema. Use empty strings / empty arrays for anything not \
             present; never invent facts."
        ),
        schema,
    }
}

/// Classify text into exactly one of `labels` (+ a confidence).
pub fn classify(labels: &[&str]) -> Task {
    Task {
        name: "classify".into(),
        system: format!(
            "Classify the text into exactly one of these labels: {}. Return ONLY the \
             JSON object with the chosen label and a confidence in 0..1.",
            labels.join(", ")
        ),
        schema: json!({
            "type": "object",
            "properties": {
                "label": {"type": "string", "enum": labels},
                "confidence": {"type": "number"}
            },
            "required": ["label", "confidence"]
        }),
    }
}

/// Summarize text to a short structured result (title + bullet points).
pub fn summarize() -> Task {
    Task {
        name: "summarize".into(),
        system: "Summarize the text. Return ONLY the JSON object: a short title and \
                 up to 5 bullet points capturing the key facts."
            .into(),
        schema: json!({
            "type": "object",
            "properties": {
                "title": {"type": "string", "maxLength": 100},
                "points": {"type": "array", "items": {"type": "string", "maxLength": 200},
                           "minItems": 1, "maxItems": 5}
            },
            "required": ["title", "points"]
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addressed_intent_request_is_grammar_constrained() {
        let task = addressed_intent();
        let req = task.request("hey can you turn off the kitchen light");
        assert_eq!(req["response_format"]["type"], "json_schema");
        assert_eq!(
            req["response_format"]["json_schema"]["name"],
            "addressed_intent"
        );
        // The schema compiles to a grammar (guaranteed-valid output).
        let g = task.grammar().unwrap();
        assert!(g.contains(r#""\"addressed\"""#), "grammar: {g}");
        assert!(g.contains("boolean"), "grammar: {g}");
    }

    #[test]
    fn schema_bounds_the_intent_string() {
        // maxLength keeps a small model from spiraling in the free-text
        // field (the lesson from the guided-gen gate).
        let g = addressed_intent().grammar().unwrap();
        assert!(g.contains("{0,200}"), "intent must be bounded: {g}");
    }

    #[test]
    fn classify_enum_and_extract_compile_to_grammars() {
        let c = classify(&["bug", "feature", "question"]);
        let g = c.grammar().unwrap();
        assert!(g.contains("bug"), "enum labels in grammar: {g}");
        assert!(
            g.contains("feature") && g.contains("question"),
            "grammar: {g}"
        );
        let e = extract(
            "recipe",
            "Extract the recipe.",
            json!({"type":"object","properties":{"title":{"type":"string","maxLength":80}},"required":["title"]}),
        );
        assert!(e.system.contains("Extract the recipe"));
        assert!(e.grammar().unwrap().contains("string-char"));
    }
}
