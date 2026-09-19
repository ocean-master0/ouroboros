/// API v1 module — presentation layer for REST endpoints.
///
/// Each file is a thin adapter: extract request data → call into domain/data
/// layers → map result to `IntoResponse`.
pub mod health;
pub mod metrics_api;
pub mod ping;
pub mod users;
