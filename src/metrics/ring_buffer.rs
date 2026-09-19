/// Ring buffer for time-series metric snapshots.
///
/// Fixed-size circular buffer: 60 slots (60 seconds of history).
/// Write: O(1), lock-protected head advancement.
/// Read: O(n) for last N seconds.
///
/// Only the single `metrics_reporter` task ever calls `push`; this is a
/// structural invariant, not just a convention.
use parking_lot::RwLock;

use crate::state::MetricSnapshot;

/// Number of slots in the ring buffer — 60 seconds of history (AC-11).
pub const RING_SIZE: usize = 60;

/// Fixed-size circular buffer for `MetricSnapshot` values.
pub struct RingBuffer {
    slots: RwLock<[MetricSnapshot; RING_SIZE]>,
    head: std::sync::atomic::AtomicUsize,
}

impl RingBuffer {
    /// Create a new ring buffer with all slots zeroed.
    pub fn new() -> Self {
        Self {
            slots: RwLock::new(std::array::from_fn(|_| MetricSnapshot::default())),
            head: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Push a new snapshot into the buffer, overwriting the oldest slot.
    pub fn push(&self, snapshot: MetricSnapshot) {
        let idx = self.head.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % RING_SIZE;
        self.slots.write()[idx] = snapshot;
    }

    /// Return the last `n` snapshots in chronological order (oldest first).
    pub fn last_n(&self, n: usize) -> Vec<MetricSnapshot> {
        let head = self.head.load(std::sync::atomic::Ordering::Relaxed);
        let slots = self.slots.read();
        let count = n.min(RING_SIZE).min(head); // Don't return unfilled slots
        (0..count)
            .map(|i| slots[(head + RING_SIZE - count + i) % RING_SIZE].clone())
            .collect()
    }
}

impl Default for RingBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_read_basic() {
        let buf = RingBuffer::new();
        for i in 0..5 {
            buf.push(MetricSnapshot {
                timestamp_secs: i,
                ping_rps: i * 100,
                ..Default::default()
            });
        }
        let result = buf.last_n(5);
        assert_eq!(result.len(), 5);
        assert_eq!(result[0].timestamp_secs, 0);
        assert_eq!(result[4].timestamp_secs, 4);
    }

    #[test]
    fn wraparound() {
        let buf = RingBuffer::new();
        // Push 65 items — wraps around the 60-slot buffer
        for i in 0..65u64 {
            buf.push(MetricSnapshot {
                timestamp_secs: i,
                ping_rps: i,
                ..Default::default()
            });
        }
        let result = buf.last_n(60);
        assert_eq!(result.len(), 60);
        // Oldest should be 5 (items 0-4 were overwritten)
        assert_eq!(result[0].timestamp_secs, 5);
        assert_eq!(result[59].timestamp_secs, 64);
    }

    #[test]
    fn last_n_capped_at_written() {
        let buf = RingBuffer::new();
        buf.push(MetricSnapshot {
            timestamp_secs: 1,
            ..Default::default()
        });
        // Only 1 item written — requesting 60 should return 1
        let result = buf.last_n(60);
        assert_eq!(result.len(), 1);
    }
}
