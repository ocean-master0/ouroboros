//! Integration tests for GET /api/v1/ping (AC-04, AC-10).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::atomic::Ordering;
use tower::ServiceExt;

use ouroboros_api::build_app;
use ouroboros_api::state::AppState;

#[tokio::test]
async fn test_get_ping_endpoint_and_counter_increment() {
    let state = AppState::test_state();
    let app = build_app(state.clone());

    assert_eq!(state.counters.ping.load(Ordering::Relaxed), 0);
    assert_eq!(state.counters.total.load(Ordering::Relaxed), 0);

    let req = Request::builder()
        .uri("/api/v1/ping")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok()),
        Some("application/json")
    );
    assert!(res.headers().contains_key("x-request-id"));

    let body_bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    assert_eq!(&body_bytes[..], b"{\"success\":true,\"message\":\"pong\"}");

    // AC-10: Single-threaded assertion that counter increments exactly once per call
    assert_eq!(state.counters.ping.load(Ordering::Relaxed), 1);
    assert_eq!(state.counters.total.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn test_ping_counter_sequential_increments() {
    let state = AppState::test_state();

    for i in 1..=5 {
        let app = build_app(state.clone());
        let req = Request::builder()
            .uri("/api/v1/ping")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(state.counters.ping.load(Ordering::Relaxed), i);
        assert_eq!(state.counters.total.load(Ordering::Relaxed), i);
    }
}
