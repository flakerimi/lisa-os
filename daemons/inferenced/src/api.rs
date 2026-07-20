//! HTTP surface: OpenAI-compatible endpoints on loopback (`docs/PLAN.md`
//! §5.1). In production, per-app identity via SO_PEERCRED → portal grants
//! attaches here (M2); guided generation (JSON Schema → GBNF via
//! `liblisa::grammar`) is threaded through to the engine in M1.

use crate::engine::Engine;
use crate::openai::*;
use axum::Router;
use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use futures::StreamExt;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<dyn Engine>,
    pub model_name: String,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/models", get(models))
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "engine": state.engine.name(),
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn models(State(state): State<AppState>) -> Json<ModelList> {
    Json(ModelList {
        object: "list",
        data: vec![ModelInfo {
            id: state.model_name.clone(),
            object: "model",
            created: unix_now(),
            owned_by: "lisa",
        }],
    })
}

async fn chat_completions(
    State(state): State<AppState>,
    Json(req): Json<ChatCompletionRequest>,
) -> Response {
    let model = req
        .model
        .clone()
        .unwrap_or_else(|| state.model_name.clone());
    let id = format!("chatcmpl-lisa-{}", unix_now());
    let created = unix_now();
    let stream = state.engine.generate(req.messages);

    if req.stream {
        let chunk_id = id.clone();
        let chunk_model = model.clone();
        let sse = async_stream::stream! {
            // Role preamble chunk, per OpenAI streaming convention.
            yield sse_json(&ChatCompletionChunk {
                id: chunk_id.clone(),
                object: "chat.completion.chunk",
                created,
                model: chunk_model.clone(),
                choices: vec![ChunkChoice {
                    index: 0,
                    delta: Delta { role: Some("assistant"), content: None },
                    finish_reason: None,
                }],
            });
            let mut stream = stream;
            while let Some(item) = stream.next().await {
                match item {
                    Ok(token) => yield sse_json(&ChatCompletionChunk {
                        id: chunk_id.clone(),
                        object: "chat.completion.chunk",
                        created,
                        model: chunk_model.clone(),
                        choices: vec![ChunkChoice {
                            index: 0,
                            delta: Delta { role: None, content: Some(token) },
                            finish_reason: None,
                        }],
                    }),
                    Err(e) => {
                        yield Ok(Event::default()
                            .data(serde_json::json!({"error": {"message": e.to_string()}}).to_string()));
                        break;
                    }
                }
            }
            yield sse_json(&ChatCompletionChunk {
                id: chunk_id.clone(),
                object: "chat.completion.chunk",
                created,
                model: chunk_model.clone(),
                choices: vec![ChunkChoice {
                    index: 0,
                    delta: Delta::default(),
                    finish_reason: Some("stop"),
                }],
            });
            yield Ok(Event::default().data("[DONE]"));
        };
        return Sse::new(sse)
            .keep_alive(KeepAlive::default())
            .into_response();
    }

    // Non-streaming: aggregate the token stream.
    let tokens: Vec<Result<String, _>> = stream.collect().await;
    let mut content = String::new();
    for t in tokens {
        match t {
            Ok(tok) => content.push_str(&tok),
            Err(e) => {
                return (
                    axum::http::StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({"error": {"message": e.to_string()}})),
                )
                    .into_response();
            }
        }
    }
    let completion_tokens = content.split_whitespace().count() as u32;
    Json(ChatCompletionResponse {
        id,
        object: "chat.completion",
        created,
        model,
        choices: vec![Choice {
            index: 0,
            message: ChatMessage {
                role: "assistant".into(),
                content,
            },
            finish_reason: "stop",
        }],
        usage: Usage {
            prompt_tokens: 0,
            completion_tokens,
            total_tokens: completion_tokens,
        },
    })
    .into_response()
}

fn sse_json<T: serde::Serialize>(value: &T) -> Result<Event, std::convert::Infallible> {
    Ok(Event::default()
        .data(serde_json::to_string(value).expect("wire types serialize infallibly")))
}
