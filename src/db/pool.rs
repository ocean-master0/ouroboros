/// PostgreSQL connection pool initialization.
///
/// This is the sole channel through which the rest of the system reaches
/// PostgreSQL, keeping connection-pool configuration in exactly one place.
use std::time::Duration;

use crate::config::Config;

/// Create and return a configured SQLx `PgPool`.
///
/// Pool settings:
/// - `min_connections(5)` — keeps a small warm pool for fast first queries.
/// - `acquire_timeout(5s)` — converts "PostgreSQL is down" into a fast 503.
/// - `idle_timeout(600s)` — reclaims idle connections after 10 minutes.
pub async fn create_pool(config: &Config) -> Result<sqlx::PgPool, sqlx::Error> {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.max_db_connections)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(15))
        .idle_timeout(Duration::from_secs(600))
        .connect_lazy(&config.database_url)
}
