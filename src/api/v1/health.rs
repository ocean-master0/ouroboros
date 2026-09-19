/// GET /health — liveness check.
///
/// No DB dependency, no hot-path optimizations required.
/// Must respond within 5ms (FR01).
pub async fn health_handler(
    axum::extract::State(state): axum::extract::State<crate::state::AppState>,
) -> axum::Json<serde_json::Value> {
    let uptime = state.start_time.elapsed().as_secs();
    axum::Json(serde_json::json!({
        "status": "ok",
        "uptime_secs": uptime,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_handler_ac03() {
        let state = crate::state::AppState::test_state();
        let axum::Json(body) = health_handler(axum::extract::State(state)).await;

        assert_eq!(body["status"], "ok");
        assert!(body["uptime_secs"].is_u64());
    }
}
