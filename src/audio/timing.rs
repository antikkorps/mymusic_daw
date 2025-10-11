// Audio timing utilities for sample-accurate MIDI scheduling

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Shared audio timing state for sample-accurate MIDI scheduling
#[derive(Clone)]
pub struct AudioTiming {
    /// Current sample position (incremented by audio callback)
    sample_position: Arc<AtomicU64>,
    /// Sample rate (for timestamp conversions)
    sample_rate: f64,
}

impl AudioTiming {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_position: Arc::new(AtomicU64::new(0)),
            sample_rate: sample_rate as f64,
        }
    }

    /// Get current sample position (called from MIDI thread)
    pub fn current_sample(&self) -> u64 {
        self.sample_position.load(Ordering::Relaxed)
    }

    /// Advance sample position (called from audio callback)
    pub fn advance(&self, frames: usize) {
        self.sample_position
            .fetch_add(frames as u64, Ordering::Relaxed);
    }

    /// Convert microseconds timestamp to sample count
    /// Used to convert midir timestamps to sample-accurate timing
    pub fn micros_to_samples(&self, micros: u64) -> u64 {
        // samples = (micros / 1_000_000) * sample_rate
        // Rewritten to avoid floating point in critical path:
        // samples = (micros * sample_rate) / 1_000_000
        ((micros as f64 * self.sample_rate) / 1_000_000.0) as u64
    }

    /// Calculate samples_from_now for a MIDI event
    ///
    /// # Arguments
    /// * `midi_timestamp_micros` - Timestamp from midir (in microseconds)
    /// * `current_time_micros` - Current time reference (in microseconds)
    ///
    /// # Returns
    /// Number of samples from now when this event should be processed
    pub fn calculate_samples_from_now(
        &self,
        midi_timestamp_micros: u64,
        current_time_micros: u64,
    ) -> u32 {
        // Calculate time delta in microseconds
        let delta_micros = if midi_timestamp_micros >= current_time_micros {
            midi_timestamp_micros - current_time_micros
        } else {
            // Event is in the past, process immediately
            0
        };

        // Convert to samples
        let delta_samples = self.micros_to_samples(delta_micros);

        // Cap at u32::MAX for safety
        delta_samples.min(u32::MAX as u64) as u32
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timing_creation() {
        let timing = AudioTiming::new(48000.0);
        assert_eq!(timing.current_sample(), 0);
        assert_eq!(timing.sample_rate(), 48000.0);
    }

    #[test]
    fn test_advance_samples() {
        let timing = AudioTiming::new(48000.0);
        timing.advance(480);
        assert_eq!(timing.current_sample(), 480);
        timing.advance(480);
        assert_eq!(timing.current_sample(), 960);
    }

    #[test]
    fn test_micros_to_samples() {
        let timing = AudioTiming::new(48000.0);

        // 1 second = 1_000_000 micros = 48000 samples
        assert_eq!(timing.micros_to_samples(1_000_000), 48000);

        // 10ms = 10_000 micros = 480 samples @ 48kHz
        assert_eq!(timing.micros_to_samples(10_000), 480);

        // 1ms = 1_000 micros = 48 samples @ 48kHz
        assert_eq!(timing.micros_to_samples(1_000), 48);
    }

    #[test]
    fn test_calculate_samples_from_now_future() {
        let timing = AudioTiming::new(48000.0);

        // Event 10ms in the future
        let current_time = 1_000_000; // 1 second
        let midi_time = 1_010_000;    // 1.01 seconds

        let samples_from_now = timing.calculate_samples_from_now(midi_time, current_time);
        assert_eq!(samples_from_now, 480); // 10ms = 480 samples @ 48kHz
    }

    #[test]
    fn test_calculate_samples_from_now_past() {
        let timing = AudioTiming::new(48000.0);

        // Event in the past (should return 0)
        let current_time = 1_010_000;
        let midi_time = 1_000_000;

        let samples_from_now = timing.calculate_samples_from_now(midi_time, current_time);
        assert_eq!(samples_from_now, 0);
    }

    #[test]
    fn test_calculate_samples_from_now_immediate() {
        let timing = AudioTiming::new(48000.0);

        // Event at same time (should return 0)
        let current_time = 1_000_000;
        let midi_time = 1_000_000;

        let samples_from_now = timing.calculate_samples_from_now(midi_time, current_time);
        assert_eq!(timing.calculate_samples_from_now(midi_time, current_time), 0);
    }
}
