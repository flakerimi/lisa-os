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
pub struct Task {
    pub name: &'static str,
    pub system: &'static str,
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
        name: "addressed_intent",
        system: "You are the ear of Lisa, an on-device assistant. You are given a \
                 transcript of something a person said out loud near the computer. \
                 Decide whether they were addressing Lisa (asking it to do or answer \
                 something) versus talking to another person, thinking aloud, or \
                 background speech. Be conservative: when unsure, addressed is false — \
                 responding when not addressed is worse than missing a request. \
                 Reply ONLY as the JSON object: addressed (bool), confidence (0..1), \
                 intent (a short imperative summary of the request, or empty when not \
                 addressed).",
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
}
