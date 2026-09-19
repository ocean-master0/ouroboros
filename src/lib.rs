//! Project Ouroboros Library
//!
//! Exposes the application modules, shared state, configuration, and router
//! assembly function `build_app` for both the server binary and integration tests.

pub mod api;
pub mod bench;
pub mod config;
pub mod dashboard;
pub mod db;
pub mod error;
pub mod metrics;
pub mod middleware;
pub mod state;

pub use config::Config;
pub use state::AppState;

use axum::routing::get;
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

/// Builds the Axum application router with all middleware and routes attached.
pub fn build_app(state: AppState) -> Router {
    // Hot-path sub-router: /api/v1/ping gets a minimal middleware stack
    // (tracing layer deliberately excluded — architecture_doc.md §7.2)
    let hot_path_router = Router::new().route("/api/v1/ping", get(api::v1::ping::ping_handler));

    // Standard sub-router: all other routes with full middleware
    let standard_router = Router::new()
        .route("/health", get(api::v1::health::health_handler))
        .route(
            "/api/v1/metrics",
            get(api::v1::metrics_api::metrics_handler),
        )
        .route("/dashboard", get(dashboard::serve_dashboard))
        .route(
            "/dashboard/assets/chart.min.js",
            get(dashboard::serve_chart_js),
        )
        .route(
            "/dashboard/assets/dashboard.js",
            get(dashboard::serve_dashboard_js),
        )
        .route("/ws/metrics", get(dashboard::ws_handler))
        .nest(
            "/internal/bench",
            bench::bench_router().layer(axum::middleware::from_fn(
                middleware::localhost_only::localhost_only_middleware,
            )),
        )
        .nest("/api/v1/users", api::v1::users::users_router())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .layer(middleware::timeout::timeout_layer(
            state.config.request_timeout_secs,
        ));

    Router::new()
        .merge(standard_router)
        .merge(hot_path_router)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::counter::counter_middleware,
        ))
        .layer(middleware::body_limit::body_limit_layer(
            state.config.body_limit_bytes,
        ))
        .layer(middleware::request_id::propagate_request_id_layer())
        .layer(middleware::request_id::set_request_id_layer())
        .with_state(state)
}
