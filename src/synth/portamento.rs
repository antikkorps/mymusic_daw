// Portamento/Glide - Smooth pitch transitions
//
// Portamento allows smooth frequency transitions between notes instead of instant pitch changes.
// Essential for expressive mono/legato playing.

use crate::audio::dsp_utils::OnePoleSmoother;

/// Portamento parameters
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PortamentoParams {
    /// Glide time in seconds (0.0 = instant, 0.001-2.0 = smooth glide)
    pub time: f32,
}

impl PortamentoParams {
    /// Create new portamento parameters
    ///
    /// # Arguments
    /// * `time` - Glide time in seconds (will be clamped to 0.0-2.0)
    pub fn new(time: f32) -> Self {
        Self {
            time: time.clamp(0.0, 2.0),
        }
    }

    /// Create instant portamento (no glide)
    pub fn instant() -> Self {
        Self { time: 0.0 }
    }

    /// Check if portamento is active (time > 0)
    pub fn is_active(&self) -> bool {
        self.time > 0.0
    }
}

impl Default for PortamentoParams {
    fn default() -> Self {
        Self::instant()
    }
}

/// Portamento glide processor
///
/// Uses a one-pole smoother to create smooth frequency transitions.
/// The glide time controls how long it takes to reach the target frequency.
pub struct PortamentoGlide {
    params: PortamentoParams,
    smoother: OnePoleSmoother,
    sample_rate: f32,
}

impl PortamentoGlide {
    /// Create a new portamento glide processor
    ///
    /// # Arguments
    /// * `params` - Portamento parameters
    /// * `initial_frequency` - Starting frequency in Hz
    /// * `sample_rate` - Audio sample rate in Hz
    pub fn new(params: PortamentoParams, initial_frequency: f32, sample_rate: f32) -> Self {
        let time_ms = params.time * 1000.0; // Convert seconds to milliseconds
        let smoother = OnePoleSmoother::new(initial_frequency, time_ms.max(0.1), sample_rate);

        Self {
            params,
            smoother,
            sample_rate,
        }
    }

    /// Set target frequency (with glide if portamento is active)
    ///
    /// If portamento time is 0, frequency changes instantly.
    /// Otherwise, frequency glides smoothly to the target.
    pub fn set_target(&mut self, target_frequency: f32) {
        if !self.params.is_active() {
            // Instant change (no glide)
            self.smoother.reset(target_frequency);
        }
        // Otherwise, the smoother will glide to the target on next process() call
    }

    /// Process one sample and return the current (smoothed) frequency
    ///
    /// Call this once per audio sample to get the current frequency value.
    ///
    /// # Arguments
    /// * `target_frequency` - The target frequency to glide towards
    ///
    /// # Returns
    /// The current frequency (smoothed if portamento is active)
    pub fn process(&mut self, target_frequency: f32) -> f32 {
        if self.params.is_active() {
            self.smoother.process(target_frequency)
        } else {
            // No glide: return target directly
            target_frequency
        }
    }

    /// Reset to a specific frequency without gliding
    pub fn reset(&mut self, frequency: f32) {
        self.smoother.reset(frequency);
    }

    /// Update portamento parameters
    ///
    /// This updates the glide time. The change takes effect on the next set_target() call.
    pub fn set_params(&mut self, params: PortamentoParams) {
        self.params = params;

        // Update smoother time constant
        let time_ms = params.time * 1000.0;
        self.smoother =
            OnePoleSmoother::new(self.smoother.get(), time_ms.max(0.1), self.sample_rate);
    }

    /// Get current portamento parameters
    pub fn params(&self) -> PortamentoParams {
        self.params
    }

    /// Get current frequency (without processing)
    pub fn current_frequency(&self) -> f32 {
        self.smoother.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 44100.0;

    #[test]
    fn test_portamento_params_clamping() {
        let params = PortamentoParams::new(5.0); // Over max
        assert_eq!(params.time, 2.0);

        let params = PortamentoParams::new(-1.0); // Under min
        assert_eq!(params.time, 0.0);

        let params = PortamentoParams::new(0.5); // Valid
        assert_eq!(params.time, 0.5);
    }

    #[test]
    fn test_portamento_params_instant() {
        let params = PortamentoParams::instant();
        assert_eq!(params.time, 0.0);
        assert!(!params.is_active());
    }

    #[test]
    fn test_portamento_params_active() {
        let params = PortamentoParams::new(0.0);
        assert!(!params.is_active());

        let params = PortamentoParams::new(0.1);
        assert!(params.is_active());
    }

    #[test]
    fn test_portamento_instant_change() {
        let params = PortamentoParams::instant();
        let mut glide = PortamentoGlide::new(params, 440.0, SAMPLE_RATE);

        // With instant portamento, frequency should change immediately
        glide.set_target(880.0);
        let freq = glide.process(880.0);
        assert_eq!(freq, 880.0);
    }

    #[test]
    fn test_portamento_glide() {
        let params = PortamentoParams::new(0.1); // 100ms glide
        let mut glide = PortamentoGlide::new(params, 440.0, SAMPLE_RATE);

        // Start at 440Hz
        assert_eq!(glide.current_frequency(), 440.0);

        // Set target to 880Hz
        glide.set_target(880.0);

        // After one sample, should have moved towards target but not reached it
        let freq1 = glide.process(880.0);
        assert!(freq1 > 440.0);
        assert!(freq1 < 880.0);

        // After many samples (500ms = 22050 samples = 5x time constant), should be very close to target
        // One-pole smoothers need ~5x the time constant to reach 99% convergence
        for _ in 0..22050 {
            glide.process(880.0);
        }
        let final_freq = glide.current_frequency();
        assert!(
            (final_freq - 880.0).abs() < 20.0,
            "Final freq: {}",
            final_freq
        ); // Within 20Hz
    }

    #[test]
    fn test_portamento_reset() {
        let params = PortamentoParams::new(0.5);
        let mut glide = PortamentoGlide::new(params, 440.0, SAMPLE_RATE);

        // Start gliding to 880Hz
        glide.set_target(880.0);
        glide.process(880.0);

        // Reset should jump immediately
        glide.reset(220.0);
        assert_eq!(glide.current_frequency(), 220.0);
    }

    #[test]
    fn test_portamento_update_params() {
        let params = PortamentoParams::instant();
        let mut glide = PortamentoGlide::new(params, 440.0, SAMPLE_RATE);

        // Initially instant
        glide.set_target(880.0);
        assert_eq!(glide.process(880.0), 880.0);

        // Update to slow glide
        let slow_params = PortamentoParams::new(1.0);
        glide.set_params(slow_params);
        glide.reset(440.0);

        // Now should glide slowly
        glide.set_target(880.0);
        let freq = glide.process(880.0);
        assert!(freq > 440.0);
        assert!(freq < 880.0);
    }

    #[test]
    fn test_portamento_no_overshoot() {
        let params = PortamentoParams::new(0.1);
        let mut glide = PortamentoGlide::new(params, 440.0, SAMPLE_RATE);

        glide.set_target(880.0);

        // Should never overshoot the target
        for _ in 0..10000 {
            let freq = glide.process(880.0);
            assert!(freq >= 440.0);
            assert!(freq <= 880.0);
        }
    }

    #[test]
    fn test_portamento_downward_glide() {
        let params = PortamentoParams::new(0.1); // 100ms glide
        let mut glide = PortamentoGlide::new(params, 880.0, SAMPLE_RATE);

        // Glide down from 880Hz to 440Hz
        glide.set_target(440.0);

        let freq = glide.process(440.0);
        assert!(freq < 880.0);
        assert!(freq > 440.0);

        // Should converge to 440Hz (500ms = 5x time constant for 99% convergence)
        for _ in 0..22050 {
            glide.process(440.0);
        }
        let final_freq = glide.current_frequency();
        assert!(
            (final_freq - 440.0).abs() < 20.0,
            "Final freq: {}",
            final_freq
        );
    }
}
