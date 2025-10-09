// Oscillateurs - Générateurs de formes d'onde

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
    sample_rate: f32,
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
        let sample = match self.waveform {
            WaveformType::Sine => (self.phase * 2.0 * PI).sin(),
            WaveformType::Square => {
                if self.phase < 0.5 { 1.0 } else { -1.0 }
            }
            WaveformType::Saw => (self.phase * 2.0) - 1.0,
            WaveformType::Triangle => {
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

        sample
    }

    fn set_frequency(&mut self, freq: f32) {
        self.phase_increment = freq / self.sample_rate;
    }

    fn reset(&mut self) {
        self.phase = 0.0;
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

        // Les samples doivent être soit 1.0 soit -1.0
        for _ in 0..1000 {
            let sample = osc.next_sample();
            assert!(
                (sample - 1.0).abs() < EPSILON || (sample + 1.0).abs() < EPSILON,
                "Square wave sample not ±1.0: {}",
                sample
            );
        }
    }

    #[test]
    fn test_saw_wave_range() {
        let mut osc = SimpleOscillator::new(WaveformType::Saw, SAMPLE_RATE);
        osc.set_frequency(440.0);

        // Saw wave doit être dans [-1, 1]
        for _ in 0..1000 {
            let sample = osc.next_sample();
            assert!(
                sample >= -1.0 && sample <= 1.0,
                "Saw wave sample out of range: {}",
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
