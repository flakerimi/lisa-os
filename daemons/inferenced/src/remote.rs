//! Remote-provider routing (`docs/PLAN.md` §5.11). Model names of the
//! form `remote:<provider>:<model>` are forwarded to the `lisa-remoted`
//! egress broker over its unix socket — inferenced itself stays
//! network-free (rule 5). The broker enforces per-scope consent and the
//! ledger `remote.` marking; inferenced only proxies.
//!
//! Transport: a minimal HTTP/1.1 POST over the unix socket with
//! `Connection: close` (read to EOF), so no extra HTTP-client dependency
//! and no TCP. Streaming through the broker is a follow-up; today the
//! broker answers non-streaming and we hand the completion back as one
//! response (the CLI/UI still render it).

use crate::engine::{Engine, EngineError, GenerateRequest, TokenStream};
use crate::pool::EngineProvider;
use futures::future::BoxFuture;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

pub const REMOTE_PREFIX: &str = "remote:";

/// Default broker socket (matches lisa-remoted's StateDirectory).
pub fn default_socket() -> PathBuf {
    std::env::var_os("LISA_REMOTED_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/lib/lisa/remoted/remoted.sock"))
}

/// Split `remote:<provider>:<model>` → (provider, model). Model may
/// contain further colons/slashes (HF ids like `org/model:policy`).
fn parse(model: &str) -> Option<(String, String)> {
    let rest = model.strip_prefix(REMOTE_PREFIX)?;
    let (provider, inner) = rest.split_once(':')?;
    if provider.is_empty() || inner.is_empty() {
        return None;
    }
    Some((provider.to_string(), inner.to_string()))
}

/// An engine that proxies one provider+model to the broker socket.
pub struct RemoteEngine {
    socket: PathBuf,
    provider: String,
    model: String,
}

impl RemoteEngine {
    async fn request(
        &self,
        messages: &[crate::openai::ChatMessage],
    ) -> Result<String, EngineError> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
        })
        .to_string();
        // Scopes declared for this request; the broker checks each against
        // its per-scope consent. A bare prompt carries the `prompt` scope.
        let request = format!(
            "POST /v1/chat/completions HTTP/1.1\r\n\
             Host: lisa-remoted\r\n\
             x-lisa-provider: {}\r\n\
             x-lisa-scopes: prompt\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n{}",
            self.provider,
            body.len(),
            body
        );
        let mut stream = UnixStream::connect(&self.socket).await.map_err(|e| {
            EngineError::Unavailable(format!(
                "lisa-remoted socket {}: {e} — is the broker running? (systemctl start lisa-remoted)",
                self.socket.display()
            ))
        })?;
        stream
            .write_all(request.as_bytes())
            .await
            .map_err(|e| EngineError::Unavailable(format!("remoted write: {e}")))?;
        let mut raw = Vec::new();
        stream
            .read_to_end(&mut raw)
            .await
            .map_err(|e| EngineError::Unavailable(format!("remoted read: {e}")))?;

        let text = String::from_utf8_lossy(&raw);
        let (head, body) = text
            .split_once("\r\n\r\n")
            .ok_or_else(|| EngineError::Unavailable("remoted: malformed response".into()))?;
        let status_ok = head.lines().next().is_some_and(|l| l.contains(" 200 "));
        let json: serde_json::Value = serde_json::from_str(body.trim())
            .map_err(|e| EngineError::Unavailable(format!("remoted json: {e} (body: {body})")))?;
        if !status_ok {
            let msg = json["error"]["message"]
                .as_str()
                .unwrap_or("remote provider request failed")
                .to_string();
            return Err(EngineError::Unavailable(msg));
        }
        Ok(json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }
}

impl Engine for RemoteEngine {
    fn name(&self) -> &'static str {
        "remote"
    }

    fn generate(&self, req: GenerateRequest) -> TokenStream {
        // RemoteEngine is constructed per-request, so move its fields in.
        let socket = self.socket.clone();
        let provider = self.provider.clone();
        let model = self.model.clone();
        Box::pin(async_stream::stream! {
            let engine = RemoteEngine { socket, provider, model };
            match engine.request(&req.messages).await {
                Ok(content) => {
                    // Hand back word-sized tokens for a streaming feel.
                    for tok in content.split_inclusive(' ') {
                        yield Ok(tok.to_string());
                    }
                }
                Err(e) => yield Err(e),
            }
        })
    }

    fn embed(&self, _texts: Vec<String>) -> BoxFuture<'static, Result<Vec<Vec<f32>>, EngineError>> {
        Box::pin(async {
            Err(EngineError::Unavailable(
                "remote providers serve chat only; embeddings run on a local model".into(),
            ))
        })
    }
}

/// Wraps any EngineProvider: intercepts `remote:` model names and routes
/// them to the broker; everything else delegates to the local provider.
/// Keeps the api/scheduler/ledger path unchanged for local models.
pub struct RemoteRouter {
    inner: Arc<dyn EngineProvider>,
    socket: PathBuf,
}

impl RemoteRouter {
    pub fn new(inner: Arc<dyn EngineProvider>, socket: PathBuf) -> Self {
        Self { inner, socket }
    }
}

impl EngineProvider for RemoteRouter {
    fn engine_for(
        &self,
        model: Option<&str>,
    ) -> BoxFuture<'_, Result<Arc<dyn Engine>, EngineError>> {
        if let Some(m) = model
            && m.starts_with(REMOTE_PREFIX)
        {
            let parsed = parse(m);
            let socket = self.socket.clone();
            return Box::pin(async move {
                let (provider, model) = parsed.ok_or_else(|| {
                    EngineError::Unavailable(
                        "remote model must be remote:<provider>:<model>, \
                         e.g. remote:huggingface:openai/gpt-oss-120b"
                            .into(),
                    )
                })?;
                Ok(Arc::new(RemoteEngine {
                    socket,
                    provider,
                    model,
                }) as Arc<dyn Engine>)
            });
        }
        self.inner.engine_for(model)
    }

    fn known_models(&self) -> Vec<String> {
        // Remote models are dynamic (provider + arbitrary id); the local
        // set is authoritative for /v1/models. `lisa remote` lists what's
        // configured.
        self.inner.known_models()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_remote_model_names() {
        assert_eq!(
            parse("remote:huggingface:openai/gpt-oss-120b"),
            Some(("huggingface".into(), "openai/gpt-oss-120b".into()))
        );
        assert_eq!(
            parse("remote:hf:org/model:cheapest"),
            Some(("hf".into(), "org/model:cheapest".into()))
        );
        assert_eq!(parse("remote:openai"), None, "model required");
        assert_eq!(parse("qwen3-8b"), None, "not a remote name");
    }

    #[tokio::test]
    async fn router_delegates_local_names_to_inner() {
        use crate::pool::SingleEngine;
        let inner = Arc::new(SingleEngine {
            engine: Arc::new(crate::engine::StubEngine),
            name: "lisa-system-stub".into(),
        });
        let router = RemoteRouter::new(inner, PathBuf::from("/nonexistent.sock"));
        // A local name resolves to the stub engine (not the broker).
        let engine = router.engine_for(Some("lisa-system-stub")).await.unwrap();
        assert_eq!(engine.name(), "stub");
    }

    #[tokio::test]
    async fn router_routes_remote_names_to_a_remote_engine() {
        use crate::pool::SingleEngine;
        let inner = Arc::new(SingleEngine {
            engine: Arc::new(crate::engine::StubEngine),
            name: "lisa-system-stub".into(),
        });
        let router = RemoteRouter::new(inner, PathBuf::from("/nonexistent.sock"));
        let engine = router
            .engine_for(Some("remote:huggingface:org/model"))
            .await
            .unwrap();
        assert_eq!(engine.name(), "remote");
    }
}
