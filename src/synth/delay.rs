// Delay - Digital delay effect with feedback and mix control
//
// This module implements a simple digital delay using a circular buffer.
// The delay supports:
// - Variable delay time (in milliseconds or samples)
// - Feedback control (amount of delayed signal fed back into the delay line)
// - Dry/Wet mix control
//
// Real-time constraints:
// - Pre-allocated circular buffer (no allocations during processing)
// - Fixed maximum delay time (set at creation)
// - Lock-free processing

use crate::audio::dsp_utils::OnePoleSmoother;

/// Delay parameters
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DelayParams {
    /// Delay time in milliseconds (0.0 - max_time_ms)
    pub time_ms: f32,
    /// Feedback amount (0.0 - 1.0, where 0.95 is very long feedback)
    pub feedback: f32,
    /// Dry/Wet mix (0.0 = fully dry, 1.0 = fully wet)
    pub mix: f32,
    /// Enable/disable delay (bypass)
    pub enabled: bool,
}

impl Default for DelayParams {
    fn default() -> Self {
        Self {
            time_ms: 250.0,     // 250ms default delay
            feedback: 0.5,      // Moderate feedback
            mix: 0.3,           // 30% wet signal
            enabled: true,
        }
    }
}

impl DelayParams {
    /// Create new delay parameters with clamping
    pub fn new(time_ms: f32, feedback: f32, mix: f32) -> Self {
        Self {
            time_ms: time_ms.max(0.0),
            feedback: feedback.clamp(0.0, 0.99), // Max 0.99 to avoid runaway feedback
            mix: mix.clamp(0.0, 1.0),
            enabled: true,
        }
    }

    /// Validate and clamp parameters to safe ranges
    pub fn validate(&mut self, max_time_ms: f32) {
        self.time_ms = self.time_ms.clamp(0.0, max_time_ms);
        self.feedback = self.feedback.clamp(0.0, 0.99);
        self.mix = self.mix.clamp(0.0, 1.0);
    }
}

/// Delay effect implementation using a circular buffer
///
/// # Example
/// ```
/// use mymusic_daw::synth::delay::{Delay, DelayParams};
///
/// let params = DelayParams::default();
/// let mut delay = Delay::new(params, 44100.0, 1000.0); // Max 1 second delay
///
/// // Process audio
/// let output = delay.process(0.5);
/// ```
pub struct Delay {
    /// Delay parameters
    params: DelayParams,
    /// Sample rate
    sample_rate: f32,
    /// Maximum delay time in milliseconds
    max_time_ms: f32,
    /// Circular buffer for delay line
    buffer: Vec<f32>,
    /// Write position in buffer (where new samples are written)
    write_pos: usize,
    /// Current delay time in samples
    delay_samples: usize,
    /// Smoothers to avoid clicks when parameters change
    feedback_smoother: OnePoleSmoother,
    mix_smoother: OnePoleSmoother,
}

impl Delay {
    /// Create a new delay effect
    ///
    /// # Arguments
    /// * `params` - Initial delay parameters
    /// * `sample_rate` - Audio sample rate in Hz
    /// * `max_time_ms` - Maximum delay time in milliseconds (buffer size)
    pub fn new(params: DelayParams, sample_rate: f32, max_time_ms: f32) -> Self {
        // Calculate buffer size (add extra samples for safety)
        let max_samples = ((max_time_ms / 1000.0) * sample_rate) as usize + 1;

        // Pre-allocate buffer filled with zeros
        let buffer = vec![0.0; max_samples];

        // Calculate initial delay in samples
        let delay_samples = ((params.time_ms / 1000.0) * sample_rate) as usize;

        // Initialize smoothers (10ms time constant for smooth parameter changes)
        let feedback_smoother = OnePoleSmoother::new(params.feedback, 10.0, sample_rate);
        let mix_smoother = OnePoleSmoother::new(params.mix, 10.0, sample_rate);

        Self {
            params,
            sample_rate,
            max_time_ms,
            buffer,
            write_pos: 0,
            delay_samples: delay_samples.min(max_samples - 1),
            feedback_smoother,
            mix_smoother,
        }
    }

    /// Set delay parameters
    pub fn set_params(&mut self, mut params: DelayParams) {
        // Validate and clamp parameters
        params.validate(self.max_time_ms);

        self.params = params;

        // Update delay time in samples
        let new_delay_samples = ((params.time_ms / 1000.0) * self.sample_rate) as usize;
        self.delay_samples = new_delay_samples.min(self.buffer.len() - 1);
    }

    /// Get current delay parameters
    pub fn params(&self) -> DelayParams {
        self.params
    }

    /// Reset delay buffer (clear all delayed samples)
    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
    }

    /// Process a single sample through the delay
    ///
    /// # Arguments
    /// * `input` - Input sample
    ///
    /// # Returns
    /// Output sample (dry + wet mix)
    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        // If disabled, bypass
        if !self.params.enabled {
            return input;
        }

        // Apply smoothing to parameters
        let feedback = self.feedback_smoother.process(self.params.feedback);
        let mix = self.mix_smoother.process(self.params.mix);

        // Calculate read position (where we read delayed samples from)
        // Read position = write position - delay_samples (wrapping around)
        let read_pos = if self.write_pos >= self.delay_samples {
            self.write_pos - self.delay_samples
        } else {
            self.buffer.len() + self.write_pos - self.delay_samples
        };

        // Read delayed sample from buffer
        let delayed = self.buffer[read_pos];

        // Write new sample to buffer: input + feedback * delayed
        // This creates the delay with feedback
        let buffer_input = input + feedback * delayed;

        // Clamp to prevent runaway feedback (soft saturation)
        let buffer_input = buffer_input.clamp(-2.0, 2.0);

        self.buffer[self.write_pos] = buffer_input;

        // Advance write position (circular)
        self.write_pos = (self.write_pos + 1) % self.buffer.len();

        // Mix dry and wet signals
        let dry = input * (1.0 - mix);
        let wet = delayed * mix;

        dry + wet
    }

    /// Get latency in samples
    ///
    /// Delay effect has latency equal to the delay time.
    pub fn latency_samples(&self) -> usize {
        self.delay_samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delay_creation() {
        let params = DelayParams::default();
        let delay = Delay::new(params, 44100.0, 1000.0);

        assert_eq!(delay.params.time_ms, 250.0);
        assert_eq!(delay.params.feedback, 0.5);
        assert_eq!(delay.params.mix, 0.3);
        assert!(delay.params.enabled);
    }

    #[test]
    fn test_delay_params_clamping() {
        let params = DelayParams::new(500.0, 1.5, -0.5);

        assert_eq!(params.time_ms, 500.0);
        assert_eq!(params.feedback, 0.99); // Clamped to max
        assert_eq!(params.mix, 0.0); // Clamped to min
    }

    #[test]
    fn test_delay_bypass() {
        let mut params = DelayParams::default();
        params.enabled = false;

        let mut delay = Delay::new(params, 44100.0, 1000.0);

        let input = 0.5;
        let output = delay.process(input);

        // When bypassed, output should equal input
        assert_eq!(output, input);
    }

    #[test]
    fn test_delay_dry_signal() {
        let params = DelayParams {
            time_ms: 100.0,
            feedback: 0.0,
            mix: 0.0, // Fully dry
            enabled: true,
        };

        let mut delay = Delay::new(params, 44100.0, 1000.0);

        // First sample should pass through (no delayed signal yet)
        let output = delay.process(1.0);
        assert!(output > 0.9); // Almost fully dry (with smoothing)
    }

    #[test]
    fn test_delay_produces_delayed_signal() {
        let sample_rate = 44100.0;
        let delay_time_ms = 10.0; // 10ms delay
        let delay_samples = (delay_time_ms / 1000.0 * sample_rate) as usize;

        let params = DelayParams {
            time_ms: delay_time_ms,
            feedback: 0.0, // No feedback for this test
            mix: 1.0,      // Fully wet
            enabled: true,
        };

        let mut delay = Delay::new(params, sample_rate, 1000.0);

        // Warm up smoothers (10ms * 44100 = 441 samples)
        for _ in 0..500 {
            delay.process(0.0);
        }

        // Send an impulse
        delay.process(1.0);

        // Process silence and collect outputs around the expected delay time
        let mut max_output = 0.0_f32;
        for _ in 0..(delay_samples + 10) {
            let output = delay.process(0.0);
            max_output = max_output.max(output.abs());
        }

        // The delayed signal should be non-zero (the impulse arrived)
        assert!(max_output > 0.5, "Max delayed output: {}", max_output);
    }

    #[test]
    fn test_delay_feedback() {
        let sample_rate = 44100.0;
        let delay_time_ms = 10.0;

        let params = DelayParams {
            time_ms: delay_time_ms,
            feedback: 0.5, // 50% feedback
            mix: 1.0,      // Fully wet
            enabled: true,
        };

        let mut delay = Delay::new(params, sample_rate, 1000.0);

        // Warm up smoothers
        for _ in 0..500 {
            delay.process(0.0);
        }

        // Send an impulse
        delay.process(1.0);

        let delay_samples = (delay_time_ms / 1000.0 * sample_rate) as usize;

        // Process silence and check for multiple echoes
        let mut echo_levels = Vec::new();
        let mut max_in_window = 0.0_f32;

        for i in 0..(delay_samples * 4) {
            let output = delay.process(0.0);

            // Track max in each delay period
            max_in_window = max_in_window.max(output.abs());

            // At end of each delay period, record the peak
            if (i + 1) % delay_samples == 0 {
                echo_levels.push(max_in_window);
                max_in_window = 0.0;
            }
        }

        // Each echo should be quieter than the previous (due to feedback < 1.0)
        assert!(echo_levels.len() >= 3, "Not enough echo cycles captured");
        assert!(echo_levels[1] < echo_levels[0], "Second echo ({}) should be < first ({})", echo_levels[1], echo_levels[0]);
        assert!(echo_levels[2] < echo_levels[1], "Third echo ({}) should be < second ({})", echo_levels[2], echo_levels[1]);
    }

    #[test]
    fn test_delay_reset() {
        let params = DelayParams::default();
        let mut delay = Delay::new(params, 44100.0, 1000.0);

        // Process some samples to fill buffer
        for _ in 0..1000 {
            delay.process(0.5);
        }

        // Reset
        delay.reset();

        // Buffer should be cleared
        assert!(delay.buffer.iter().all(|&x| x == 0.0));
        assert_eq!(delay.write_pos, 0);
    }

    #[test]
    fn test_delay_stability() {
        let params = DelayParams {
            time_ms: 100.0,
            feedback: 0.9, // High feedback
            mix: 0.5,
            enabled: true,
        };

        let mut delay = Delay::new(params, 44100.0, 1000.0);

        // Process many samples and ensure no NaN or Inf
        for i in 0..10000 {
            let input = if i == 0 { 1.0 } else { 0.0 }; // Impulse at start
            let output = delay.process(input);

            assert!(output.is_finite(), "Sample {} is not finite: {}", i, output);
            assert!(output.abs() < 10.0, "Sample {} exceeds reasonable bounds: {}", i, output);
        }
    }

    #[test]
    fn test_delay_latency() {
        let sample_rate = 44100.0;
        let delay_time_ms = 50.0;

        let params = DelayParams {
            time_ms: delay_time_ms,
            feedback: 0.0,
            mix: 1.0,
            enabled: true,
        };

        let delay = Delay::new(params, sample_rate, 1000.0);

        let expected_samples = (delay_time_ms / 1000.0 * sample_rate) as usize;
        assert_eq!(delay.latency_samples(), expected_samples);
    }

    #[test]
    fn test_delay_max_time_clamping() {
        let params = DelayParams {
            time_ms: 2000.0, // Request 2 seconds
            feedback: 0.5,
            mix: 0.5,
            enabled: true,
        };

        let mut delay = Delay::new(params, 44100.0, 1000.0); // Max 1 second

        // Set params should clamp to max
        let mut new_params = params;
        new_params.validate(1000.0);
        delay.set_params(new_params);

        assert_eq!(delay.params.time_ms, 1000.0);
    }

    #[test]
    fn test_delay_parameter_changes() {
        let params = DelayParams::default();
        let mut delay = Delay::new(params, 44100.0, 1000.0);

        // Change parameters
        let new_params = DelayParams {
            time_ms: 500.0,
            feedback: 0.8,
            mix: 0.7,
            enabled: true,
        };

        delay.set_params(new_params);

        assert_eq!(delay.params.time_ms, 500.0);
        assert_eq!(delay.params.feedback, 0.8);
        assert_eq!(delay.params.mix, 0.7);

        // Should still be stable after parameter change
        for _ in 0..100 {
            let output = delay.process(0.5);
            assert!(output.is_finite());
        }
    }

    #[test]
    fn test_delay_circular_buffer() {
        let sample_rate = 44100.0;
        let delay_time_ms = 10.0;
        let delay_samples = (delay_time_ms / 1000.0 * sample_rate) as usize;

        let params = DelayParams {
            time_ms: delay_time_ms,
            feedback: 0.0,
            mix: 1.0,
            enabled: true,
        };

        let mut delay = Delay::new(params, sample_rate, 100.0);

        // Process more samples than buffer size to test circular wrapping
        let buffer_size = delay.buffer.len();

        for i in 0..(buffer_size * 2) {
            let input = if i == 0 { 1.0 } else { 0.0 };
            let output = delay.process(input);
            assert!(output.is_finite());
        }

        // Write position should have wrapped around
        assert!(delay.write_pos < buffer_size);
    }
}
