//! Integration tests for the OpenAI-compat surface. These are the M0
//! forerunners of the §5.1 acceptance block (which additionally requires a
//! real model, latency budgets, and the egress packet counter in CI).

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;

use lisa_inferenced::{api, engine, scheduler};
use std::sync::Arc;

fn test_router() -> axum::Router {
    api::router(api::AppState {
        engine: Arc::new(engine::StubEngine),
        scheduler: Arc::new(scheduler::Scheduler::new(1)),
        model_name: "lisa-system-stub".to_string(),
    })
}

#[tokio::test]
async fn health_reports_ok_and_engine() {
    let res = test_router()
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value =
        serde_json::from_slice(&res.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["engine"], "stub");
}

#[tokio::test]
async fn models_lists_the_resident_model() {
    let res = test_router()
        .oneshot(Request::get("/v1/models").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value =
        serde_json::from_slice(&res.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["data"][0]["id"], "lisa-system-stub");
}

#[tokio::test]
async fn chat_completion_non_streaming_echoes_prompt() {
    let req = Request::post("/v1/chat/completions")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "messages": [{"role": "user", "content": "write a haiku about entropy"}]
            })
            .to_string(),
        ))
        .unwrap();
    let res = test_router().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value =
        serde_json::from_slice(&res.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let content = body["choices"][0]["message"]["content"].as_str().unwrap();
    assert!(
        content.contains("write a haiku about entropy"),
        "got: {content}"
    );
    assert_eq!(body["choices"][0]["finish_reason"], "stop");
    assert_eq!(body["object"], "chat.completion");
}

#[tokio::test]
async fn chat_completion_streaming_emits_sse_chunks_and_done() {
    let req = Request::post("/v1/chat/completions")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "messages": [{"role": "user", "content": "stream me"}],
                "stream": true
            })
            .to_string(),
        ))
        .unwrap();
    let res = test_router().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let ct = res.headers()[header::CONTENT_TYPE]
        .to_str()
        .unwrap()
        .to_string();
    assert!(ct.starts_with("text/event-stream"), "content-type: {ct}");

    let body =
        String::from_utf8(res.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap();
    assert!(body.contains("chat.completion.chunk"), "body: {body}");
    assert!(body.trim_end().ends_with("data: [DONE]"), "body: {body}");

    // Reassemble the deltas the way a real SSE client would.
    let content: String = body
        .lines()
        .filter_map(|l| l.strip_prefix("data: "))
        .filter(|d| *d != "[DONE]")
        .filter_map(|d| serde_json::from_str::<serde_json::Value>(d).ok())
        .filter_map(|c| {
            c["choices"][0]["delta"]["content"]
                .as_str()
                .map(str::to_string)
        })
        .collect();
    assert!(content.contains("stream me"), "reassembled: {content}");
}
