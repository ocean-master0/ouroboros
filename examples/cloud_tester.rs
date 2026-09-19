//! Project Ouroboros - Rust Cloud API Client & Load Tester
//! Tests connection from your local laptop to the live Render deployment.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const CLOUD_URL: &str = "https://ouroboros-bxa2.onrender.com/api/v1/users?name=Abhishek";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("============================================================");
    println!("🚀 Project Ouroboros - Rust Cloud API Tester");
    println!("Target URL: {}", CLOUD_URL);
    println!("============================================================\n");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(50)
        .tcp_nodelay(true)
        .build()?;

    // --- STEP 1: Single Diagnostic Probe ---
    println!("[1/2] Sending single diagnostic probe from laptop to Render...");
    let probe_start = Instant::now();
    let probe_resp = client.get(CLOUD_URL).send().await;

    match probe_resp {
        Ok(resp) => {
            let latency = probe_start.elapsed().as_millis();
            let status = resp.status();
            println!("  ✅ Connected Successfully!");
            println!("  ⏱️  Round-trip Latency : {} ms", latency);
            println!("  📡 HTTP Status Code    : {}", status);

            if let Ok(body) = resp.text().await {
                println!("  📄 Response Body Preview:\n{}\n", body);
            }
        }
        Err(e) => {
            eprintln!("  ❌ Connection Failed: {}", e);
            return Ok(());
        }
    }

    // --- STEP 2: Concurrency & Throughput Test ---
    let concurrency = 20; // 20 parallel async worker tasks (matched to DB pool size)
    let duration_secs = 5; // 5 seconds load test

    println!(
        "[2/2] Running Multi-threaded Async Benchmark ({} workers, {}s)...",
        concurrency, duration_secs
    );

    let success_count = Arc::new(AtomicU64::new(0));
    let error_count = Arc::new(AtomicU64::new(0));
    let is_running = Arc::new(AtomicBool::new(true));

    let mut worker_handles = Vec::with_capacity(concurrency);
    let bench_start = Instant::now();

    for _ in 0..concurrency {
        let client = client.clone();
        let success = success_count.clone();
        let error = error_count.clone();
        let running = is_running.clone();

        worker_handles.push(tokio::spawn(async move {
            while running.load(Ordering::Relaxed) {
                match client.get(CLOUD_URL).send().await {
                    Ok(res) if res.status().is_success() => {
                        success.fetch_add(1, Ordering::Relaxed);
                    }
                    _ => {
                        error.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }));
    }

    // Wait for test duration
    tokio::time::sleep(Duration::from_secs(duration_secs)).await;
    is_running.store(false, Ordering::Relaxed);

    // Join all workers
    for handle in worker_handles {
        let _ = handle.await;
    }

    let total_elapsed = bench_start.elapsed().as_secs_f64();
    let total_success = success_count.load(Ordering::Relaxed);
    let total_errors = error_count.load(Ordering::Relaxed);
    let total_requests = total_success + total_errors;
    let rps = total_requests as f64 / total_elapsed;

    println!("============================================================");
    println!("📊 BENCHMARK RESULTS (Laptop -> Render Cloud):");
    println!("============================================================");
    println!("  • Elapsed Time        : {:.2} seconds", total_elapsed);
    println!("  • Total Requests Sent : {}", total_requests);
    println!("  • Successful Requests : {}", total_success);
    println!("  • Failed / Dropped    : {}", total_errors);
    println!("  • Measured Throughput : {:.2} Requests/sec (RPS)", rps);
    println!("============================================================");

    Ok(())
}
