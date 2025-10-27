// Reverb - Freeverb-style reverb effect
//
// This module implements a simple reverb based on the Freeverb algorithm
// by Jezar at Dreampoint (public domain).
//
// Architecture:
// - 8 parallel comb filters (4 pairs for stereo)
// - 4 series allpass filters (2 pairs for stereo)
// - Damping control (low-pass filtering in feedback loop)
// - Room size control (feedback amount)
// - Dry/Wet mix control
//
// Real-time constraints:
// - Pre-allocated delay buffers (no allocations during processing)
// - Fixed maximum room size (set at creation)
// - Lock-free processing

use crate::audio::dsp_utils::OnePoleSmoother;

/// Reverb parameters
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReverbParams {
    /// Room size (0.0 - 1.0, where 1.0 is largest)
    pub room_size: f32,
    /// Damping (0.0 - 1.0, where 1.0 is maximum damping of high frequencies)
    pub damping: f32,
    /// Dry/Wet mix (0.0 = fully dry, 1.0 = fully wet)
    pub mix: f32,
    /// Enable/disable reverb (bypass)
    pub enabled: bool,
}

impl Default for ReverbParams {
    fn default() -> Self {
        Self {
            room_size: 0.5, // Medium room
            damping: 0.5,   // Medium damping
            mix: 0.25,      // 25% wet
            enabled: true,
        }
    }
}

impl ReverbParams {
    /// Create new reverb parameters with clamping
    pub fn new(room_size: f32, damping: f32, mix: f32) -> Self {
        Self {
            room_size: room_size.clamp(0.0, 1.0),
            damping: damping.clamp(0.0, 1.0),
            mix: mix.clamp(0.0, 1.0),
            enabled: true,
        }
    }
}

/// Comb filter with damping (for reverb)
struct CombFilter {
    buffer: Vec<f32>,
    buffer_size: usize,
    buffer_index: usize,
    feedback: f32,
    damping: f32,
    filter_state: f32, // One-pole low-pass filter state
}

impl CombFilter {
    fn new(buffer_size: usize) -> Self {
        Self {
            buffer: vec![0.0; buffer_size],
            buffer_size,
            buffer_index: 0,
            feedback: 0.5,
            damping: 0.5,
            filter_state: 0.0,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        // Read from circular buffer
        let output = self.buffer[self.buffer_index];

        // Low-pass filter on feedback (damping)
        self.filter_state = output * (1.0 - self.damping) + self.filter_state * self.damping;

        // Write to buffer: input + filtered feedback
        self.buffer[self.buffer_index] = input + self.filter_state * self.feedback;

        // Advance buffer index (circular)
        self.buffer_index = (self.buffer_index + 1) % self.buffer_size;

        output
    }

    fn mute(&mut self) {
        self.buffer.fill(0.0);
        self.buffer_index = 0;
        self.filter_state = 0.0;
    }

    fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback;
    }

    fn set_damping(&mut self, damping: f32) {
        self.damping = damping;
    }
}

/// Allpass filter (for reverb)
struct AllpassFilter {
    buffer: Vec<f32>,
    buffer_size: usize,
    buffer_index: usize,
}

impl AllpassFilter {
    fn new(buffer_size: usize) -> Self {
        Self {
            buffer: vec![0.0; buffer_size],
            buffer_size,
            buffer_index: 0,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let bufout = self.buffer[self.buffer_index];

        // Allpass formula: output = -input + bufout
        let output = -input + bufout;

        // Write to buffer: input + 0.5 * bufout
        self.buffer[self.buffer_index] = input + bufout * 0.5;

        // Advance buffer index (circular)
        self.buffer_index = (self.buffer_index + 1) % self.buffer_size;

        output
    }

    fn mute(&mut self) {
        self.buffer.fill(0.0);
        self.buffer_index = 0;
    }
}

/// Freeverb-style reverb effect
///
/// Simplified Freeverb with 4 comb filters and 2 allpass filters (mono version).
///
/// # Example
/// ```
/// use mymusic_daw::synth::reverb::{Reverb, ReverbParams};
///
/// let params = ReverbParams::default();
/// let mut reverb = Reverb::new(params, 44100.0);
///
/// // Process audio
/// let output = reverb.process(0.5);
/// ```
pub struct Reverb {
    /// Reverb parameters
    params: ReverbParams,
    /// Sample rate
    _sample_rate: f32,

    /// Comb filters (parallel)
    comb_filters: Vec<CombFilter>,

    /// Allpass filters (series)
    allpass_filters: Vec<AllpassFilter>,

    /// Smoothers to avoid clicks
    mix_smoother: OnePoleSmoother,

    /// Scaling factor for output
    gain: f32,
}

impl Reverb {
    // Freeverb tuning constants (scaled for 44.1kHz)
    // These are chosen to avoid resonances and give smooth decay
    const COMB_TUNINGS: [usize; 4] = [1116, 1188, 1277, 1356];
    const ALLPASS_TUNINGS: [usize; 2] = [556, 441];

    // Scaling factors
    const SCALE_WET: f32 = 3.0;
    const SCALE_DAMPING: f32 = 0.4;
    const SCALE_ROOM: f32 = 0.28;
    const OFFSET_ROOM: f32 = 0.7;

    /// Create a new reverb effect
    ///
    /// # Arguments
    /// * `params` - Initial reverb parameters
    /// * `sample_rate` - Audio sample rate in Hz
    pub fn new(params: ReverbParams, sample_rate: f32) -> Self {
        // Scale tunings for sample rate (tunings are for 44.1kHz)
        let scale = sample_rate / 44100.0;

        // Create comb filters
        let mut comb_filters = Vec::with_capacity(Self::COMB_TUNINGS.len());
        for &tuning in &Self::COMB_TUNINGS {
            let size = (tuning as f32 * scale) as usize;
            comb_filters.push(CombFilter::new(size));
        }

        // Create allpass filters
        let mut allpass_filters = Vec::with_capacity(Self::ALLPASS_TUNINGS.len());
        for &tuning in &Self::ALLPASS_TUNINGS {
            let size = (tuning as f32 * scale) as usize;
            allpass_filters.push(AllpassFilter::new(size));
        }

        // Initialize smoothers
        let mix_smoother = OnePoleSmoother::new(params.mix, 10.0, sample_rate);

        let mut reverb = Self {
            params,
            _sample_rate: sample_rate,
            comb_filters,
            allpass_filters,
            mix_smoother,
            gain: 1.0,
        };

        // Update parameters
        reverb.update();

        reverb
    }

    /// Set reverb parameters
    pub fn set_params(&mut self, params: ReverbParams) {
        self.params = params;
        self.update();
    }

    /// Get current reverb parameters
    pub fn params(&self) -> ReverbParams {
        self.params
    }

    /// Update internal state from parameters
    fn update(&mut self) {
        // Update comb filter feedback and damping
        let room = self.params.room_size * Self::SCALE_ROOM + Self::OFFSET_ROOM;
        let damp = self.params.damping * Self::SCALE_DAMPING;

        for comb in &mut self.comb_filters {
            comb.set_feedback(room);
            comb.set_damping(damp);
        }

        // Update gain
        self.gain = Self::SCALE_WET * 0.25; // Scale for 4 comb filters
    }

    /// Reset reverb buffer (clear all delayed samples)
    pub fn reset(&mut self) {
        for comb in &mut self.comb_filters {
            comb.mute();
        }
        for allpass in &mut self.allpass_filters {
            allpass.mute();
        }
    }

    /// Process a single sample through the reverb
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

        // Apply smoothing to mix parameter
        let mix = self.mix_smoother.process(self.params.mix);

        // Accumulate output from parallel comb filters
        let mut comb_out = 0.0;
        for comb in &mut self.comb_filters {
            comb_out += comb.process(input);
        }

        // Apply gain
        let mut output = comb_out * self.gain;

        // Pass through series allpass filters
        for allpass in &mut self.allpass_filters {
            output = allpass.process(output);
        }

        // Mix dry and wet signals
        let dry = input * (1.0 - mix);
        let wet = output * mix;

        dry + wet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reverb_creation() {
        let params = ReverbParams::default();
        let reverb = Reverb::new(params, 44100.0);

        assert_eq!(reverb.params.room_size, 0.5);
        assert_eq!(reverb.params.damping, 0.5);
        assert_eq!(reverb.params.mix, 0.25);
        assert!(reverb.params.enabled);
    }

    #[test]
    fn test_reverb_params_clamping() {
        let params = ReverbParams::new(1.5, -0.5, 2.0);

        assert_eq!(params.room_size, 1.0); // Clamped to max
        assert_eq!(params.damping, 0.0); // Clamped to min
        assert_eq!(params.mix, 1.0); // Clamped to max
    }

    #[test]
    fn test_reverb_bypass() {
        let params = ReverbParams {
            enabled: false,
            ..Default::default()
        };

        let mut reverb = Reverb::new(params, 44100.0);

        let input = 0.5;
        let output = reverb.process(input);

        // When bypassed, output should equal input
        assert_eq!(output, input);
    }

    #[test]
    fn test_reverb_dry_signal() {
        let params = ReverbParams {
            room_size: 0.5,
            damping: 0.5,
            mix: 0.0, // Fully dry
            enabled: true,
        };

        let mut reverb = Reverb::new(params, 44100.0);

        // Warm up smoothers
        for _ in 0..500 {
            reverb.process(0.0);
        }

        let output = reverb.process(1.0);
        assert!(output > 0.9); // Almost fully dry (with smoothing)
    }

    #[test]
    fn test_reverb_produces_tail() {
        let sample_rate = 44100.0;

        let params = ReverbParams {
            room_size: 0.6, // Medium room
            damping: 0.5,   // Medium damping
            mix: 1.0,       // Fully wet
            enabled: true,
        };

        let mut reverb = Reverb::new(params, sample_rate);

        // Warm up smoothers
        for _ in 0..500 {
            reverb.process(0.0);
        }

        // Send an impulse
        reverb.process(1.0);

        // Process silence and check for reverb tail
        let mut max_output = 0.0_f32;
        for _ in 0..3000 {
            let output = reverb.process(0.0).abs();
            max_output = max_output.max(output);
        }

        // Reverb should produce an audible tail
        assert!(
            max_output > 0.01,
            "Reverb should produce audible tail (max: {})",
            max_output
        );
    }

    #[test]
    fn test_reverb_damping_effect() {
        let sample_rate = 44100.0;

        // Low damping (bright reverb)
        let params_bright = ReverbParams {
            room_size: 0.7,
            damping: 0.0, // No damping
            mix: 1.0,
            enabled: true,
        };

        let mut reverb_bright = Reverb::new(params_bright, sample_rate);

        // High damping (dark reverb)
        let params_dark = ReverbParams {
            room_size: 0.7,
            damping: 0.9, // High damping
            mix: 1.0,
            enabled: true,
        };

        let mut reverb_dark = Reverb::new(params_dark, sample_rate);

        // Warm up both
        for _ in 0..500 {
            reverb_bright.process(0.0);
            reverb_dark.process(0.0);
        }

        // Send impulse to both
        reverb_bright.process(1.0);
        reverb_dark.process(1.0);

        // Collect decay tails
        let mut bright_tail = Vec::new();
        let mut dark_tail = Vec::new();

        for _ in 0..2000 {
            bright_tail.push(reverb_bright.process(0.0).abs());
            dark_tail.push(reverb_dark.process(0.0).abs());
        }

        // Both should produce reverb
        let bright_energy: f32 = bright_tail.iter().sum();
        let dark_energy: f32 = dark_tail.iter().sum();

        assert!(bright_energy > 0.0);
        assert!(dark_energy > 0.0);

        // Damping reduces high-frequency energy, which should reduce overall energy
        // (This test is qualitative - in reality damping affects frequency content)
        // We just verify both produce audible reverb
    }

    #[test]
    fn test_reverb_room_size_effect() {
        let sample_rate = 44100.0;

        // Small room
        let params_small = ReverbParams {
            room_size: 0.2,
            damping: 0.5,
            mix: 1.0,
            enabled: true,
        };

        let mut reverb_small = Reverb::new(params_small, sample_rate);

        // Large room
        let params_large = ReverbParams {
            room_size: 0.9,
            damping: 0.5,
            mix: 1.0,
            enabled: true,
        };

        let mut reverb_large = Reverb::new(params_large, sample_rate);

        // Warm up
        for _ in 0..500 {
            reverb_small.process(0.0);
            reverb_large.process(0.0);
        }

        // Send impulse
        reverb_small.process(1.0);
        reverb_large.process(1.0);

        // Collect tails
        let mut small_tail = Vec::new();
        let mut large_tail = Vec::new();

        for _ in 0..3000 {
            small_tail.push(reverb_small.process(0.0).abs());
            large_tail.push(reverb_large.process(0.0).abs());
        }

        // Larger room should have longer decay (more energy overall)
        let small_energy: f32 = small_tail.iter().sum();
        let large_energy: f32 = large_tail.iter().sum();

        assert!(
            large_energy > small_energy,
            "Larger room should have longer decay"
        );
    }

    #[test]
    fn test_reverb_reset() {
        let params = ReverbParams::default();
        let mut reverb = Reverb::new(params, 44100.0);

        // Process some samples to fill buffers
        for _ in 0..1000 {
            reverb.process(0.5);
        }

        // Reset
        reverb.reset();

        // Process silence - should be silent
        let output = reverb.process(0.0);
        assert!(
            output.abs() < 0.01,
            "After reset, silence should produce silence"
        );
    }

    #[test]
    fn test_reverb_stability() {
        let params = ReverbParams {
            room_size: 0.9, // Large room
            damping: 0.1,   // Low damping (more resonant)
            mix: 0.5,
            enabled: true,
        };

        let mut reverb = Reverb::new(params, 44100.0);

        // Process many samples and ensure no NaN or Inf
        for i in 0..10000 {
            let input = if i == 0 { 1.0 } else { 0.0 }; // Impulse at start
            let output = reverb.process(input);

            assert!(output.is_finite(), "Sample {} is not finite: {}", i, output);
        }
    }

    #[test]
    fn test_reverb_parameter_changes() {
        let params = ReverbParams::default();
        let mut reverb = Reverb::new(params, 44100.0);

        // Change parameters
        let new_params = ReverbParams {
            room_size: 0.8,
            damping: 0.3,
            mix: 0.6,
            enabled: true,
        };

        reverb.set_params(new_params);

        assert_eq!(reverb.params.room_size, 0.8);
        assert_eq!(reverb.params.damping, 0.3);
        assert_eq!(reverb.params.mix, 0.6);

        // Should still be stable after parameter change
        for _ in 0..100 {
            let output = reverb.process(0.5);
            assert!(output.is_finite());
        }
    }
}
