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
