/// Background metrics reporter — runs as a 1-second tick task.
///
/// This is the single point where "counting" (hot path, lock-free) becomes
/// "reporting" (cold path, may allocate, may lock briefly).
///
/// Satisfies AC-09 (terminal log), feeds AC-06 (WebSocket broadcast) and
/// AC-11 (ring buffer) simultaneously.
///
/// Spawned once from `main.rs` via `tokio::spawn` and lives for the process
/// lifetime.
use crate::state::{AppState, MetricSnapshot};

/// Return current Unix timestamp in seconds.
fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Run the background metrics reporter loop.
///
/// This task swaps counters to 0 every second, computes RPS, pushes a
/// snapshot to the ring buffer, broadcasts to WebSocket subscribers, and
/// logs to the terminal.
pub async fn run_metrics_reporter(state: AppState) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
    let mut last_tick = std::time::Instant::now();

    loop {
        interval.tick().await;
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(last_tick).as_secs_f64();
        last_tick = now;

        let ping_count = state
            .counters
            .ping
            .swap(0, std::sync::atomic::Ordering::Relaxed);
        let error_count = state
            .counters
            .errors
            .swap(0, std::sync::atomic::Ordering::Relaxed);
        let total_count = state
            .counters
            .total
            .swap(0, std::sync::atomic::Ordering::Relaxed);
        let _echo_count = state
            .counters
            .echo
            .swap(0, std::sync::atomic::Ordering::Relaxed);

        let ping_rps = if elapsed > 0.0 {
            (ping_count as f64 / elapsed) as u64
        } else {
            0
        };
        let total_rps = if elapsed > 0.0 {
            (total_count as f64 / elapsed) as u64
        } else {
            0
        };
        // Use total_count for error rate to account for all endpoints
        let error_rate_pct = if total_count > 0 {
            (error_count as f64 / total_count as f64) * 100.0
        } else {
            0.0
        };

        let snapshot = MetricSnapshot {
            timestamp_secs: unix_now(),
            ping_rps,
            total_rps,
            error_rate_pct,
            active_conns: state
                .active_connections
                .load(std::sync::atomic::Ordering::Relaxed),
        };

        state.metrics_buffer.push(snapshot.clone());
        // Err if no subscribers — ignored, not a bug
        let _ = state.metrics_tx.send(snapshot.clone());

        tracing::info!(
            rps = ping_rps,
            total_rps = total_rps,
            target_reached = ping_rps >= 1_000_000,
            errors = error_count,
            "RPS window"
        );
    }
}
