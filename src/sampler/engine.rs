use std::sync::Arc;
use crate::synth::envelope::{AdsrEnvelope, AdsrParams};
use std::f32::consts::FRAC_PI_2;

pub struct SamplerVoice {
    sample_data: Arc<Vec<f32>>,
    position: f64,
    pitch_step: f64,
    is_active: bool,
    note: u8,
    velocity: f32,
    age: u64,
    envelope: AdsrEnvelope,
    pan: f32, // Pan, from -1.0 (left) to 1.0 (right)
}

impl SamplerVoice {
    pub fn new(sample_data: Arc<Vec<f32>>, sample_rate: f32) -> Self {
        Self {
            sample_data,
            position: 0.0,
            pitch_step: 1.0,
            is_active: false,
            note: 0,
            velocity: 0.0,
            age: 0,
            envelope: AdsrEnvelope::new(AdsrParams::default(), sample_rate),
            pan: 0.0, // Center pan
        }
    }

    pub fn note_on(&mut self, note: u8, velocity: u8, age: u64) {
        const BASE_NOTE: f64 = 60.0; // C4
        let semitones_from_base = note as f64 - BASE_NOTE;
        self.pitch_step = 2.0_f64.powf(semitones_from_base / 12.0);

        self.note = note;
        self.velocity = velocity as f32 / 127.0;
        self.age = age;
        self.position = 0.0;
        self.is_active = true;
        self.envelope.note_on();
    }

    pub fn note_off(&mut self) {
        self.envelope.note_off();
    }

    pub fn force_stop(&mut self) {
        self.is_active = false;
        self.envelope.reset();
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn get_note(&self) -> u8 {
        self.note
    }

    pub fn get_age(&self) -> u64 {
        self.age
    }

    pub fn is_releasing(&self) -> bool {
        // Sampler doesn't have a distinct release phase in this simple model
        // It's active until the envelope is done.
        !self.is_active && self.envelope.is_active()
    }

    pub fn change_pitch_legato(&mut self, note: u8, velocity: u8, age: u64) {
        const BASE_NOTE: f64 = 60.0; // C4
        let semitones_from_base = note as f64 - BASE_NOTE;
        self.pitch_step = 2.0_f64.powf(semitones_from_base / 12.0);
        self.note = note;
        self.velocity = velocity as f32 / 127.0;
        self.age = age;
    }

    // Dummy method for API compatibility
    pub fn set_aftertouch(&mut self, _value: f32) {}

    /// Returns a stereo sample `(left, right)`
    pub fn next_sample_with_matrix(&mut self, _matrix: &crate::synth::modulation::ModulationMatrix) -> (f32, f32) {
        if !self.is_active {
            return (0.0, 0.0);
        }

        let pos_integer = self.position as usize;
        let pos_fractional = self.position.fract();

        let sample1 = self.sample_data.get(pos_integer).copied().unwrap_or(0.0);
        let sample2 = self.sample_data.get(pos_integer + 1).copied().unwrap_or(0.0);

        let mut sample = sample1 + (sample2 - sample1) * pos_fractional as f32;

        self.position += self.pitch_step;

        if self.position >= self.sample_data.len() as f64 {
            self.is_active = false;
            self.position = 0.0;
            return (0.0, 0.0); // Return silence on the sample that finishes
        }

        let envelope_value = self.envelope.process();
        if !self.envelope.is_active() {
            self.is_active = false;
        }

        sample *= self.velocity * envelope_value;

        let angle = (self.pan.clamp(-1.0, 1.0) * 0.5 + 0.5) * FRAC_PI_2;
        let left = sample * angle.cos();
        let right = sample * angle.sin();

        (left, right)
    }
}
