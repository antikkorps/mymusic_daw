// Voice - Une note jouée

use super::envelope::{AdsrEnvelope, AdsrParams};
use super::oscillator::{Oscillator, SimpleOscillator, WaveformType};

pub struct Voice {
    oscillator: SimpleOscillator,
    envelope: AdsrEnvelope,
    note: u8,
    velocity: f32,
    active: bool,
    waveform: WaveformType,
    sample_rate: f32,
    /// Age counter for voice stealing priority (higher = older)
    age: u64,
}

impl Voice {
    pub fn new(sample_rate: f32) -> Self {
        let waveform = WaveformType::Sine;
        let adsr_params = AdsrParams::default();
        Self {
            oscillator: SimpleOscillator::new(waveform, sample_rate),
            envelope: AdsrEnvelope::new(adsr_params, sample_rate),
            note: 0,
            velocity: 0.0,
            active: false,
            waveform,
            sample_rate,
            age: 0,
        }
    }

    pub fn note_on(&mut self, note: u8, velocity: u8, age: u64) {
        self.note = note;
        self.velocity = velocity as f32 / 127.0;
        self.active = true;
        self.age = age;

        // Convert MIDI note to fréquency: 440 * 2^((note - 69) / 12)
        let frequency = 440.0 * 2_f32.powf((note as f32 - 69.0) / 12.0);
        self.oscillator.set_frequency(frequency);
        self.oscillator.reset();

        // Trigger ADSR envelope
        self.envelope.note_on();
    }

    pub fn note_off(&mut self) {
        self.active = false;
        // Trigger envelope release
        self.envelope.note_off();
    }

    pub fn is_active(&self) -> bool {
        // Voice is active if envelope is still running (even during release)
        self.envelope.is_active()
    }

    pub fn get_note(&self) -> u8 {
        self.note
    }

    pub fn get_age(&self) -> u64 {
        self.age
    }

    pub fn get_velocity(&self) -> f32 {
        self.velocity
    }

    /// Check if the voice is in release phase (note off but still sounding)
    pub fn is_releasing(&self) -> bool {
        !self.active && self.envelope.is_active()
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

    pub fn set_adsr(&mut self, params: AdsrParams) {
        self.envelope.set_params(params);
    }

    pub fn next_sample(&mut self) -> f32 {
        // Process envelope
        let envelope_value = self.envelope.process();

        // Generate oscillator sample and apply envelope and velocity
        let sample = self.oscillator.next_sample() * self.velocity * envelope_value;

        sample
    }
}
