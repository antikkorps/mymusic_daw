// Voice - Une note jouée

use super::oscillator::{Oscillator, SimpleOscillator, WaveformType};

pub struct Voice {
    oscillator: SimpleOscillator,
    note: u8,
    velocity: f32,
    active: bool,
    waveform: WaveformType,
    sample_rate: f32,
}

impl Voice {
    pub fn new(sample_rate: f32) -> Self {
        let waveform = WaveformType::Sine;
        Self {
            oscillator: SimpleOscillator::new(waveform, sample_rate),
            note: 0,
            velocity: 0.0,
            active: false,
            waveform,
            sample_rate,
        }
    }

    pub fn note_on(&mut self, note: u8, velocity: u8) {
        self.note = note;
        self.velocity = velocity as f32 / 127.0;
        self.active = true;

        // Convert MIDI note to fréquency: 440 * 2^((note - 69) / 12)
        let frequency = 440.0 * 2_f32.powf((note as f32 - 69.0) / 12.0);
        self.oscillator.set_frequency(frequency);
        self.oscillator.reset();
    }

    pub fn note_off(&mut self) {
        self.active = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn get_note(&self) -> u8 {
        self.note
    }

    pub fn set_waveform(&mut self, waveform: WaveformType) {
        self.waveform = waveform;
        // Recréer l'oscillateur avec la nouvelle forme d'onde
        self.oscillator = SimpleOscillator::new(waveform, self.sample_rate);
        // Restaurer la fréquence si une note est active
        if self.active {
            let frequency = 440.0 * 2_f32.powf((self.note as f32 - 69.0) / 12.0);
            self.oscillator.set_frequency(frequency);
        }
    }

    pub fn next_sample(&mut self) -> f32 {
        if self.active {
            self.oscillator.next_sample() * self.velocity
        } else {
            0.0
        }
    }
}
