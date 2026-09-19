//! Project Ouroboros — Entry Point
//!
//! A self-contained, high-performance Rust HTTP API with built-in monitoring,
//! load testing, and real-time dashboard. Single binary, zero external
//! runtime dependencies beyond PostgreSQL.
//!
//! This file is the composition root:
//! 1. Loads configuration
//! 2. Initializes tracing
//! 3. Creates the database connection pool
//! 4. Constructs AppState (the central shared-state object)
//! 5. Assembles the Axum router with middleware
//! 6. Spawns background tasks (metrics reporter)
//! 7. Binds the TCP listener and serves with graceful shutdown

use std::sync::atomic::AtomicI64;
use std::sync::Arc;

use tokio::sync::broadcast;
use tracing_subscriber::EnvFilter;

use ouroboros_api::bench::engine::BenchEngine;
use ouroboros_api::build_app;
use ouroboros_api::config::Config;
use ouroboros_api::db;
use ouroboros_api::metrics;
use ouroboros_api::metrics::ring_buffer::RingBuffer;
use ouroboros_api::state::{AppState, Counters, MetricSnapshot};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Load configuration — fail fast if DATABASE_URL is missing
    let config = Config::load()?;

    // 2. Initialize tracing
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    tracing::info!("Starting Ouroboros API v{}", env!("CARGO_PKG_VERSION"));

    // 3. Create database connection pool
    let db = db::pool::create_pool(&config).await?;
    tracing::info!("Database connection pool established");

    db::run_migrations(&db).await?;
    tracing::info!("Database migrations applied successfully");

    // 4. Construct AppState — the central shared-state object
    let (metrics_tx, _) = broadcast::channel::<MetricSnapshot>(128);
    let counters = Arc::new(Counters::new());
    let bench_engine = Arc::new(BenchEngine::new());
    bench_engine.set_counters(counters.clone());

    let state = AppState {
        db,
        config: Arc::new(config.clone()),
        counters,
        metrics_buffer: Arc::new(RingBuffer::new()),
        metrics_tx,
        bench_engine,
        active_connections: Arc::new(AtomicI64::new(0)),
        start_time: std::time::Instant::now(),
    };

    // 5. Assemble Axum router via shared build_app
    let app = build_app(state.clone());

    // 6. Spawn background tasks
    tokio::spawn(metrics::reporter::run_metrics_reporter(state.clone()));
    tracing::info!("Metrics reporter started (1s tick)");

    // 7. Bind listener and serve with graceful shutdown
    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server shut down cleanly");
    Ok(())
}

/// Wait for a shutdown signal (Ctrl+C on Windows, SIGTERM or Ctrl+C on Unix).
async fn shutdown_signal() {
    #[cfg(windows)]
    {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("Ctrl+C received, shutting down...");
    }

    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        tokio::select! {
            _ = sigterm.recv() => tracing::info!("SIGTERM received"),
            _ = tokio::signal::ctrl_c() => tracing::info!("Ctrl+C received"),
        }
    }
}
