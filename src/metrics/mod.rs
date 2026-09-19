/// Metrics module — domain layer for observability.
///
/// Three files with strictly separated responsibilities:
/// - `counters.rs` — write target for the hot path (reactive to requests)
/// - `ring_buffer.rs` — generic storage data structure
/// - `reporter.rs` — the only component bridging counters and ring buffer
pub mod counters;
pub mod reporter;
pub mod ring_buffer;
