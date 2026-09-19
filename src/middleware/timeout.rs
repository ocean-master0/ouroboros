use axum::http::StatusCode;
/// Request timeout middleware layer.
///
/// Ensures requests running longer than configured seconds fail fast with HTTP 408 (FR14).
use std::time::Duration;
use tower_http::timeout::TimeoutLayer;

pub fn timeout_layer(secs: u64) -> TimeoutLayer {
    TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(secs))
}
