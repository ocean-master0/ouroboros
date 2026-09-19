/// Built-in bench engine — self-contained load generator.
///
/// This module is architecturally independent of Axum. `BenchEngine` and
/// `start_bench()` know nothing about HTTP request/response types; they only
/// touch atomics and, in Loopback mode, use a plain `reqwest::Client`.
///
/// The engine is Axum-agnostic to stay unit-testable without spinning up an
/// HTTP server (architecture_doc.md §5.3).
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

/// Shared bench engine state, accessed atomically by worker tasks.
pub struct BenchEngine {
    /// Whether a benchmark is currently running.
    pub running: AtomicBool,
    /// Total requests sent (success + failed).
    pub requests_sent: AtomicU64,
    /// Successful requests.
    pub requests_success: AtomicU64,
    /// Failed requests.
    pub requests_failed: AtomicU64,
    /// Peak RPS observed during this run.
    pub peak_rps: AtomicU64,
    /// Mode of the current/last run.
    pub mode: parking_lot::RwLock<BenchMode>,
    /// Configured concurrency of the current/last run.
    pub concurrency: AtomicU64,
    /// Start time of the current run (if any).
    pub started_at: parking_lot::RwLock<Option<std::time::Instant>>,
    /// Reference to global counters to synchronize reporter and dashboard in direct mode.
    pub counters: parking_lot::RwLock<Option<Arc<crate::state::Counters>>>,
}

impl BenchEngine {
    /// Create a new idle bench engine.
    pub fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
            requests_sent: AtomicU64::new(0),
            requests_success: AtomicU64::new(0),
            requests_failed: AtomicU64::new(0),
            peak_rps: AtomicU64::new(0),
            mode: parking_lot::RwLock::new(BenchMode::Loopback),
            concurrency: AtomicU64::new(0),
            started_at: parking_lot::RwLock::new(None),
            counters: parking_lot::RwLock::new(None),
        }
    }

    /// Attach application counters for unified metrics reporting.
    pub fn set_counters(&self, counters: Arc<crate::state::Counters>) {
        *self.counters.write() = Some(counters);
    }

    /// Reset all counters for a new run.
    pub fn reset(&self) {
        self.requests_sent.store(0, Ordering::Relaxed);
        self.requests_success.store(0, Ordering::Relaxed);
        self.requests_failed.store(0, Ordering::Relaxed);
        self.peak_rps.store(0, Ordering::Relaxed);
    }
}

impl Default for BenchEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Bench mode — determines how the load generator calls the target.
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum BenchMode {
    /// Real HTTP requests over 127.0.0.1 — exercises the full network stack.
    #[default]
    Loopback,
    /// Direct in-process function call — no network overhead.
    Direct,
}

/// Request body for `POST /internal/bench/start`.
#[derive(serde::Deserialize)]
pub struct BenchStartRequest {
    /// Number of concurrent worker tasks (1–10,000, default 100).
    #[serde(default = "default_concurrency")]
    pub concurrency: u32,
    /// Duration in seconds (5–3600, default 30).
    #[serde(default = "default_duration")]
    pub duration_secs: u64,
    /// Benchmark mode (default: loopback).
    #[serde(default)]
    pub mode: BenchMode,
    /// Target endpoint path (default: "/api/v1/ping").
    #[serde(default = "default_target")]
    pub target_endpoint: String,
}

/// Response body for `GET /internal/bench/status`.
#[derive(serde::Serialize)]
pub struct BenchStatusResponse {
    pub running: bool,
    pub mode: BenchMode,
    pub concurrency: u32,
    pub elapsed_secs: u64,
    pub requests_sent: u64,
    pub requests_success: u64,
    pub requests_failed: u64,
    pub current_rps: u64,
    pub peak_rps: u64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub target_rps: u64,
    pub target_reached: bool,
}

/// Start a benchmark run with the given parameters.
///
/// Spawns `concurrency` worker tasks and one auto-stop timer task.
pub async fn start_bench(
    engine: Arc<BenchEngine>,
    concurrency: usize,
    duration_secs: u64,
    mode: BenchMode,
    target_url: String,
) {
    engine.reset();
    engine.running.store(true, Ordering::Relaxed);
    engine
        .concurrency
        .store(concurrency as u64, Ordering::Relaxed);
    *engine.mode.write() = mode.clone();
    *engine.started_at.write() = Some(std::time::Instant::now());

    for _ in 0..concurrency {
        let engine = engine.clone();
        let url = target_url.clone();
        let mode = mode.clone();

        tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .pool_max_idle_per_host(100)
                .tcp_nodelay(true)
                .build()
                .unwrap();

            while engine.running.load(Ordering::Relaxed) {
                match mode {
                    BenchMode::Loopback => match client.get(&url).send().await {
                        Ok(r) if r.status().is_success() => {
                            engine.requests_success.fetch_add(1, Ordering::Relaxed);
                        }
                        _ => {
                            engine.requests_failed.fetch_add(1, Ordering::Relaxed);
                        }
                    },
                    BenchMode::Direct => {
                        let _ = crate::api::v1::ping::ping_direct().await;
                        engine.requests_success.fetch_add(1, Ordering::Relaxed);
                        if let Some(ref c) = *engine.counters.read() {
                            c.ping.fetch_add(1, Ordering::Relaxed);
                            c.total.fetch_add(1, Ordering::Relaxed);
                        }
                        tokio::task::yield_now().await;
                    }
                }
                engine.requests_sent.fetch_add(1, Ordering::Relaxed);
            }
        });
    }

    // Auto-stop timer — satisfies US-008 (auto-stop after duration)
    let stop_engine = engine.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(duration_secs)).await;
        stop_engine.running.store(false, Ordering::Relaxed);
    });
}

fn default_concurrency() -> u32 {
    100
}
fn default_duration() -> u64 {
    30
}
fn default_target() -> String {
    "/api/v1/ping".to_string()
}
