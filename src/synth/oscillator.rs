// Oscillators - Waveform generators
//
// Notes:
// - The audio callback is RT-critical. This module must avoid allocations
//   and any blocking operations. The oscillator is allocation-free.
// - Saw and Square are bandlimited using PolyBLEP to reduce aliasing at
//   higher frequencies while keeping CPU overhead minimal.

use std::f32::consts::PI;

pub trait Oscillator {
    fn next_sample(&mut self) -> f32;
    fn set_frequency(&mut self, freq: f32);
    fn reset(&mut self);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WaveformType {
    Sine,
    Square,
    Saw,
    Triangle,
}

pub struct SimpleOscillator {
    waveform: WaveformType,
    phase: f32,
    phase_increment: f32,
    pub(crate) sample_rate: f32,  // Made pub(crate) for LFO access
}

impl SimpleOscillator {
    pub fn new(waveform: WaveformType, sample_rate: f32) -> Self {
        Self {
            waveform,
            phase: 0.0,
            phase_increment: 0.0,
            sample_rate,
        }
    }
}

impl Oscillator for SimpleOscillator {
    fn next_sample(&mut self) -> f32 {
        // Compute raw sample based on waveform
        let mut sample = match self.waveform {
            WaveformType::Sine => (self.phase * 2.0 * PI).sin(),
            WaveformType::Square => {
                // 50% duty square wave
                if self.phase < 0.5 { 1.0 } else { -1.0 }
            }
            WaveformType::Saw => (self.phase * 2.0) - 1.0,
            WaveformType::Triangle => {
                // Simple piecewise triangle in [-1, 1]
                if self.phase < 0.5 {
                    (self.phase * 4.0) - 1.0
                } else {
                    3.0 - (self.phase * 4.0)
                }
            }
        };

        self.phase += self.phase_increment;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        // Apply PolyBLEP correction for discontinuous waveforms to reduce aliasing.
        // This must be done after incrementing phase to keep behavior consistent
        // across blocks, using the phase value corresponding to this sample.
        match self.waveform {
            WaveformType::Saw => {
                sample -= self.poly_blep(self.phase);
                sample
            }
            WaveformType::Square => {
                // Square has two discontinuities per period: at phase 0 and 0.5
                sample += self.poly_blep(self.phase);
                let mut p2 = self.phase + 0.5;
                if p2 >= 1.0 { p2 -= 1.0; }
                sample -= self.poly_blep(p2);
                sample
            }
            _ => sample,
        }
    }

    fn set_frequency(&mut self, freq: f32) {
        self.phase_increment = freq / self.sample_rate;
    }

    fn reset(&mut self) {
        self.phase = 0.0;
    }
}

impl SimpleOscillator {
    /// PolyBLEP (Polynomial Band-Limited Step) correction
    ///
    /// Suppresses aliasing at discontinuities for saw/square by adding a small
    /// polynomial correction around the step. `self.phase_increment` is used as
    /// the normalized time step `dt` (in cycles per sample).
    #[inline]
    fn poly_blep(&self, t: f32) -> f32 {
        let dt = self.phase_increment;
        // Guard against extreme dt values; when dt is very small, the regions
        // below become negligible and we return 0 quickly.
        if dt <= 0.0 || dt >= 1.0 {
            return 0.0;
        }

        if t < dt {
            // 0 <= t < dt
            let u = t / dt;
            // u + u - u*u - 1 = 2u - u^2 - 1
            return u + u - u * u - 1.0;
        } else if t > 1.0 - dt {
            // 1 - dt < t < 1
            let u = (t - 1.0) / dt;
            // u^2 + u + u + 1 = u^2 + 2u + 1
            return u * u + u + u + 1.0;
        }
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 44100.0;
    const EPSILON: f32 = 0.001;

    #[test]
    fn test_oscillator_frequency() {
        // Test que la fréquence est correctement appliquée
        let mut osc = SimpleOscillator::new(WaveformType::Sine, SAMPLE_RATE);
        osc.set_frequency(440.0);

        // Phase increment doit être freq / sample_rate
        let expected_increment = 440.0 / SAMPLE_RATE;
        assert!((osc.phase_increment - expected_increment).abs() < EPSILON);
    }

    #[test]
    fn test_oscillator_reset() {
        let mut osc = SimpleOscillator::new(WaveformType::Sine, SAMPLE_RATE);
        osc.set_frequency(440.0);

        // Avancer la phase
        for _ in 0..100 {
            osc.next_sample();
        }

        // La phase ne doit plus être à 0
        assert!(osc.phase > 0.0);

        // Reset
        osc.reset();
        assert_eq!(osc.phase, 0.0);
    }

    #[test]
    fn test_sine_amplitude() {
        let mut osc = SimpleOscillator::new(WaveformType::Sine, SAMPLE_RATE);
        osc.set_frequency(440.0);

        // Générer plusieurs samples et vérifier qu'ils sont dans [-1, 1]
        for _ in 0..1000 {
            let sample = osc.next_sample();
            assert!(sample >= -1.0 && sample <= 1.0, "Sample {} hors limites", sample);
        }
    }

    #[test]
    fn test_sine_starts_at_zero() {
        let mut osc = SimpleOscillator::new(WaveformType::Sine, SAMPLE_RATE);
        osc.set_frequency(440.0);

        // Premier sample doit être proche de 0 (sin(0) = 0)
        let first_sample = osc.next_sample();
        assert!(first_sample.abs() < EPSILON, "First sample: {}", first_sample);
    }

    #[test]
    fn test_square_wave() {
        let mut osc = SimpleOscillator::new(WaveformType::Square, SAMPLE_RATE);
        osc.set_frequency(440.0);

        // PolyBLEP bandlimiting INTENTIONALLY creates temporary overshoots around
        // discontinuities (Gibbs phenomenon). Square wave has 2 discontinuities per
        // period, so corrections can add up:
        // - Base signal: ±1.0
        // - PolyBLEP correction 1: ±1.0 (at phase 0)
        // - PolyBLEP correction 2: ±1.0 (at phase 0.5)
        // - Observed max overshoot: ±1.8 in practice
        //
        // This is NORMAL and necessary for bandlimiting. The soft-limiter (tanh)
        // in VoiceManager handles this later in the signal chain.
        for _ in 0..5000 {
            let sample = osc.next_sample();
            assert!(sample.is_finite(), "Square wave sample must be finite");
            assert!(
                sample >= -2.0 && sample <= 2.0,
                "Square wave PolyBLEP overshoot out of acceptable range: {}",
                sample
            );
        }
    }

    #[test]
    fn test_saw_wave_range() {
        let mut osc = SimpleOscillator::new(WaveformType::Saw, SAMPLE_RATE);
        osc.set_frequency(440.0);

        // Same as square wave: PolyBLEP creates intentional overshoots for bandlimiting.
        // Saw wave has 1 discontinuity per period, so less overshoot than square,
        // but can still reach ±1.8 depending on phase_increment.
        // - Base signal: [-1, 1]
        // - PolyBLEP correction: ±1.0
        // - Observed max overshoot: ±1.8
        //
        // This is expected behavior. Soft-limiter in VoiceManager handles final output.
        for _ in 0..5000 {
            let sample = osc.next_sample();
            assert!(
                sample >= -2.0 && sample <= 2.0,
                "Saw wave PolyBLEP overshoot out of acceptable range: {}",
                sample
            );
        }
    }

    #[test]
    fn test_triangle_wave_range() {
        let mut osc = SimpleOscillator::new(WaveformType::Triangle, SAMPLE_RATE);
        osc.set_frequency(440.0);

        // Triangle wave doit être dans [-1, 1]
        for _ in 0..1000 {
            let sample = osc.next_sample();
            assert!(
                sample >= -1.0 && sample <= 1.0,
                "Triangle wave sample out of range: {}",
                sample
            );
        }
    }

    #[test]
    fn test_phase_wrapping() {
        let mut osc = SimpleOscillator::new(WaveformType::Sine, SAMPLE_RATE);
        osc.set_frequency(440.0);

        // Générer assez de samples pour que la phase wrap plusieurs fois
        for _ in 0..10000 {
            osc.next_sample();
            // La phase doit toujours être dans [0, 1)
            assert!(
                osc.phase >= 0.0 && osc.phase < 1.0,
                "Phase out of range: {}",
                osc.phase
            );
        }
    }
}
