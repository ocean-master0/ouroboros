/// GET /api/v1/metrics — JSON snapshot of ring buffer history.
///
/// Returns the last N seconds of metric snapshots (default 60, max 60).
/// The whole handler is one line beyond extraction; there is deliberately
/// no separate "metrics service" layer for MVP.
pub async fn metrics_handler(
    axum::extract::State(state): axum::extract::State<crate::state::AppState>,
    axum::extract::Query(params): axum::extract::Query<MetricsQuery>,
) -> axum::Json<Vec<crate::state::MetricSnapshot>> {
    let n = params.n.unwrap_or(60).min(60);
    axum::Json(state.metrics_buffer.last_n(n))
}

#[derive(Default, serde::Deserialize)]
pub struct MetricsQuery {
    pub n: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::MetricSnapshot;

    #[tokio::test]
    async fn test_metrics_handler_empty() {
        let state = crate::state::AppState::test_state();
        let axum::Json(res) = metrics_handler(
            axum::extract::State(state),
            axum::extract::Query(MetricsQuery { n: None }),
        )
        .await;

        assert_eq!(res.len(), 0);
    }

    #[tokio::test]
    async fn test_metrics_handler_ac11_ring_buffer() {
        let state = crate::state::AppState::test_state();

        // Simulate 75 snapshots (past 60 slots capacity)
        for i in 1..=75 {
            state.metrics_buffer.push(MetricSnapshot {
                timestamp_secs: 1000 + i,
                ping_rps: i * 100,
                total_rps: i * 100,
                error_rate_pct: 0.0,
                active_conns: 0,
            });
        }

        // Default query: should return last 60 snapshots (AC-11)
        let axum::Json(res) = metrics_handler(
            axum::extract::State(state.clone()),
            axum::extract::Query(MetricsQuery { n: None }),
        )
        .await;
        assert_eq!(res.len(), 60);
        assert_eq!(res[0].timestamp_secs, 1016);
        assert_eq!(res[59].timestamp_secs, 1075);

        // Custom n=10: should return last 10 snapshots
        let axum::Json(res_10) = metrics_handler(
            axum::extract::State(state.clone()),
            axum::extract::Query(MetricsQuery { n: Some(10) }),
        )
        .await;
        assert_eq!(res_10.len(), 10);
        assert_eq!(res_10[0].timestamp_secs, 1066);
        assert_eq!(res_10[9].timestamp_secs, 1075);

        // Capped at 60: query with n=100 returns 60
        let axum::Json(res_capped) = metrics_handler(
            axum::extract::State(state),
            axum::extract::Query(MetricsQuery { n: Some(100) }),
        )
        .await;
        assert_eq!(res_capped.len(), 60);
    }
}
