/// Request counter middleware.
///
/// Increments `counters.total` atomically per request using `Ordering::Relaxed`.
/// Also maintains `active_connections` gauge during in-flight request processing.
use std::sync::atomic::Ordering;

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;

use crate::state::AppState;

pub async fn counter_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    state.counters.total.fetch_add(1, Ordering::Relaxed);
    state.active_connections.fetch_add(1, Ordering::Relaxed);
    let res = next.run(req).await;
    state.active_connections.fetch_sub(1, Ordering::Relaxed);
    res
}
