//! Integration tests for /internal/bench lifecycle (AC-07, AC-08).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use ouroboros_api::build_app;
use ouroboros_api::state::AppState;

#[tokio::test]
async fn test_bench_lifecycle_start_status_stop() {
    let state = AppState::test_state();

    // 1. Initial status: running: false
    let app = build_app(state.clone());
    let req = Request::builder()
        .uri("/internal/bench/status")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let status_json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(status_json["running"], false);

    // 2. Start bench session (direct mode, duration 10s)
    let app = build_app(state.clone());
    let req = Request::builder()
        .uri("/internal/bench/start")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "mode": "direct",
                "duration_secs": 10,
                "concurrency": 2
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let start_json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(start_json["started"], true);

    // 3. Status shows running: true
    let app = build_app(state.clone());
    let req = Request::builder()
        .uri("/internal/bench/status")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let status_json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(status_json["running"], true);

    // 4. Stop bench session
    let app = build_app(state.clone());
    let req = Request::builder()
        .uri("/internal/bench/stop")
        .method("POST")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let stop_json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(stop_json["stopped"], true);

    // 5. Final status: running: false
    let app = build_app(state.clone());
    let req = Request::builder()
        .uri("/internal/bench/status")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let status_json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(status_json["running"], false);
}
