//! Integration tests for GET /health (AC-03).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use ouroboros_api::build_app;
use ouroboros_api::state::AppState;

#[tokio::test]
async fn test_get_health_endpoint() {
    let state = AppState::test_state();
    let app = build_app(state);

    let req = Request::builder()
        .uri("/health")
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
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(json["status"], "ok");
    assert!(json["uptime_secs"].is_u64());
}
