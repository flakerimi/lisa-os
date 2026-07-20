//! Engine abstraction. `lisa-inferenced` is a *supervisor, not an engine*
//! (`docs/PLAN.md` §5.1): real inference happens in child processes
//! (llama-server et al.); this trait is the seam between the scheduler/API
//! side and those children. M0 ships the deterministic stub; llama-server
//! proxying lands in M1.

use crate::openai::ChatMessage;
use futures::Stream;
use std::pin::Pin;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("engine unavailable: {0}")]
    Unavailable(String),
}

pub type TokenStream = Pin<Box<dyn Stream<Item = Result<String, EngineError>> + Send>>;

pub trait Engine: Send + Sync {
    fn name(&self) -> &'static str;
    fn generate(&self, messages: Vec<ChatMessage>) -> TokenStream;
}

/// Deterministic echo engine: proves the full plumbing (HTTP, SSE, CLI)
/// without weights. Streams word-sized tokens with a small delay so
/// streaming consumers actually exercise incremental rendering.
pub struct StubEngine;

impl Engine for StubEngine {
    fn name(&self) -> &'static str {
        "stub"
    }

    fn generate(&self, messages: Vec<ChatMessage>) -> TokenStream {
        let last_user = messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.clone())
            .unwrap_or_default();
        let reply = format!(
            "[lisa-inferenced stub] You said: \u{201c}{last_user}\u{201d}. \
             Real engines land in M1 — see docs/PLAN.md \u{a7}5.1."
        );
        let tokens: Vec<String> = reply.split_inclusive(' ').map(str::to_string).collect();
        Box::pin(async_stream::stream! {
            for t in tokens {
                tokio::time::sleep(Duration::from_millis(5)).await;
                yield Ok(t);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn stub_echoes_last_user_message() {
        let engine = StubEngine;
        let msgs = vec![
            ChatMessage {
                role: "system".into(),
                content: "policy".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: "hello lisa".into(),
            },
        ];
        let text: String = engine
            .generate(msgs)
            .map(|t| t.unwrap())
            .collect::<Vec<_>>()
            .await
            .join("");
        assert!(text.contains("hello lisa"), "got: {text}");
    }
}
