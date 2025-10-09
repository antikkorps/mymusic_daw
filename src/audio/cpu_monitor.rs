// CPU Monitor - Audio callback performance tracking
//
// This module monitors the CPU load of the audio callback to prevent dropouts.
// Uses atomics for thread-safe metric sharing between audio and UI threads.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// CPU monitor for audio callback
///
/// Measures callback execution time and calculates CPU percentage
/// relative to available time.
///
/// Thread-safe: Uses atomics to share metrics between audio and UI threads.
#[derive(Clone)]
pub struct CpuMonitor {
    // Accumulated statistics
    total_callback_time_ns: Arc<AtomicU64>,
    total_available_time_ns: Arc<AtomicU64>,
    sample_count: Arc<AtomicU64>,

    // Configuration
    sample_rate: f32,
    buffer_size: usize,

    // Measurement frequency (1 out of N callbacks)
    measure_every_n: u32,
    current_count: Arc<AtomicU32>,
}

impl CpuMonitor {
    /// Create a new CPU monitor
    ///
    /// # Arguments
    /// * `sample_rate` - Sample rate in Hz (e.g., 44100.0)
    /// * `buffer_size` - Audio buffer size in samples
    /// * `measure_every_n` - Measure 1 out of N callbacks (e.g., 10 = measure 10% of callbacks)
    pub fn new(sample_rate: f32, buffer_size: usize, measure_every_n: u32) -> Self {
        Self {
            total_callback_time_ns: Arc::new(AtomicU64::new(0)),
            total_available_time_ns: Arc::new(AtomicU64::new(0)),
            sample_count: Arc::new(AtomicU64::new(0)),
            sample_rate,
            buffer_size,
            measure_every_n: measure_every_n.max(1),
            current_count: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Start a measurement (call at callback start)
    ///
    /// Returns `Some(Instant)` if this callback should be measured,
    /// `None` otherwise (to avoid overhead)
    #[inline]
    pub fn start_measure(&self) -> Option<Instant> {
        let count = self.current_count.fetch_add(1, Ordering::Relaxed);

        if count % self.measure_every_n == 0 {
            Some(Instant::now())
        } else {
            None
        }
    }

    /// End a measurement (call at callback end)
    ///
    /// # Arguments
    /// * `start_time` - The timestamp returned by `start_measure()`
    #[inline]
    pub fn end_measure(&self, start_time: Option<Instant>) {
        if let Some(start) = start_time {
            let elapsed_ns = start.elapsed().as_nanos() as u64;

            // Available time for this buffer (in nanoseconds)
            let available_ns = ((self.buffer_size as f64 / self.sample_rate as f64) * 1_000_000_000.0) as u64;

            // Accumulate statistics
            self.total_callback_time_ns.fetch_add(elapsed_ns, Ordering::Relaxed);
            self.total_available_time_ns.fetch_add(available_ns, Ordering::Relaxed);
            self.sample_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get current CPU percentage
    ///
    /// Returns an f32 between 0.0 and 100.0+ (can exceed 100% if overloaded)
    pub fn get_cpu_percentage(&self) -> f32 {
        let total_callback = self.total_callback_time_ns.load(Ordering::Relaxed);
        let total_available = self.total_available_time_ns.load(Ordering::Relaxed);

        if total_available == 0 {
            return 0.0;
        }

        (total_callback as f64 / total_available as f64 * 100.0) as f32
    }

    /// Get the number of measured samples
    pub fn get_sample_count(&self) -> u64 {
        self.sample_count.load(Ordering::Relaxed)
    }

    /// Reset statistics (start from zero)
    pub fn reset(&self) {
        self.total_callback_time_ns.store(0, Ordering::Relaxed);
        self.total_available_time_ns.store(0, Ordering::Relaxed);
        self.sample_count.store(0, Ordering::Relaxed);
        self.current_count.store(0, Ordering::Relaxed);
    }

    /// Get load level (for UI display)
    ///
    /// Returns:
    /// - `CpuLoad::Low` if < 50%
    /// - `CpuLoad::Medium` if 50-75%
    /// - `CpuLoad::High` if > 75%
    pub fn get_load_level(&self) -> CpuLoad {
        let cpu = self.get_cpu_percentage();

        if cpu < 50.0 {
            CpuLoad::Low
        } else if cpu < 75.0 {
            CpuLoad::Medium
        } else {
            CpuLoad::High
        }
    }
}

/// CPU load level (for UI display)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CpuLoad {
    Low,    // < 50% (green)
    Medium, // 50-75% (orange)
    High,   // > 75% (red)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_cpu_monitor_creation() {
        let monitor = CpuMonitor::new(44100.0, 512, 10);
        assert_eq!(monitor.get_cpu_percentage(), 0.0);
        assert_eq!(monitor.get_sample_count(), 0);
    }

    #[test]
    fn test_measure_sampling() {
        let monitor = CpuMonitor::new(44100.0, 512, 10);

        let mut measured = 0;
        let mut not_measured = 0;

        for _ in 0..100 {
            if monitor.start_measure().is_some() {
                measured += 1;
            } else {
                not_measured += 1;
            }
        }

        // About 10% of callbacks should be measured
        assert!(measured >= 8 && measured <= 12);
        assert!(not_measured >= 88 && not_measured <= 92);
    }

    #[test]
    fn test_cpu_percentage_calculation() {
        let monitor = CpuMonitor::new(44100.0, 512, 1); // Measure all callbacks

        // Simulate some measurements
        for _ in 0..10 {
            let start = monitor.start_measure();
            thread::sleep(Duration::from_micros(100)); // Simulate work
            monitor.end_measure(start);
        }

        let cpu = monitor.get_cpu_percentage();
        assert!(cpu > 0.0);
        assert!(cpu < 100.0); // Should not saturate with this light test
    }

    #[test]
    fn test_reset() {
        let monitor = CpuMonitor::new(44100.0, 512, 1);

        // Accumulate some measurements
        for _ in 0..5 {
            let start = monitor.start_measure();
            thread::sleep(Duration::from_micros(50));
            monitor.end_measure(start);
        }

        assert!(monitor.get_cpu_percentage() > 0.0);
        assert!(monitor.get_sample_count() > 0);

        // Reset
        monitor.reset();
        assert_eq!(monitor.get_cpu_percentage(), 0.0);
        assert_eq!(monitor.get_sample_count(), 0);
    }

    #[test]
    fn test_load_levels() {
        let monitor = CpuMonitor::new(44100.0, 512, 1);

        // Initial level (Low)
        assert_eq!(monitor.get_load_level(), CpuLoad::Low);
    }
}
