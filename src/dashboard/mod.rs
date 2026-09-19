/// Dashboard module — presentation layer for the browser UI.
///
/// Serves embedded static HTML/JS assets (compiled into binary, DR01, C10),
/// and manages WebSocket subscribers pushing 1-second metric snapshots.
use axum::http::header;
use axum::response::{Html, Response};

const DASHBOARD_HTML: &str = include_str!("assets/index.html");
const CHART_JS: &str = include_str!("assets/chart.min.js");
const DASHBOARD_JS: &str = include_str!("assets/dashboard.js");

/// GET /dashboard — serve the embedded HTML dashboard.
pub async fn serve_dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

/// GET /dashboard/assets/chart.min.js — serve vendored Chart.js.
pub async fn serve_chart_js() -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/javascript")
        .body(axum::body::Body::from(CHART_JS))
        .unwrap()
}

/// GET /dashboard/assets/dashboard.js — serve dashboard UI logic.
pub async fn serve_dashboard_js() -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/javascript")
        .body(axum::body::Body::from(DASHBOARD_JS))
        .unwrap()
}

/// GET /ws/metrics — WebSocket live metrics streaming endpoint.
pub async fn ws_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<crate::state::AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle a connected WebSocket client session.
async fn handle_socket(mut socket: axum::extract::ws::WebSocket, state: crate::state::AppState) {
    let mut rx = state.metrics_tx.subscribe();
    while let Ok(snapshot) = rx.recv().await {
        let payload = match serde_json::to_string(&snapshot) {
            Ok(p) => p,
            Err(_) => continue,
        };
        if socket
            .send(axum::extract::ws::Message::Text(payload.into()))
            .await
            .is_err()
        {
            break; // client disconnected — task terminates cleanly, no resource leak
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn test_serve_dashboard_ac05() {
        let Html(body) = serve_dashboard().await;
        assert!(body.contains("Project Ouroboros"));
        assert!(body.contains("id=\"rps\""));
        assert!(body.contains("id=\"target\""));
        assert!(body.contains("id=\"rpsChart\""));
    }

    #[tokio::test]
    async fn test_serve_assets() {
        let chart_res = serve_chart_js().await;
        assert_eq!(
            chart_res.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/javascript"
        );
        let chart_bytes = to_bytes(chart_res.into_body(), usize::MAX).await.unwrap();
        assert!(!chart_bytes.is_empty());

        let dash_res = serve_dashboard_js().await;
        assert_eq!(
            dash_res.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/javascript"
        );
        let dash_bytes = to_bytes(dash_res.into_body(), usize::MAX).await.unwrap();
        assert!(dash_bytes.starts_with(b"// Project Ouroboros"));
    }
}
