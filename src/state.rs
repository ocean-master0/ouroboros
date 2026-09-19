use std::sync::atomic::{AtomicI64, AtomicU64};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::bench::engine::BenchEngine;
use crate::config::Config;
use crate::metrics::ring_buffer::RingBuffer;

/// Central shared application state — the architectural hub every component
/// connects through.
///
/// `AppState` is `Clone` and cheap to clone (everything inside is `Arc` or a
/// pool handle). Axum clones it per-request via the `State` extractor.
///
/// Constructed once in `main.rs`, after config load and before router assembly.
#[derive(Clone)]
pub struct AppState {
    /// PostgreSQL connection pool.
    pub db: sqlx::PgPool,
    /// Immutable application configuration (process lifetime).
    pub config: Arc<Config>,
    /// Atomic request counters (hot-path write target).
    pub counters: Arc<Counters>,
    /// Ring buffer holding the last 60 seconds of metric snapshots.
    pub metrics_buffer: Arc<RingBuffer>,
    /// Broadcast channel for pushing metric snapshots to WebSocket subscribers.
    pub metrics_tx: broadcast::Sender<MetricSnapshot>,
    /// Built-in bench engine shared state.
    pub bench_engine: Arc<BenchEngine>,
    /// Active TCP connection count (incremented/decremented by middleware).
    pub active_connections: Arc<AtomicI64>,
    /// Server start time for uptime calculation.
    pub start_time: std::time::Instant,
}

impl AppState {
    /// Constructs a mock AppState for testing handlers, atomic counters, and integration tests.
    pub fn test_state() -> Self {
        let config = Config {
            database_url: "postgres://mock@localhost/mock".to_string(),
            port: 8080,
            max_db_connections: 10,
            bench_max_concurrency: 100,
            body_limit_bytes: 1024 * 1024,
            request_timeout_secs: 5,
            log_level: "info".to_string(),
        };
        let db = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy(&config.database_url)
            .expect("mock pool creation failed");
        let (metrics_tx, _) = broadcast::channel(16);
        let counters = Arc::new(Counters::new());
        let bench_engine = Arc::new(BenchEngine::new());
        bench_engine.set_counters(counters.clone());
        Self {
            db,
            config: Arc::new(config),
            counters,
            metrics_buffer: Arc::new(RingBuffer::new()),
            metrics_tx,
            bench_engine,
            active_connections: Arc::new(AtomicI64::new(0)),
            start_time: std::time::Instant::now(),
        }
    }
}

/// Atomic request counters — the write target for the hot path.
///
/// All increments use `Ordering::Relaxed` — correct because these counters are
/// read only by the 1-second reporter tick, which does not need happens-before
/// ordering relative to individual increments.
pub struct Counters {
    /// Ping endpoint request count (swapped to 0 every 1s by the reporter).
    pub ping: AtomicU64,
    /// Echo endpoint request count.
    pub echo: AtomicU64,
    /// Error count across all endpoints.
    pub errors: AtomicU64,
    /// Total request count across all endpoints.
    pub total: AtomicU64,
}

impl Counters {
    /// Create a new zeroed `Counters` instance.
    pub fn new() -> Self {
        Self {
            ping: AtomicU64::new(0),
            echo: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            total: AtomicU64::new(0),
        }
    }
}

impl Default for Counters {
    fn default() -> Self {
        Self::new()
    }
}

/// A single point-in-time metrics snapshot stored in the ring buffer and
/// broadcast to WebSocket clients.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MetricSnapshot {
    /// Unix timestamp of this snapshot.
    pub timestamp_secs: u64,
    /// Ping endpoint requests per second in the last window.
    pub ping_rps: u64,
    /// Total requests per second across all endpoints.
    pub total_rps: u64,
    /// Error rate as a percentage (0.0–100.0).
    pub error_rate_pct: f64,
    /// Currently active TCP connections.
    pub active_conns: i64,
}
