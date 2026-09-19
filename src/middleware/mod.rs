/// Middleware module — cross-cutting pipeline layers.
pub mod body_limit;
pub mod counter;
pub mod localhost_only;
pub mod request_id;
pub mod timeout;

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use axum::Router;
    use std::sync::atomic::Ordering;
    use tower::ServiceExt;

    use super::*;
    use crate::state::AppState;

    #[tokio::test]
    async fn test_counter_and_request_id_middleware() {
        let state = AppState::test_state();

        let app = Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                counter::counter_middleware,
            ))
            .layer(request_id::propagate_request_id_layer())
            .layer(request_id::set_request_id_layer())
            .with_state(state.clone());

        let req = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let res = app.oneshot(req).await.unwrap();

        assert_eq!(res.status(), StatusCode::OK);
        assert!(res.headers().contains_key("x-request-id"));
        assert_eq!(state.counters.total.load(Ordering::Relaxed), 1);
        assert_eq!(state.active_connections.load(Ordering::Relaxed), 0);
    }
}
