//! Performance profiling utilities for the audio engine
//! 
//! This module provides tools to profile and analyze the performance
//! of the audio callback and related DSP operations.

use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::{atomic::{AtomicU64, Ordering}, Mutex};

/// Global profiler instance with atomic operations for thread safety
pub struct AudioProfiler {
    /// Total time spent in audio callback (nanoseconds)
    pub total_callback_time: AtomicU64,
    /// Number of callback executions
    pub callback_count: AtomicU64,
    /// Maximum callback time observed (nanoseconds)
    pub max_callback_time: AtomicU64,
    /// Minimum callback time observed (nanoseconds)
    pub min_callback_time: AtomicU64,
    /// Operation times (using Mutex for thread safety)
    operation_times: Mutex<HashMap<String, AtomicU64>>,
    /// Operation counts (using Mutex for thread safety)
    operation_counts: Mutex<HashMap<String, AtomicU64>>,
}

impl AudioProfiler {
    /// Create a new profiler instance
    pub fn new() -> Self {
        Self {
            total_callback_time: AtomicU64::new(0),
            callback_count: AtomicU64::new(0),
            max_callback_time: AtomicU64::new(0),
            min_callback_time: AtomicU64::new(u64::MAX),
            operation_times: Mutex::new(HashMap::new()),
            operation_counts: Mutex::new(HashMap::new()),
        }
    }

    /// Record start of an audio callback
    pub fn start_callback(&self) -> CallbackTimer {
        self.callback_count.fetch_add(1, Ordering::Relaxed);
        CallbackTimer {
            start_time: Instant::now(),
            profiler: self,
        }
    }

    /// Record timing for a specific operation
    pub fn record_operation(&self, operation: &str, duration: Duration) {
        let nanos = duration.as_nanos() as u64;
        
        // Update operation time
        if let Ok(mut times) = self.operation_times.lock() {
            let time_atomic = times.entry(operation.to_string())
                .or_insert_with(|| AtomicU64::new(0));
            time_atomic.fetch_add(nanos, Ordering::Relaxed);
        }
        
        // Update operation count
        if let Ok(mut counts) = self.operation_counts.lock() {
            let count_atomic = counts.entry(operation.to_string())
                .or_insert_with(|| AtomicU64::new(0));
            count_atomic.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get performance statistics
    pub fn get_stats(&self) -> ProfilerStats {
        let callback_count = self.callback_count.load(Ordering::Relaxed);
        let total_time = self.total_callback_time.load(Ordering::Relaxed);
        let max_time = self.max_callback_time.load(Ordering::Relaxed);
        let min_time = self.min_callback_time.load(Ordering::Relaxed);

        let avg_time = if callback_count > 0 {
            total_time / callback_count
        } else {
            0
        };

        let mut operation_stats = HashMap::new();
        
        // Collect operation stats
        if let (Ok(times), Ok(counts)) = (self.operation_times.lock(), self.operation_counts.lock()) {
            for (operation, time_atomic) in times.iter() {
                let count = counts.get(operation)
                    .map(|c| c.load(Ordering::Relaxed))
                    .unwrap_or(0);
                let time = time_atomic.load(Ordering::Relaxed);
                
                if count > 0 {
                    operation_stats.insert(operation.clone(), OperationStats {
                        total_time: time,
                        call_count: count,
                        avg_time: time / count,
                    });
                }
            }
        }

        ProfilerStats {
            callback_count,
            total_callback_time: total_time,
            avg_callback_time: avg_time,
            max_callback_time: max_time,
            min_callback_time: if min_time == u64::MAX { 0 } else { min_time },
            operation_stats,
        }
    }

    /// Reset all statistics
    pub fn reset(&self) {
        self.total_callback_time.store(0, Ordering::Relaxed);
        self.callback_count.store(0, Ordering::Relaxed);
        self.max_callback_time.store(0, Ordering::Relaxed);
        self.min_callback_time.store(u64::MAX, Ordering::Relaxed);
        
        if let Ok(times) = self.operation_times.lock() {
            for time in times.values() {
                time.store(0, Ordering::Relaxed);
            }
        }
        
        if let Ok(counts) = self.operation_counts.lock() {
            for count in counts.values() {
                count.store(0, Ordering::Relaxed);
            }
        }
    }

    /// Generate a flamegraph-compatible report
    pub fn generate_flamegraph_report(&self) -> String {
        let stats = self.get_stats();
        let mut report = String::new();
        
        report.push_str("# Audio Performance Profile\n\n");
        report.push_str(&format!("Total callbacks: {}\n", stats.callback_count));
        report.push_str(&format!("Avg callback time: {:.2}μs\n", stats.avg_callback_time as f64 / 1000.0));
        report.push_str(&format!("Max callback time: {:.2}μs\n", stats.max_callback_time as f64 / 1000.0));
        report.push_str(&format!("Min callback time: {:.2}μs\n\n", stats.min_callback_time as f64 / 1000.0));
        
        report.push_str("## Operation Breakdown\n\n");
        for (operation, op_stats) in &stats.operation_stats {
            report.push_str(&format!(
                "{}: {} calls, avg {:.2}μs, total {:.2}ms\n",
                operation,
                op_stats.call_count,
                op_stats.avg_time as f64 / 1000.0,
                op_stats.total_time as f64 / 1_000_000.0
            ));
        }
        
        report
    }
}

/// Timer for measuring audio callback duration
pub struct CallbackTimer<'a> {
    start_time: Instant,
    profiler: &'a AudioProfiler,
}

impl<'a> Drop for CallbackTimer<'a> {
    fn drop(&mut self) {
        let duration = self.start_time.elapsed();
        let nanos = duration.as_nanos() as u64;
        
        // Update total time
        self.profiler.total_callback_time.fetch_add(nanos, Ordering::Relaxed);
        
        // Update max time
        let mut current_max = self.profiler.max_callback_time.load(Ordering::Relaxed);
        while nanos > current_max {
            match self.profiler.max_callback_time.compare_exchange_weak(
                current_max,
                nanos,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }
        
        // Update min time
        let mut current_min = self.profiler.min_callback_time.load(Ordering::Relaxed);
        while nanos < current_min && current_min != u64::MAX {
            match self.profiler.min_callback_time.compare_exchange_weak(
                current_min,
                nanos,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_min = x,
            }
        }
    }
}

/// RAII timer for measuring specific operations
pub struct OperationTimer<'a> {
    operation: String,
    start_time: Instant,
    profiler: &'a AudioProfiler,
}

impl<'a> OperationTimer<'a> {
    /// Create a new operation timer
    pub fn new(operation: &str, profiler: &'a AudioProfiler) -> Self {
        Self {
            operation: operation.to_string(),
            start_time: Instant::now(),
            profiler,
        }
    }
}

impl<'a> Drop for OperationTimer<'a> {
    fn drop(&mut self) {
        let duration = self.start_time.elapsed();
        self.profiler.record_operation(&self.operation, duration);
    }
}

/// Performance statistics
#[derive(Debug, Clone)]
pub struct ProfilerStats {
    pub callback_count: u64,
    pub total_callback_time: u64,
    pub avg_callback_time: u64,
    pub max_callback_time: u64,
    pub min_callback_time: u64,
    pub operation_stats: HashMap<String, OperationStats>,
}

/// Statistics for a specific operation
#[derive(Debug, Clone)]
pub struct OperationStats {
    pub total_time: u64,
    pub call_count: u64,
    pub avg_time: u64,
}

/// Global profiler instance
static GLOBAL_PROFILER: std::sync::LazyLock<AudioProfiler> = std::sync::LazyLock::new(AudioProfiler::new);

/// Get global profiler instance
pub fn global_profiler() -> &'static AudioProfiler {
    &GLOBAL_PROFILER
}

/// Convenience function to start profiling a callback
pub fn start_callback_profiling() -> CallbackTimer<'static> {
    global_profiler().start_callback()
}

/// Convenience function to profile an operation
pub fn profile_operation(operation: &str) -> OperationTimer<'static> {
    OperationTimer::new(operation, global_profiler())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_profiler_basic_functionality() {
        let profiler = AudioProfiler::new();
        
        // Simulate some callbacks
        for _ in 0..10 {
            let _timer = profiler.start_callback();
            thread::sleep(Duration::from_micros(100));
        }
        
        let stats = profiler.get_stats();
        assert_eq!(stats.callback_count, 10);
        assert!(stats.avg_callback_time > 0);
        assert!(stats.max_callback_time >= stats.min_callback_time);
    }

    #[test]
    fn test_operation_profiling() {
        let profiler = AudioProfiler::new();
        
        // Profile some operations
        for _ in 0..5 {
            let _timer = OperationTimer::new("test_operation", &profiler);
            thread::sleep(Duration::from_micros(50));
        }
        
        let stats = profiler.get_stats();
        assert!(stats.operation_stats.contains_key("test_operation"));
        
        let op_stats = &stats.operation_stats["test_operation"];
        assert_eq!(op_stats.call_count, 5);
        assert!(op_stats.avg_time > 0);
    }

    #[test]
    fn test_profiler_reset() {
        let profiler = AudioProfiler::new();
        
        // Generate some data
        let _timer = profiler.start_callback();
        thread::sleep(Duration::from_micros(100));
        
        let stats_before = profiler.get_stats();
        assert!(stats_before.callback_count > 0);
        
        // Reset and check
        profiler.reset();
        let stats_after = profiler.get_stats();
        assert_eq!(stats_after.callback_count, 0);
        assert_eq!(stats_after.total_callback_time, 0);
    }
}