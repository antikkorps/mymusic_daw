// Filter - State Variable Filter (Chamberlin)
//
// Digital implementation of a 2-pole State Variable Filter with simultaneous
// low-pass, high-pass, band-pass, and notch outputs.
//
// References:
// - Hal Chamberlin's "Musical Applications of Microprocessors" (1985)
// - https://www.earlevel.com/main/2003/03/02/the-digital-state-variable-filter/
//
// Characteristics:
// - 12dB/octave slope (2-pole)
// - Stable up to ~Fs/6 (8kHz @ 48kHz sample rate)
// - Independent frequency and Q control
// - Simultaneous outputs (LP, HP, BP, Notch)

use crate::audio::dsp_utils::OnePoleSmoother;
use std::f32::consts::PI;

/// Filter type/mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterType {
    /// Low-pass filter (12dB/octave)
    #[default]
    LowPass,
    /// High-pass filter (12dB/octave)
    HighPass,
    /// Band-pass filter (6dB/octave on each side)
    BandPass,
    /// Notch/band-reject filter
    Notch,
}

/// Filter parameters
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterParams {
    /// Cutoff frequency in Hz (20Hz - 20kHz)
    pub cutoff: f32,
    /// Resonance (Q factor: 0.5 - 20.0, where >10 can self-oscillate)
    pub resonance: f32,
    /// Filter type
    pub filter_type: FilterType,
    /// Enable/disable filter (bypass)
    pub enabled: bool,
}

impl Default for FilterParams {
    fn default() -> Self {
        Self {
            cutoff: 1000.0,   // 1kHz default
            resonance: 0.707, // Butterworth response (Q = 0.707)
            filter_type: FilterType::LowPass,
            enabled: true,
        }
    }
}

/// State Variable Filter (Chamberlin) implementation
///
/// This filter uses the Chamberlin digital approximation of a state variable filter.
/// It maintains two state variables (low-pass and band-pass) and computes all outputs
/// simultaneously.
///
/// # Example
/// ```
/// use mymusic_daw::synth::filter::{StateVariableFilter, FilterParams, FilterType};
///
/// let params = FilterParams {
///     cutoff: 1000.0,
///     resonance: 1.0,
///     filter_type: FilterType::LowPass,
///     enabled: true,
/// };
///
/// let mut filter = StateVariableFilter::new(params, 44100.0);
///
/// // Process audio
/// let output = filter.process(input_sample);
/// ```
pub struct StateVariableFilter {
    // Filter parameters
    params: FilterParams,
    sample_rate: f32,

    // State variables
    low: f32,  // Low-pass state
    band: f32, // Band-pass state

    // Coefficients (computed from cutoff and resonance)
    f: f32, // Frequency coefficient
    q: f32, // Resonance coefficient (damping)

    // Smoothers to avoid zipper noise when parameters change
    cutoff_smoother: OnePoleSmoother,
    resonance_smoother: OnePoleSmoother,
}

impl StateVariableFilter {
    /// Create a new State Variable Filter
    ///
    /// # Arguments
    /// * `params` - Initial filter parameters
    /// * `sample_rate` - Audio sample rate in Hz
    pub fn new(params: FilterParams, sample_rate: f32) -> Self {
        // Initialize smoothers with 5ms time constant (fast but no clicks)
        let cutoff_smoother = OnePoleSmoother::new(params.cutoff, 5.0, sample_rate);
        let resonance_smoother = OnePoleSmoother::new(params.resonance, 5.0, sample_rate);

        let mut filter = Self {
            params,
            sample_rate,
            low: 0.0,
            band: 0.0,
            f: 0.0,
            q: 0.0,
            cutoff_smoother,
            resonance_smoother,
        };

        // Compute initial coefficients
        filter.update_coefficients(params.cutoff, params.resonance);

        filter
    }

    /// Update filter parameters
    ///
    /// Parameters are smoothed internally to avoid zipper noise.
    pub fn set_params(&mut self, params: FilterParams) {
        self.params = params;
        // Smoothing is applied in process() to maintain RT-safety
    }

    /// Get current filter parameters
    pub fn params(&self) -> FilterParams {
        self.params
    }

    /// Reset filter state (clear delay lines)
    ///
    /// Useful when switching notes or resetting the synth to avoid clicks.
    pub fn reset(&mut self) {
        self.low = 0.0;
        self.band = 0.0;
    }

    /// Update internal coefficients from cutoff and resonance
    ///
    /// # Formulas
    /// - `f = 2 * sin(π * fc / Fs)` - frequency coefficient
    /// - `q = 1 / Q` - damping coefficient
    ///
    /// # Stability
    /// Cutoff is clamped to Fs/6 to ensure numerical stability.
    fn update_coefficients(&mut self, cutoff: f32, resonance: f32) {
        // Clamp cutoff to safe range: 20Hz to Fs/6 (stability limit)
        let max_cutoff = self.sample_rate / 6.0;
        let safe_cutoff = cutoff.clamp(20.0, max_cutoff);

        // Compute frequency coefficient: f = 2 * sin(π * fc / Fs)
        self.f = 2.0 * (PI * safe_cutoff / self.sample_rate).sin();

        // Compute damping (resonance): q = 1/Q
        // Clamp Q to reasonable range: 0.5 (no resonance) to 20.0 (high resonance)
        let q_factor = resonance.clamp(0.5, 20.0);
        self.q = 1.0 / q_factor;

        // Clamp q to avoid instability
        self.q = self.q.clamp(0.01, 2.0);
    }

    /// Process a single sample through the filter
    ///
    /// # Arguments
    /// * `input` - Input sample
    ///
    /// # Returns
    /// Filtered output based on current filter type
    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        // If filter is disabled, bypass
        if !self.params.enabled {
            return input;
        }

        // Apply smoothing to parameters (avoid zipper noise)
        let smoothed_cutoff = self.cutoff_smoother.process(self.params.cutoff);
        let smoothed_resonance = self.resonance_smoother.process(self.params.resonance);

        // Update coefficients if parameters changed
        self.update_coefficients(smoothed_cutoff, smoothed_resonance);

        // Chamberlin State Variable Filter algorithm
        // Reference: Musical Applications of Microprocessors (Chamberlin, 1985)

        // Compute high-pass output: hp = input - low - q*band
        let high = input - self.low - self.q * self.band;

        // Update band-pass state: band = band + f*hp
        self.band += self.f * high;

        // Update low-pass state: low = low + f*band
        self.low += self.f * self.band;

        // Compute notch output: notch = input - q*band
        let notch = input - self.q * self.band;

        // Return output based on filter type
        match self.params.filter_type {
            FilterType::LowPass => self.low,
            FilterType::HighPass => high,
            FilterType::BandPass => self.band,
            FilterType::Notch => notch,
        }
    }

    /// Process a single sample with modulated cutoff
    ///
    /// This is optimized for real-time modulation (e.g., LFO or envelope).
    /// The cutoff parameter is applied directly without smoothing.
    ///
    /// # Arguments
    /// * `input` - Input sample
    /// * `modulated_cutoff` - Cutoff frequency in Hz (after modulation)
    ///
    /// # Returns
    /// Filtered output
    #[inline]
    pub fn process_modulated(&mut self, input: f32, modulated_cutoff: f32) -> f32 {
        if !self.params.enabled {
            return input;
        }

        // Apply resonance smoothing (but not cutoff - it's already modulated)
        let smoothed_resonance = self.resonance_smoother.process(self.params.resonance);

        // Update coefficients with modulated cutoff
        self.update_coefficients(modulated_cutoff, smoothed_resonance);

        // Same algorithm as process()
        let high = input - self.low - self.q * self.band;
        self.band += self.f * high;
        self.low += self.f * self.band;
        let notch = input - self.q * self.band;

        match self.params.filter_type {
            FilterType::LowPass => self.low,
            FilterType::HighPass => high,
            FilterType::BandPass => self.band,
            FilterType::Notch => notch,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_creation() {
        let params = FilterParams::default();
        let filter = StateVariableFilter::new(params, 44100.0);

        assert_eq!(filter.params.cutoff, 1000.0);
        assert_eq!(filter.params.resonance, 0.707);
        assert_eq!(filter.params.filter_type, FilterType::LowPass);
        assert!(filter.params.enabled);
    }

    #[test]
    fn test_filter_bypass() {
        let params = FilterParams {
            enabled: false,
            ..Default::default()
        };

        let mut filter = StateVariableFilter::new(params, 44100.0);

        let input = 0.5;
        let output = filter.process(input);

        // When bypassed, output should equal input
        assert_eq!(output, input);
    }

    #[test]
    fn test_lowpass_dc_blocking() {
        let params = FilterParams {
            cutoff: 100.0, // Very low cutoff
            resonance: 0.707,
            filter_type: FilterType::LowPass,
            enabled: true,
        };

        let mut filter = StateVariableFilter::new(params, 44100.0);

        // Feed DC signal (constant value)
        let mut last_output = 0.0;
        for _ in 0..1000 {
            last_output = filter.process(1.0);
        }

        // Low-pass should pass DC, so output should converge to 1.0
        assert!((last_output - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_highpass_dc_blocking() {
        let params = FilterParams {
            cutoff: 1000.0,
            resonance: 0.707,
            filter_type: FilterType::HighPass,
            enabled: true,
        };

        let mut filter = StateVariableFilter::new(params, 44100.0);

        // Feed DC signal
        let mut last_output = 0.0;
        for _ in 0..1000 {
            last_output = filter.process(1.0);
        }

        // High-pass should block DC, so output should converge to ~0
        assert!(last_output.abs() < 0.1);
    }

    #[test]
    fn test_filter_stability() {
        let params = FilterParams {
            cutoff: 5000.0,
            resonance: 10.0, // High resonance
            filter_type: FilterType::LowPass,
            enabled: true,
        };

        let mut filter = StateVariableFilter::new(params, 44100.0);

        // Process many samples and ensure no NaN or Inf
        for _ in 0..10000 {
            let output = filter.process(0.5);
            assert!(output.is_finite());
            assert!(!output.is_nan());
        }
    }

    #[test]
    fn test_resonance_clamping() {
        let params = FilterParams {
            cutoff: 1000.0,
            resonance: 100.0, // Excessive resonance
            filter_type: FilterType::LowPass,
            enabled: true,
        };

        let mut filter = StateVariableFilter::new(params, 44100.0);

        // Should still be stable despite excessive resonance
        for _ in 0..1000 {
            let output = filter.process(0.5);
            assert!(output.is_finite());
            assert!(output.abs() < 100.0); // Reasonable bounds
        }
    }

    #[test]
    fn test_cutoff_modulation() {
        let params = FilterParams::default();
        let mut filter = StateVariableFilter::new(params, 44100.0);

        // Modulate cutoff rapidly (simulate LFO)
        for i in 0..1000 {
            let modulated_cutoff = 500.0 + 500.0 * (i as f32 / 100.0).sin();
            let output = filter.process_modulated(0.5, modulated_cutoff);
            assert!(output.is_finite());
        }
    }

    #[test]
    fn test_all_filter_types() {
        let sample_rate = 44100.0;

        for filter_type in [
            FilterType::LowPass,
            FilterType::HighPass,
            FilterType::BandPass,
            FilterType::Notch,
        ] {
            let params = FilterParams {
                cutoff: 1000.0,
                resonance: 1.0,
                filter_type,
                enabled: true,
            };

            let mut filter = StateVariableFilter::new(params, sample_rate);

            // All filter types should be stable
            for _ in 0..1000 {
                let output = filter.process(0.5);
                assert!(output.is_finite());
            }
        }
    }

    /// Generate a sine wave at a given frequency
    fn generate_sine(frequency: f32, sample_rate: f32, num_samples: usize) -> Vec<f32> {
        (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate;
                (2.0 * PI * frequency * t).sin()
            })
            .collect()
    }

    /// Compute RMS (root mean square) level of a signal
    fn compute_rms(signal: &[f32]) -> f32 {
        let sum_squares: f32 = signal.iter().map(|x| x * x).sum();
        (sum_squares / signal.len() as f32).sqrt()
    }

    #[test]
    fn test_lowpass_frequency_response() {
        let sample_rate = 44100.0;
        let cutoff = 1000.0;

        let params = FilterParams {
            cutoff,
            resonance: 0.707, // Butterworth
            filter_type: FilterType::LowPass,
            enabled: true,
        };

        let mut filter = StateVariableFilter::new(params, sample_rate);

        // Test with a low frequency (well below cutoff) - should pass
        let low_freq = 200.0;
        let low_input = generate_sine(low_freq, sample_rate, 4410); // 100ms
        let mut low_output = vec![0.0; low_input.len()];
        for (i, &sample) in low_input.iter().enumerate() {
            low_output[i] = filter.process(sample);
        }

        // Skip first samples (filter settling), then compute RMS
        let low_input_rms = compute_rms(&low_input[1000..]);
        let low_output_rms = compute_rms(&low_output[1000..]);
        let low_attenuation = low_output_rms / low_input_rms;

        // Low frequency should pass with minimal attenuation (>0.8)
        assert!(
            low_attenuation > 0.8,
            "Low freq attenuation too high: {}",
            low_attenuation
        );

        // Reset filter for next test
        filter.reset();

        // Test with a high frequency (well above cutoff) - should attenuate
        let high_freq = 5000.0;
        let high_input = generate_sine(high_freq, sample_rate, 4410);
        let mut high_output = vec![0.0; high_input.len()];
        for (i, &sample) in high_input.iter().enumerate() {
            high_output[i] = filter.process(sample);
        }

        let high_input_rms = compute_rms(&high_input[1000..]);
        let high_output_rms = compute_rms(&high_output[1000..]);
        let high_attenuation = high_output_rms / high_input_rms;

        // High frequency should be attenuated significantly (<0.5)
        assert!(
            high_attenuation < 0.5,
            "High freq attenuation too low: {}",
            high_attenuation
        );
    }

    #[test]
    fn test_highpass_frequency_response() {
        let sample_rate = 44100.0;
        let cutoff = 1000.0;

        let params = FilterParams {
            cutoff,
            resonance: 0.707,
            filter_type: FilterType::HighPass,
            enabled: true,
        };

        let mut filter = StateVariableFilter::new(params, sample_rate);

        // Test with a low frequency (below cutoff) - should attenuate
        let low_freq = 200.0;
        let low_input = generate_sine(low_freq, sample_rate, 4410);
        let mut low_output = vec![0.0; low_input.len()];
        for (i, &sample) in low_input.iter().enumerate() {
            low_output[i] = filter.process(sample);
        }

        let low_input_rms = compute_rms(&low_input[1000..]);
        let low_output_rms = compute_rms(&low_output[1000..]);
        let low_attenuation = low_output_rms / low_input_rms;

        // Low frequency should be attenuated (<0.5)
        assert!(
            low_attenuation < 0.5,
            "Low freq attenuation too low: {}",
            low_attenuation
        );

        // Reset filter for next test
        filter.reset();

        // Test with a high frequency (above cutoff) - should pass
        let high_freq = 5000.0;
        let high_input = generate_sine(high_freq, sample_rate, 4410);
        let mut high_output = vec![0.0; high_input.len()];
        for (i, &sample) in high_input.iter().enumerate() {
            high_output[i] = filter.process(sample);
        }

        let high_input_rms = compute_rms(&high_input[1000..]);
        let high_output_rms = compute_rms(&high_output[1000..]);
        let high_attenuation = high_output_rms / high_input_rms;

        // High frequency should pass with minimal attenuation (>0.8)
        assert!(
            high_attenuation > 0.8,
            "High freq attenuation too high: {}",
            high_attenuation
        );
    }

    #[test]
    fn test_bandpass_frequency_response() {
        let sample_rate = 44100.0;
        let cutoff = 1000.0;

        let params = FilterParams {
            cutoff,
            resonance: 2.0, // Narrow bandpass
            filter_type: FilterType::BandPass,
            enabled: true,
        };

        let mut filter = StateVariableFilter::new(params, sample_rate);

        // Test at cutoff frequency - should pass
        let center_freq = 1000.0;
        let center_input = generate_sine(center_freq, sample_rate, 4410);
        let mut center_output = vec![0.0; center_input.len()];
        for (i, &sample) in center_input.iter().enumerate() {
            center_output[i] = filter.process(sample);
        }

        let center_input_rms = compute_rms(&center_input[1000..]);
        let center_output_rms = compute_rms(&center_output[1000..]);
        let center_attenuation = center_output_rms / center_input_rms;

        // Center frequency should pass reasonably well (>0.3)
        assert!(
            center_attenuation > 0.3,
            "Center freq attenuation too high: {}",
            center_attenuation
        );

        // Reset filter for next test
        filter.reset();

        // Test far from cutoff - should attenuate
        let far_freq = 5000.0;
        let far_input = generate_sine(far_freq, sample_rate, 4410);
        let mut far_output = vec![0.0; far_input.len()];
        for (i, &sample) in far_input.iter().enumerate() {
            far_output[i] = filter.process(sample);
        }

        let far_input_rms = compute_rms(&far_input[1000..]);
        let far_output_rms = compute_rms(&far_output[1000..]);
        let far_attenuation = far_output_rms / far_input_rms;

        // Frequencies far from cutoff should be attenuated (<0.3)
        assert!(
            far_attenuation < 0.3,
            "Far freq attenuation too low: {}",
            far_attenuation
        );
    }

    #[test]
    fn test_notch_frequency_response() {
        let sample_rate = 44100.0;
        let cutoff = 1000.0;

        let params = FilterParams {
            cutoff,
            resonance: 2.0, // Narrow notch
            filter_type: FilterType::Notch,
            enabled: true,
        };

        let mut filter = StateVariableFilter::new(params, sample_rate);

        // Test at cutoff frequency - should reject
        let center_freq = 1000.0;
        let center_input = generate_sine(center_freq, sample_rate, 4410);
        let mut center_output = vec![0.0; center_input.len()];
        for (i, &sample) in center_input.iter().enumerate() {
            center_output[i] = filter.process(sample);
        }

        let center_input_rms = compute_rms(&center_input[1000..]);
        let center_output_rms = compute_rms(&center_output[1000..]);
        let center_attenuation = center_output_rms / center_input_rms;

        // Center frequency should be attenuated significantly (<0.3)
        assert!(
            center_attenuation < 0.3,
            "Center freq attenuation too low: {}",
            center_attenuation
        );

        // Reset filter for next test
        filter.reset();

        // Test far from cutoff - should pass
        let far_freq = 5000.0;
        let far_input = generate_sine(far_freq, sample_rate, 4410);
        let mut far_output = vec![0.0; far_input.len()];
        for (i, &sample) in far_input.iter().enumerate() {
            far_output[i] = filter.process(sample);
        }

        let far_input_rms = compute_rms(&far_input[1000..]);
        let far_output_rms = compute_rms(&far_output[1000..]);
        let far_attenuation = far_output_rms / far_input_rms;

        // Frequencies far from cutoff should pass (>0.8)
        assert!(
            far_attenuation > 0.8,
            "Far freq attenuation too high: {}",
            far_attenuation
        );
    }

    #[test]
    fn test_resonance_increases_gain() {
        let sample_rate = 44100.0;
        let cutoff = 1000.0;
        let test_freq = 1000.0; // At cutoff

        // Low resonance
        let low_q_params = FilterParams {
            cutoff,
            resonance: 0.707, // Butterworth
            filter_type: FilterType::LowPass,
            enabled: true,
        };

        let mut low_q_filter = StateVariableFilter::new(low_q_params, sample_rate);
        let input = generate_sine(test_freq, sample_rate, 4410);
        let mut low_q_output = vec![0.0; input.len()];
        for (i, &sample) in input.iter().enumerate() {
            low_q_output[i] = low_q_filter.process(sample);
        }

        let low_q_rms = compute_rms(&low_q_output[1000..]);

        // High resonance
        let high_q_params = FilterParams {
            cutoff,
            resonance: 5.0, // High resonance
            filter_type: FilterType::LowPass,
            enabled: true,
        };

        let mut high_q_filter = StateVariableFilter::new(high_q_params, sample_rate);
        let mut high_q_output = vec![0.0; input.len()];
        for (i, &sample) in input.iter().enumerate() {
            high_q_output[i] = high_q_filter.process(sample);
        }

        let high_q_rms = compute_rms(&high_q_output[1000..]);

        // Higher resonance should produce higher gain at cutoff frequency
        assert!(
            high_q_rms > low_q_rms,
            "High Q RMS ({}) should be > Low Q RMS ({})",
            high_q_rms,
            low_q_rms
        );
    }

    #[test]
    fn test_reset_clears_state() {
        let params = FilterParams::default();
        let mut filter = StateVariableFilter::new(params, 44100.0);

        // Process some samples to build up state
        for _ in 0..100 {
            filter.process(1.0);
        }

        // State should be non-zero
        assert!(filter.low.abs() > 0.01 || filter.band.abs() > 0.01);

        // Reset
        filter.reset();

        // State should be zero
        assert_eq!(filter.low, 0.0);
        assert_eq!(filter.band, 0.0);
    }

    #[test]
    fn test_extreme_cutoff_frequencies() {
        let sample_rate = 44100.0;

        // Very low cutoff (20Hz)
        let low_params = FilterParams {
            cutoff: 20.0,
            resonance: 1.0,
            filter_type: FilterType::LowPass,
            enabled: true,
        };

        let mut low_filter = StateVariableFilter::new(low_params, sample_rate);

        // Should be stable
        for _ in 0..1000 {
            let output = low_filter.process(0.5);
            assert!(output.is_finite());
        }

        // Very high cutoff (near Fs/6 limit)
        let high_params = FilterParams {
            cutoff: 7000.0, // Close to Fs/6 (~7350Hz @ 44.1kHz)
            resonance: 1.0,
            filter_type: FilterType::LowPass,
            enabled: true,
        };

        let mut high_filter = StateVariableFilter::new(high_params, sample_rate);

        // Should be stable
        for _ in 0..1000 {
            let output = high_filter.process(0.5);
            assert!(output.is_finite());
        }
    }

    #[test]
    fn test_parameter_changes() {
        let mut params = FilterParams {
            cutoff: 1000.0,
            resonance: 1.0,
            filter_type: FilterType::LowPass,
            enabled: true,
        };

        let mut filter = StateVariableFilter::new(params, 44100.0);

        // Process some samples
        for _ in 0..100 {
            filter.process(0.5);
        }

        // Change cutoff
        params.cutoff = 2000.0;
        filter.set_params(params);

        // Should still be stable after parameter change
        for _ in 0..100 {
            let output = filter.process(0.5);
            assert!(output.is_finite());
        }

        // Change filter type
        params.filter_type = FilterType::HighPass;
        filter.set_params(params);

        // Should still be stable after type change
        for _ in 0..100 {
            let output = filter.process(0.5);
            assert!(output.is_finite());
        }
    }

    #[test]
    fn test_modulated_vs_normal_process() {
        let params = FilterParams::default();
        let mut filter1 = StateVariableFilter::new(params, 44100.0);
        let mut filter2 = StateVariableFilter::new(params, 44100.0);

        let input = 0.5;
        let cutoff = 1000.0;

        // Both should produce same output when cutoff matches
        let output1 = filter1.process(input);
        let output2 = filter2.process_modulated(input, cutoff);

        // Allow some tolerance due to smoothing differences
        assert!(
            (output1 - output2).abs() < 0.1,
            "Normal process ({}) and modulated process ({}) differ too much",
            output1,
            output2
        );
    }
}
