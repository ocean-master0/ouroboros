/// Bench engine module — HTTP adapter for load generation.
///
/// Exposes operational endpoints under `/internal/bench/*`:
/// - `POST /internal/bench/start`
/// - `POST /internal/bench/stop`
/// - `GET /internal/bench/status`
pub mod engine;

use std::sync::atomic::Ordering;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;

use crate::bench::engine::{BenchStartRequest, BenchStatusResponse};
use crate::state::AppState;

/// Assembly of the internal bench router.
pub fn bench_router() -> Router<AppState> {
    Router::new()
        .route("/start", post(start_handler))
        .route("/stop", post(stop_handler))
        .route("/status", get(status_handler))
}

/// POST /internal/bench/start — start a load test run.
pub async fn start_handler(
    State(state): State<AppState>,
    Json(payload): Json<BenchStartRequest>,
) -> Response {
    // 1. Validate parameters per API_REFERENCE.md §8.1
    let max_concurrency = state.config.bench_max_concurrency.max(10_000);
    if payload.concurrency == 0 || payload.concurrency > max_concurrency {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "error": "concurrency must be between 1 and 10000",
                "code": "VALIDATION_ERROR"
            })),
        )
            .into_response();
    }

    if payload.duration_secs < 5 || payload.duration_secs > 3_600 {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "error": "duration_secs must be between 5 and 3600",
                "code": "VALIDATION_ERROR"
            })),
        )
            .into_response();
    }

    let target = payload.target_endpoint.trim();
    if !target.starts_with('/') || target.contains('@') || target.contains("://") || target.contains(' ') {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({
                "error": "target_endpoint must be a relative path starting with '/'",
                "code": "VALIDATION_ERROR"
            })),
        )
            .into_response();
    }

    // 2. Check if a benchmark is already active
    if state.bench_engine.running.load(Ordering::Relaxed) {
        return crate::error::AppError::BenchAlreadyRunning.into_response();
    }

    // 3. Build target URL for loopback mode
    let target_url = format!(
        "http://127.0.0.1:{}{}",
        state.config.port, payload.target_endpoint
    );

    let bench_id = uuid::Uuid::new_v4().to_string();

    // 4. Start load generation tasks
    engine::start_bench(
        state.bench_engine.clone(),
        payload.concurrency as usize,
        payload.duration_secs,
        payload.mode,
        target_url,
    )
    .await;

    (
        StatusCode::OK,
        Json(json!({
            "bench_id": bench_id,
            "started": true
        })),
    )
        .into_response()
}

/// POST /internal/bench/stop — cancel or stop an active bench run.
pub async fn stop_handler(State(state): State<AppState>) -> Response {
    let was_running = state.bench_engine.running.swap(false, Ordering::Relaxed);

    (
        StatusCode::OK,
        Json(json!({
            "stopped": was_running
        })),
    )
        .into_response()
}

/// GET /internal/bench/status — telemetry and progress of current or last bench run.
pub async fn status_handler(State(state): State<AppState>) -> Json<BenchStatusResponse> {
    let engine = &state.bench_engine;
    let running = engine.running.load(Ordering::Relaxed);
    let mode = engine.mode.read().clone();
    let concurrency = engine.concurrency.load(Ordering::Relaxed) as u32;

    let elapsed_secs = engine
        .started_at
        .read()
        .map(|t| t.elapsed().as_secs())
        .unwrap_or(0);

    let requests_sent = engine.requests_sent.load(Ordering::Relaxed);
    let requests_success = engine.requests_success.load(Ordering::Relaxed);
    let requests_failed = engine.requests_failed.load(Ordering::Relaxed);

    let calculated_rps = requests_sent.checked_div(elapsed_secs).unwrap_or(0);

    let peak = engine.peak_rps.load(Ordering::Relaxed);
    if calculated_rps > peak {
        engine.peak_rps.store(calculated_rps, Ordering::Relaxed);
    }
    let peak_rps = engine.peak_rps.load(Ordering::Relaxed);

    let current_rps = if running { calculated_rps } else { 0 };

    const TARGET_RPS: u64 = 1_000_000;
    let target_reached = current_rps >= TARGET_RPS || peak_rps >= TARGET_RPS;

    Json(BenchStatusResponse {
        running,
        mode,
        concurrency,
        elapsed_secs,
        requests_sent,
        requests_success,
        requests_failed,
        current_rps,
        peak_rps,
        p50_ms: 0.0,
        p95_ms: 0.0,
        p99_ms: 0.0,
        target_rps: TARGET_RPS,
        target_reached,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bench::engine::BenchMode;

    #[tokio::test]
    async fn test_bench_start_stop_status_cycle() {
        let state = AppState::test_state();

        // 1. Initial status: running is false
        let Json(initial_status) = status_handler(State(state.clone())).await;
        assert!(!initial_status.running);

        // 2. Start bench with direct mode
        let start_req = BenchStartRequest {
            concurrency: 4,
            duration_secs: 10,
            mode: BenchMode::Direct,
            target_endpoint: "/api/v1/ping".to_string(),
        };
        let response = start_handler(State(state.clone()), Json(start_req)).await;
        assert_eq!(response.status(), StatusCode::OK);

        // 3. Status should show running == true
        let Json(running_status) = status_handler(State(state.clone())).await;
        assert!(running_status.running);

        // 4. Stop bench
        let stop_res = stop_handler(State(state.clone())).await;
        assert_eq!(stop_res.status(), StatusCode::OK);

        // 5. Status should show running == false
        let Json(stopped_status) = status_handler(State(state.clone())).await;
        assert!(!stopped_status.running);
    }

    #[tokio::test]
    async fn test_bench_validation_errors() {
        let state = AppState::test_state();

        // Invalid concurrency = 0
        let req1 = BenchStartRequest {
            concurrency: 0,
            duration_secs: 10,
            mode: BenchMode::Direct,
            target_endpoint: "/api/v1/ping".to_string(),
        };
        let res1 = start_handler(State(state.clone()), Json(req1)).await;
        assert_eq!(res1.status(), StatusCode::UNPROCESSABLE_ENTITY);

        // Invalid duration < 5
        let req2 = BenchStartRequest {
            concurrency: 10,
            duration_secs: 2,
            mode: BenchMode::Direct,
            target_endpoint: "/api/v1/ping".to_string(),
        };
        let res2 = start_handler(State(state.clone()), Json(req2)).await;
        assert_eq!(res2.status(), StatusCode::UNPROCESSABLE_ENTITY);

        // Invalid target endpoint (SSRF attempt)
        let req3 = BenchStartRequest {
            concurrency: 10,
            duration_secs: 10,
            mode: BenchMode::Loopback,
            target_endpoint: "@evil.com".to_string(),
        };
        let res3 = start_handler(State(state), Json(req3)).await;
        assert_eq!(res3.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
