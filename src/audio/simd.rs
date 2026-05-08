//! SIMD-optimized audio processing utilities
//!
//! This module provides SIMD-accelerated versions of common DSP operations
//! using `wide` crate for cross-platform SIMD support.

use wide::*;

/// SIMD-optimized sine wave generator
pub struct SimdOscillator {
    /// Current phase values (4-way SIMD)
    phase: f32x4,
    /// Phase increment values (4-way SIMD)
    phase_increment: f32x4,
    /// Frequency values (4-way SIMD)
    frequency: f32x4,
    /// Sample rate
    sample_rate: f32,
}

impl SimdOscillator {
    /// Create a new SIMD oscillator with initial frequency
    pub fn new(frequency: f32, sample_rate: f32) -> Self {
        let phase_increment = f32x4::from([frequency; 4]) / f32x4::from([sample_rate; 4]);
        Self {
            phase: f32x4::ZERO,
            phase_increment,
            frequency: f32x4::from([frequency; 4]),
            sample_rate,
        }
    }

    /// Set frequency for all voices
    pub fn set_frequency(&mut self, frequency: f32) {
        self.frequency = f32x4::from([frequency; 4]);
        self.phase_increment = self.frequency / f32x4::from([self.sample_rate; 4]);
    }

    /// Set individual frequencies for 4 voices
    pub fn set_frequencies(&mut self, freqs: [f32; 4]) {
        self.frequency = f32x4::from(freqs);
        self.phase_increment = self.frequency / f32x4::from([self.sample_rate; 4]);
    }

    /// Reset the phase of a single lane to 0 (e.g. on note_on).
    #[inline]
    pub fn reset_lane(&mut self, lane: usize) {
        debug_assert!(lane < 4);
        let mut arr: [f32; 4] = self.phase.into();
        arr[lane] = 0.0;
        self.phase = f32x4::from(arr);
    }

    /// Reset all 4 lane phases to 0.
    #[inline]
    pub fn reset_all(&mut self) {
        self.phase = f32x4::ZERO;
    }

    /// Generate 4 sine samples simultaneously (SIMD).
    pub fn next_samples(&mut self) -> [f32; 4] {
        // sin(2π * phase) using OLD phase to match the scalar oscillator
        let samples = (self.phase * f32x4::from([std::f32::consts::PI * 2.0; 4])).sin();

        // Advance phase and wrap to [0, 1)
        self.phase += self.phase_increment;
        self.phase = self.phase - self.phase.floor();

        samples.into()
    }

    /// PolyBLEP correction (Polynomial Band-Limited Step) — SIMD version.
    ///
    /// Mirrors `SimpleOscillator::poly_blep` (synth/oscillator.rs:106) lane-wise:
    /// - if t < dt:        2u - u² - 1   where u = t / dt
    /// - if t > 1 - dt:    u² + 2u + 1   where u = (t - 1) / dt
    /// - otherwise:        0
    ///
    /// Both branches are evaluated unconditionally (SIMD has no early exit) and
    /// then selected via `blend`. Lanes outside both regions get 0.
    #[inline]
    fn poly_blep_simd(t: f32x4, dt: f32x4) -> f32x4 {
        let one = f32x4::from([1.0; 4]);

        // Branch 1: t < dt → 2u − u² − 1, u = t/dt
        let u_lo = t / dt;
        let blep_lo = u_lo + u_lo - u_lo * u_lo - one;

        // Branch 2: t > 1 - dt → u² + 2u + 1, u = (t−1)/dt
        let u_hi = (t - one) / dt;
        let blep_hi = u_hi * u_hi + u_hi + u_hi + one;

        // Build result: start with 0, blend in blep_lo where t<dt, blep_hi where t>1-dt
        let lo_mask = t.cmp_lt(dt);
        let hi_mask = t.cmp_gt(one - dt);
        let with_lo = lo_mask.blend(blep_lo, f32x4::ZERO);
        hi_mask.blend(blep_hi, with_lo)
    }

    /// Generate 4 sawtooth samples with PolyBLEP anti-aliasing (matches scalar
    /// `SimpleOscillator` for `WaveformType::Saw`).
    pub fn next_saw_polyblep(&mut self) -> [f32; 4] {
        let two = f32x4::from([2.0; 4]);
        let one = f32x4::from([1.0; 4]);

        // Naive saw using OLD phase: 2*phase − 1
        let saw = self.phase * two - one;

        // Advance phase and wrap (matches scalar order: PolyBLEP uses NEW phase)
        self.phase += self.phase_increment;
        self.phase = self.phase - self.phase.floor();

        // Subtract PolyBLEP at the falling edge (phase 0 / 1)
        let blep = Self::poly_blep_simd(self.phase, self.phase_increment);
        let result = saw - blep;

        result.into()
    }

    /// Generate 4 square samples with PolyBLEP anti-aliasing (matches scalar
    /// `SimpleOscillator` for `WaveformType::Square`).
    pub fn next_square_polyblep(&mut self) -> [f32; 4] {
        let half = f32x4::from([0.5; 4]);
        let one = f32x4::from([1.0; 4]);
        let neg_one = f32x4::from([-1.0; 4]);

        // Naive square using OLD phase: +1 if phase < 0.5 else −1
        let mask_lt_half = self.phase.cmp_lt(half);
        let square = mask_lt_half.blend(one, neg_one);

        // Advance phase and wrap
        self.phase += self.phase_increment;
        self.phase = self.phase - self.phase.floor();

        // Two corrections per period: + at phase 0, − at phase 0.5
        let blep1 = Self::poly_blep_simd(self.phase, self.phase_increment);
        let mut p2 = self.phase + half;
        p2 = p2 - p2.floor();
        let blep2 = Self::poly_blep_simd(p2, self.phase_increment);

        let result = square + blep1 - blep2;
        result.into()
    }

    /// Generate 4 triangle samples (matches scalar `SimpleOscillator`).
    ///
    /// Scalar formula: `if p < 0.5 { 4p − 1 } else { 3 − 4p }`
    /// Equivalent SIMD-friendly form: `1 − 2 * |2p − 1|`
    /// (verified: p=0 → −1, p=0.5 → +1, p=1 → −1).
    pub fn next_triangle(&mut self) -> [f32; 4] {
        let two = f32x4::from([2.0; 4]);
        let one = f32x4::from([1.0; 4]);

        let triangle = one - two * (two * self.phase - one).abs();

        self.phase += self.phase_increment;
        self.phase = self.phase - self.phase.floor();

        triangle.into()
    }
}

/// SIMD-optimized gain staging for multiple voices
pub fn simd_gain_stage_voices(voices: &mut [[f32; 2]; 4], active_count: usize) {
    if active_count == 0 {
        return;
    }

    // Calculate dynamic gain: 1.0 / sqrt(active_voices)
    let gain = 1.0 / (active_count as f32).sqrt();
    let gain_simd = f32x4::from([gain; 4]);

    // Apply gain to all voices
    for voice in voices.iter_mut().take(active_count) {
        let left_simd = f32x4::from([voice[0]; 4]);
        let right_simd = f32x4::from([voice[1]; 4]);

        let left_gained = left_simd * gain_simd;
        let right_gained = right_simd * gain_simd;

        let left_array: [f32; 4] = left_gained.into();
        let right_array: [f32; 4] = right_gained.into();

        voice[0] = left_array[0];
        voice[1] = right_array[0];
    }
}

/// SIMD-optimized soft clipping
pub fn simd_soft_clip(samples: &mut [f32]) {
    const CHUNK_SIZE: usize = 4;

    // Process in chunks to avoid borrowing issues
    for chunk_start in (0..samples.len()).step_by(CHUNK_SIZE) {
        let end = (chunk_start + CHUNK_SIZE).min(samples.len());
        if end - chunk_start == CHUNK_SIZE {
            // Process full chunk with SIMD
            let chunk_array: [f32; 4] = samples[chunk_start..end].try_into().unwrap_or([0.0; 4]);
            let simd_samples = f32x4::from(chunk_array);

            // Manual tanh approximation using SIMD
            let x = simd_samples;
            let x2 = x * x;
            let a = x * (135135.0 + x2 * (27.0 + x2));
            let b = 135135.0 + x2 * (45.0 + x2);
            let clipped = a / b;

            let clipped_array: [f32; 4] = clipped.into();
            samples[chunk_start..end].copy_from_slice(&clipped_array);
        } else {
            // Process remaining samples scalar
            for i in chunk_start..end {
                samples[i] = samples[i].tanh();
            }
        }
    }
}

/// SIMD-optimized denormal flushing
pub fn simd_flush_denormals(samples: &mut [f32]) {
    const CHUNK_SIZE: usize = 4;
    // Match the scalar threshold in `dsp_utils::flush_denormals_to_zero`.
    const DENORMAL_THRESHOLD: f32 = 1e-15;

    // Process in chunks to avoid borrowing issues
    for chunk_start in (0..samples.len()).step_by(CHUNK_SIZE) {
        let end = (chunk_start + CHUNK_SIZE).min(samples.len());
        if end - chunk_start == CHUNK_SIZE {
            // Process full chunk with SIMD
            let chunk_array: [f32; 4] = samples[chunk_start..end].try_into().unwrap_or([0.0; 4]);
            let simd_samples = f32x4::from(chunk_array);
            let threshold = f32x4::from([DENORMAL_THRESHOLD; 4]);
            // mask is true where |sample| < threshold (denormal) → flush to 0
            let flushed = simd_samples
                .abs()
                .cmp_lt(threshold)
                .blend(f32x4::ZERO, simd_samples);
            let flushed_array: [f32; 4] = flushed.into();
            samples[chunk_start..end].copy_from_slice(&flushed_array);
        } else {
            // Process remaining samples scalar
            for i in chunk_start..end {
                if samples[i].abs() < DENORMAL_THRESHOLD {
                    samples[i] = 0.0;
                }
            }
        }
    }
}

/// SIMD-optimized stereo mixing
pub fn simd_mix_stereo(left: &[f32], right: &[f32], gain: f32) -> Vec<f32> {
    assert_eq!(left.len(), right.len());

    let mut output = vec![0.0f32; left.len() * 2];
    let gain_simd = f32x4::from([gain; 4]);

    // Process 2 stereo samples (4 mono samples) at a time
    for (i, output_chunk) in output.chunks_exact_mut(4).enumerate() {
        let left_idx = i * 2;
        let right_idx = i * 2;

        if left_idx + 1 < left.len() && right_idx + 1 < right.len() {
            let left_simd = f32x4::from([left[left_idx], left[left_idx + 1], 0.0, 0.0]);
            let right_simd = f32x4::from([right[right_idx], right[right_idx + 1], 0.0, 0.0]);

            let left_gained = left_simd * gain_simd;
            let right_gained = right_simd * gain_simd;

            // Interleave: L0, R0, L1, R1
            let left_array: [f32; 4] = left_gained.into();
            let right_array: [f32; 4] = right_gained.into();

            output_chunk[0] = left_array[0];
            output_chunk[1] = right_array[0];
            output_chunk[2] = left_array[1];
            output_chunk[3] = right_array[1];
        }
    }

    output
}

/// SIMD-optimized State Variable Filter (Chamberlin)
///
/// Processes 4 filters simultaneously using SIMD for improved performance.
/// Each lane represents an independent filter instance.
pub struct SimdStateVariableFilter {
    /// Low-pass state for 4 filters
    low: f32x4,
    /// Band-pass state for 4 filters
    band: f32x4,
    /// Frequency coefficients for 4 filters
    f: f32x4,
    /// Resonance coefficients for 4 filters
    q: f32x4,
    /// Sample rate
    sample_rate: f32,
}

impl SimdStateVariableFilter {
    /// Create a new SIMD filter with identical parameters for all 4 lanes
    pub fn new(cutoff: f32, resonance: f32, sample_rate: f32) -> Self {
        let f = Self::compute_f(cutoff, sample_rate);
        let q = Self::compute_q(resonance);

        Self {
            low: f32x4::ZERO,
            band: f32x4::ZERO,
            f: f32x4::from([f; 4]),
            q: f32x4::from([q; 4]),
            sample_rate,
        }
    }

    /// Create a new SIMD filter with different parameters for each lane
    pub fn new_multi(cutoffs: [f32; 4], resonances: [f32; 4], sample_rate: f32) -> Self {
        let f_values: [f32; 4] = [
            Self::compute_f(cutoffs[0], sample_rate),
            Self::compute_f(cutoffs[1], sample_rate),
            Self::compute_f(cutoffs[2], sample_rate),
            Self::compute_f(cutoffs[3], sample_rate),
        ];

        let q_values: [f32; 4] = [
            Self::compute_q(resonances[0]),
            Self::compute_q(resonances[1]),
            Self::compute_q(resonances[2]),
            Self::compute_q(resonances[3]),
        ];

        Self {
            low: f32x4::ZERO,
            band: f32x4::ZERO,
            f: f32x4::from(f_values),
            q: f32x4::from(q_values),
            sample_rate,
        }
    }

    /// Compute frequency coefficient: f = 2 * sin(π * fc / Fs)
    #[inline]
    fn compute_f(cutoff: f32, sample_rate: f32) -> f32 {
        let max_cutoff = sample_rate / 6.0;
        let safe_cutoff = cutoff.clamp(20.0, max_cutoff);
        2.0 * (std::f32::consts::PI * safe_cutoff / sample_rate).sin()
    }

    /// Compute resonance coefficient: q = 1 / Q
    #[inline]
    fn compute_q(resonance: f32) -> f32 {
        let q_factor = resonance.clamp(0.5, 20.0);
        (1.0 / q_factor).clamp(0.01, 2.0)
    }

    /// Update frequency coefficients for all lanes
    pub fn set_cutoff(&mut self, cutoff: f32) {
        let f = Self::compute_f(cutoff, self.sample_rate);
        self.f = f32x4::from([f; 4]);
    }

    /// Update frequency coefficients for each lane individually
    pub fn set_cutoffs(&mut self, cutoffs: [f32; 4]) {
        let f_values: [f32; 4] = [
            Self::compute_f(cutoffs[0], self.sample_rate),
            Self::compute_f(cutoffs[1], self.sample_rate),
            Self::compute_f(cutoffs[2], self.sample_rate),
            Self::compute_f(cutoffs[3], self.sample_rate),
        ];
        self.f = f32x4::from(f_values);
    }

    /// Update resonance coefficients for all lanes
    pub fn set_resonance(&mut self, resonance: f32) {
        let q = Self::compute_q(resonance);
        self.q = f32x4::from([q; 4]);
    }

    /// Update resonance coefficients for each lane individually
    pub fn set_resonances(&mut self, resonances: [f32; 4]) {
        let q_values: [f32; 4] = [
            Self::compute_q(resonances[0]),
            Self::compute_q(resonances[1]),
            Self::compute_q(resonances[2]),
            Self::compute_q(resonances[3]),
        ];
        self.q = f32x4::from(q_values);
    }

    /// Reset filter state (clear delay lines)
    pub fn reset(&mut self) {
        self.low = f32x4::ZERO;
        self.band = f32x4::ZERO;
    }

    /// Process 4 samples simultaneously (one per filter lane)
    ///
    /// # Arguments
    /// * `inputs` - Input samples for each of the 4 filters
    /// * `filter_type` - Filter type to apply (same for all lanes)
    ///
    /// # Returns
    /// Filtered outputs for each of the 4 filters
    #[inline]
    pub fn process(&mut self, inputs: [f32; 4], filter_type: FilterType) -> [f32; 4] {
        let input_simd = f32x4::from(inputs);

        // Chamberlin State Variable Filter algorithm (SIMD version)
        // Compute high-pass: hp = input - low - q*band
        let high = input_simd - self.low - self.q * self.band;

        // Update band-pass: band = band + f*hp
        self.band += self.f * high;

        // Update low-pass: low = low + f*band
        self.low += self.f * self.band;

        // Compute notch: notch = input - q*band
        let notch = input_simd - self.q * self.band;

        // Select output based on filter type
        let output = match filter_type {
            FilterType::LowPass => self.low,
            FilterType::HighPass => high,
            FilterType::BandPass => self.band,
            FilterType::Notch => notch,
        };

        output.into()
    }
}

/// Filter type for SIMD filter (same as scalar version)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterType {
    LowPass,
    HighPass,
    BandPass,
    Notch,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_oscillator_basic() {
        let mut osc = SimdOscillator::new(440.0, 44100.0);

        // Generate a few samples
        let samples1 = osc.next_samples();
        let samples2 = osc.next_samples();

        // Check that we get different values
        assert_ne!(samples1, samples2);

        // Check that all voices have the same frequency initially
        for i in 1..4 {
            assert_eq!(samples1[i], samples1[0]);
        }
    }

    #[test]
    fn test_simd_oscillator_frequency_change() {
        let mut osc = SimdOscillator::new(440.0, 44100.0);

        // Change frequency
        osc.set_frequency(880.0);

        // Generate samples
        let samples = osc.next_samples();

        // All voices should have the new frequency
        for i in 1..4 {
            assert_eq!(samples[i], samples[0]);
        }
    }

    #[test]
    fn test_simd_oscillator_individual_frequencies() {
        let mut osc = SimdOscillator::new(440.0, 44100.0);
        osc.set_frequencies([220.0, 440.0, 880.0, 1760.0]);

        // First sample is always 0 (sin(0)=0 in all lanes). Advance enough samples
        // so that the per-lane phase differences become audible.
        for _ in 0..100 {
            osc.next_samples();
        }

        let samples = osc.next_samples();
        let all_same = samples.iter().all(|&s| (s - samples[0]).abs() < 1e-6);
        assert!(
            !all_same,
            "Samples should differ across lanes when frequencies differ: {:?}",
            samples
        );
    }

    #[test]
    fn test_simd_gain_staging() {
        let mut voices = [[1.0, 1.0]; 4];

        // Test with 4 active voices
        simd_gain_stage_voices(&mut voices, 4);
        let expected_gain = 1.0 / (4.0_f32).sqrt();
        assert!((voices[0][0] - expected_gain).abs() < 1e-6);

        // Reset and test with 2 active voices
        voices = [[1.0, 1.0]; 4];
        simd_gain_stage_voices(&mut voices, 2);
        let expected_gain = 1.0 / (2.0_f32).sqrt();
        assert!((voices[0][0] - expected_gain).abs() < 1e-6);
    }

    #[test]
    fn test_simd_soft_clip() {
        let mut samples = vec![-2.0, -0.5, 0.5, 2.0, 0.0];
        let original = samples.clone();

        simd_soft_clip(&mut samples);

        // Values should be clamped by tanh
        assert!(samples[0] > original[0]); // -2.0 should be increased
        assert!(samples[1] > original[1]); // -0.5 should be slightly increased
        assert!(samples[2] < original[2]); // 0.5 should be slightly decreased
        assert!(samples[3] < original[3]); // 2.0 should be decreased
        assert_eq!(samples[4], original[4]); // 0.0 should remain unchanged
    }

    #[test]
    fn test_simd_flush_denormals() {
        // Threshold is 1e-15 (matches scalar `dsp_utils::flush_denormals_to_zero`).
        // Values strictly below are flushed; values >= are kept.
        let mut samples = [1e-30, -1e-20, 1e-8, 0.1];

        simd_flush_denormals(&mut samples);

        // True denormal-range values are flushed to zero
        assert_eq!(samples[0], 0.0, "1e-30 should be flushed");
        assert_eq!(samples[1], 0.0, "-1e-20 should be flushed");
        // 1e-8 is well above the threshold and is normal audio territory: keep it
        assert!((samples[2] - 1e-8).abs() < 1e-12);
        // Normal value unchanged
        assert!((samples[3] - 0.1).abs() < 1e-6);
    }

    #[test]
    fn test_simd_mix_stereo() {
        let left = [0.5, -0.5, 0.25, -0.25];
        let right = [0.3, -0.3, 0.15, -0.15];
        let gain = 0.8;

        let output = simd_mix_stereo(&left, &right, gain);

        // Check interleaving and gain application
        assert_eq!(output[0], 0.5 * gain); // L0
        assert_eq!(output[1], 0.3 * gain); // R0
        assert_eq!(output[2], -0.5 * gain); // L1
        assert_eq!(output[3], -0.3 * gain); // R1
    }

    // -----------------------------------------------------------------------
    // Parity tests: SIMD oscillator output vs scalar SimpleOscillator.
    //
    // The SIMD lanes share a single oscillator state. With identical phase
    // and frequency we expect bit-identical (up to f32 rounding) output to
    // `SimpleOscillator` for every waveform.
    // -----------------------------------------------------------------------
    use crate::synth::oscillator::{Oscillator, SimpleOscillator, WaveformType};

    const PARITY_SR: f32 = 44100.0;
    const PARITY_FREQ: f32 = 440.0;
    const PARITY_SAMPLES: usize = 1024;
    /// Slack allowed between scalar and SIMD output. f32x4::sin and f32::sin
    /// are different implementations (`wide` vs std); the spec from `wide`
    /// guarantees only ~1e-6 absolute accuracy. PolyBLEP-corrected waveforms
    /// inherit any sin error indirectly via the saw/square base.
    const PARITY_TOL: f32 = 1e-3;

    fn collect_scalar(waveform: WaveformType) -> Vec<f32> {
        let mut osc = SimpleOscillator::new(waveform, PARITY_SR);
        osc.set_frequency(PARITY_FREQ);
        (0..PARITY_SAMPLES).map(|_| osc.next_sample()).collect()
    }

    fn collect_simd<F>(mut next: F) -> Vec<f32>
    where
        F: FnMut(&mut SimdOscillator) -> [f32; 4],
    {
        let mut osc = SimdOscillator::new(PARITY_FREQ, PARITY_SR);
        let mut out = Vec::with_capacity(PARITY_SAMPLES);
        for _ in 0..PARITY_SAMPLES {
            // All 4 lanes share frequency and start phase, so any lane is fine
            // — pick lane 0 for the comparison.
            out.push(next(&mut osc)[0]);
        }
        out
    }

    fn assert_parity(name: &str, scalar: &[f32], simd: &[f32], tol: f32) {
        assert_eq!(scalar.len(), simd.len());
        let mut max_diff = 0.0f32;
        let mut argmax = 0usize;
        for (i, (a, b)) in scalar.iter().zip(simd.iter()).enumerate() {
            let d = (a - b).abs();
            if d > max_diff {
                max_diff = d;
                argmax = i;
            }
        }
        assert!(
            max_diff < tol,
            "{name}: max divergence {max_diff} (tol {tol}) at sample {argmax}: scalar={} simd={}",
            scalar[argmax],
            simd[argmax]
        );
    }

    #[test]
    fn test_simd_parity_sine() {
        let scalar = collect_scalar(WaveformType::Sine);
        let simd = collect_simd(|o| o.next_samples());
        assert_parity("sine", &scalar, &simd, PARITY_TOL);
    }

    #[test]
    fn test_simd_parity_triangle() {
        let scalar = collect_scalar(WaveformType::Triangle);
        let simd = collect_simd(|o| o.next_triangle());
        assert_parity("triangle", &scalar, &simd, PARITY_TOL);
    }

    #[test]
    fn test_simd_parity_saw_polyblep() {
        let scalar = collect_scalar(WaveformType::Saw);
        let simd = collect_simd(|o| o.next_saw_polyblep());
        assert_parity("saw+polyblep", &scalar, &simd, PARITY_TOL);
    }

    #[test]
    fn test_simd_parity_square_polyblep() {
        let scalar = collect_scalar(WaveformType::Square);
        let simd = collect_simd(|o| o.next_square_polyblep());
        assert_parity("square+polyblep", &scalar, &simd, PARITY_TOL);
    }

    #[test]
    fn test_simd_oscillator_reset_lane() {
        let mut osc = SimdOscillator::new(440.0, 44100.0);
        // Advance phase
        for _ in 0..100 {
            osc.next_samples();
        }
        // Reset lane 2 only
        osc.reset_lane(2);
        let phases: [f32; 4] = osc.phase.into();
        assert_eq!(phases[2], 0.0, "Lane 2 should be reset to 0");
        assert!(phases[0] > 0.0, "Other lanes should keep their phase");
        assert!(phases[1] > 0.0);
        assert!(phases[3] > 0.0);
    }
}
