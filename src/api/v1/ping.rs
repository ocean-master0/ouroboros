/// GET /api/v1/ping — the hot-path benchmark target endpoint.
///
/// **Zero-allocation contract:** response body is a `&'static [u8]` baked at
/// compile time. No `serde_json::to_string` at request time, no DB access,
/// no logging on this route.
///
/// Counter uses `Ordering::Relaxed` — correct because this counter is read
/// only by the 1-second reporter tick (metrics/reporter.rs), which does not
/// need happens-before ordering relative to individual increments.
use axum::http::{header, StatusCode};
use axum::response::Response;
use bytes::Bytes;

/// Pre-serialized pong response body — zero allocation per request.
static PONG_BODY: &[u8] = br#"{"success":true,"message":"pong"}"#;

/// Hot-path HTTP handler for `GET /api/v1/ping`.
pub async fn ping_handler(
    axum::extract::State(state): axum::extract::State<crate::state::AppState>,
) -> Response {
    state
        .counters
        .ping
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CONTENT_LENGTH, PONG_BODY.len())
        .header("Connection", "keep-alive")
        .body(axum::body::Body::from(Bytes::from_static(PONG_BODY)))
        .unwrap()
}

/// Direct-mode callable — same logic, no HTTP overhead.
/// Used by the bench engine's "direct" mode (§11 of MVP_TECH_DOC).
pub async fn ping_direct() -> &'static [u8] {
    PONG_BODY
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use std::sync::atomic::Ordering;

    #[tokio::test]
    async fn test_ping_handler_response_and_contract() {
        let state = crate::state::AppState::test_state();
        let response = ping_handler(axum::extract::State(state.clone())).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        assert_eq!(
            response.headers().get(header::CONTENT_LENGTH).unwrap(),
            PONG_BODY.len().to_string().as_str()
        );
        assert_eq!(response.headers().get("Connection").unwrap(), "keep-alive");

        let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body_bytes[..], PONG_BODY);

        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["message"], "pong");
    }

    #[tokio::test]
    async fn test_ac10_atomic_counter_correctness() {
        let state = crate::state::AppState::test_state();
        assert_eq!(state.counters.ping.load(Ordering::Relaxed), 0);

        let iterations = 100;
        for _ in 0..iterations {
            let _ = ping_handler(axum::extract::State(state.clone())).await;
        }

        assert_eq!(
            state.counters.ping.load(Ordering::Relaxed),
            iterations,
            "AC-10: Counter value must exactly equal actual request count"
        );
    }

    #[tokio::test]
    async fn test_ping_direct() {
        assert_eq!(ping_direct().await, PONG_BODY);
    }
}
