// LFO (Low Frequency Oscillator) implementation
//
// Used for modulation of various parameters (pitch, volume, filter cutoff, etc.)
// Operates at low frequencies (0.1 Hz - 20 Hz typically)

use super::oscillator::{Oscillator, SimpleOscillator, WaveformType};

/// LFO modulation destination
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LfoDestination {
    /// No modulation
    None,
    /// Modulate pitch (vibrato)
    Pitch,
    /// Modulate volume (tremolo)
    Volume,
    /// Modulate filter cutoff (wah effect) - for Phase 3a
    FilterCutoff,
}

impl Default for LfoDestination {
    fn default() -> Self {
        Self::None
    }
}

/// LFO parameters
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LfoParams {
    /// LFO waveform
    pub waveform: WaveformType,
    /// LFO frequency in Hz (0.1 to 20.0)
    pub rate: f32,
    /// Modulation depth (0.0 to 1.0)
    pub depth: f32,
    /// Modulation destination
    pub destination: LfoDestination,
}

impl LfoParams {
    /// Create LFO parameters with validation
    pub fn new(waveform: WaveformType, rate: f32, depth: f32, destination: LfoDestination) -> Self {
        Self {
            waveform,
            rate: rate.clamp(0.1, 20.0),
            depth: depth.clamp(0.0, 1.0),
            destination,
        }
    }

    /// Validate and clamp parameters
    pub fn validate(&mut self) {
        self.rate = self.rate.clamp(0.1, 20.0);
        self.depth = self.depth.clamp(0.0, 1.0);
    }
}

impl Default for LfoParams {
    fn default() -> Self {
        Self {
            waveform: WaveformType::Sine,
            rate: 5.0,  // 5 Hz
            depth: 0.5, // 50% modulation depth
            destination: LfoDestination::None,
        }
    }
}

/// LFO Generator
///
/// Low frequency oscillator for modulating parameters.
/// Uses the same oscillator as audio but at much lower frequencies.
pub struct Lfo {
    params: LfoParams,
    oscillator: SimpleOscillator,
}

impl Lfo {
    /// Create a new LFO
    pub fn new(params: LfoParams, sample_rate: f32) -> Self {
        let mut oscillator = SimpleOscillator::new(params.waveform, sample_rate);
        oscillator.set_frequency(params.rate);

        Self { params, oscillator }
    }

    /// Set new LFO parameters
    pub fn set_params(&mut self, params: LfoParams) {
        let rate_changed = (self.params.rate - params.rate).abs() > 0.001;
        let waveform_changed = self.params.waveform != params.waveform;

        self.params = params;

        if rate_changed {
            self.oscillator.set_frequency(self.params.rate);
        }

        if waveform_changed {
            // Need to recreate oscillator with new waveform
            // We lose phase continuity here, but that's acceptable for LFO parameter changes
            let sample_rate = self.oscillator.sample_rate;
            self.oscillator = SimpleOscillator::new(self.params.waveform, sample_rate);
            self.oscillator.set_frequency(self.params.rate);
        }
    }

    /// Get current parameters
    pub fn params(&self) -> LfoParams {
        self.params
    }

    /// Set LFO rate (frequency in Hz)
    pub fn set_rate(&mut self, rate: f32) {
        let rate = rate.clamp(0.1, 20.0);
        if (self.params.rate - rate).abs() > 0.001 {
            self.params.rate = rate;
            self.oscillator.set_frequency(rate);
        }
    }

    /// Set modulation depth (0.0 to 1.0)
    pub fn set_depth(&mut self, depth: f32) {
        self.params.depth = depth.clamp(0.0, 1.0);
    }

    /// Set waveform type
    pub fn set_waveform(&mut self, waveform: WaveformType) {
        if self.params.waveform != waveform {
            self.params.waveform = waveform;
            let sample_rate = self.oscillator.sample_rate;
            self.oscillator = SimpleOscillator::new(waveform, sample_rate);
            self.oscillator.set_frequency(self.params.rate);
        }
    }

    /// Set modulation destination
    pub fn set_destination(&mut self, destination: LfoDestination) {
        self.params.destination = destination;
    }

    /// Process one sample and return the modulation value
    ///
    /// Returns a value between -depth and +depth (centered around 0)
    /// The caller is responsible for applying this modulation to the target parameter
    pub fn process(&mut self) -> f32 {
        // Get oscillator sample (range -1.0 to 1.0)
        let osc_value = self.oscillator.next_sample();

        // Scale by depth
        osc_value * self.params.depth
    }

    /// Reset LFO phase to beginning
    pub fn reset(&mut self) {
        self.oscillator.reset();
    }

    /// Get the modulation destination
    pub fn destination(&self) -> LfoDestination {
        self.params.destination
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SAMPLE_RATE: f32 = 48000.0;
    const EPSILON: f32 = 0.001;

    #[test]
    fn test_lfo_params_default() {
        let params = LfoParams::default();
        assert_eq!(params.waveform, WaveformType::Sine);
        assert_eq!(params.rate, 5.0);
        assert_eq!(params.depth, 0.5);
        assert_eq!(params.destination, LfoDestination::None);
    }

    #[test]
    fn test_lfo_params_clamping() {
        let params = LfoParams::new(WaveformType::Sine, -1.0, 2.0, LfoDestination::Pitch);
        assert!(params.rate >= 0.1);
        assert!(params.depth <= 1.0);
    }

    #[test]
    fn test_lfo_creation() {
        let params = LfoParams::default();
        let lfo = Lfo::new(params, TEST_SAMPLE_RATE);
        assert_eq!(lfo.params(), params);
    }

    #[test]
    fn test_lfo_output_range() {
        let params = LfoParams::new(WaveformType::Sine, 5.0, 0.5, LfoDestination::Pitch);
        let mut lfo = Lfo::new(params, TEST_SAMPLE_RATE);

        // Generate samples and check they're in the expected range
        for _ in 0..10000 {
            let value = lfo.process();
            assert!(
                (-0.5..=0.5).contains(&value),
                "LFO value {} out of range [-0.5, 0.5]",
                value
            );
        }
    }

    #[test]
    fn test_lfo_zero_depth() {
        let params = LfoParams::new(WaveformType::Sine, 5.0, 0.0, LfoDestination::Pitch);
        let mut lfo = Lfo::new(params, TEST_SAMPLE_RATE);

        // With zero depth, all values should be zero
        for _ in 0..1000 {
            let value = lfo.process();
            assert!(value.abs() < EPSILON, "Expected 0, got {}", value);
        }
    }

    #[test]
    fn test_lfo_full_depth() {
        let params = LfoParams::new(WaveformType::Sine, 5.0, 1.0, LfoDestination::Pitch);
        let mut lfo = Lfo::new(params, TEST_SAMPLE_RATE);

        // With full depth, values should span -1.0 to 1.0
        for _ in 0..10000 {
            let value = lfo.process();
            assert!(
                (-1.0..=1.0).contains(&value),
                "LFO value {} out of range [-1.0, 1.0]",
                value
            );
        }
    }

    #[test]
    fn test_lfo_set_rate() {
        let params = LfoParams::default();
        let mut lfo = Lfo::new(params, TEST_SAMPLE_RATE);

        lfo.set_rate(10.0);
        assert!((lfo.params().rate - 10.0).abs() < EPSILON);

        // Test clamping
        lfo.set_rate(100.0);
        assert!(lfo.params().rate <= 20.0);

        lfo.set_rate(0.01);
        assert!(lfo.params().rate >= 0.1);
    }

    #[test]
    fn test_lfo_set_depth() {
        let params = LfoParams::default();
        let mut lfo = Lfo::new(params, TEST_SAMPLE_RATE);

        lfo.set_depth(0.75);
        assert!((lfo.params().depth - 0.75).abs() < EPSILON);

        // Test clamping
        lfo.set_depth(2.0);
        assert!(lfo.params().depth <= 1.0);

        lfo.set_depth(-1.0);
        assert!(lfo.params().depth >= 0.0);
    }

    #[test]
    fn test_lfo_set_waveform() {
        let params = LfoParams::default();
        let mut lfo = Lfo::new(params, TEST_SAMPLE_RATE);

        assert_eq!(lfo.params().waveform, WaveformType::Sine);

        lfo.set_waveform(WaveformType::Square);
        assert_eq!(lfo.params().waveform, WaveformType::Square);
    }

    #[test]
    fn test_lfo_destination() {
        let params = LfoParams::default();
        let mut lfo = Lfo::new(params, TEST_SAMPLE_RATE);

        assert_eq!(lfo.destination(), LfoDestination::None);

        lfo.set_destination(LfoDestination::Pitch);
        assert_eq!(lfo.destination(), LfoDestination::Pitch);

        lfo.set_destination(LfoDestination::Volume);
        assert_eq!(lfo.destination(), LfoDestination::Volume);
    }

    #[test]
    fn test_lfo_reset() {
        let params = LfoParams::new(WaveformType::Sine, 5.0, 1.0, LfoDestination::Pitch);
        let mut lfo = Lfo::new(params, TEST_SAMPLE_RATE);

        // Advance the LFO
        for _ in 0..1000 {
            lfo.process();
        }

        // Reset and check that first value is close to 0 (sine starts at 0)
        lfo.reset();
        let first_value = lfo.process();
        assert!(
            first_value.abs() < 0.1,
            "Expected ~0 after reset, got {}",
            first_value
        );
    }
}
