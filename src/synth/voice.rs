// Voice - Une note jouée

use super::envelope::{AdsrEnvelope, AdsrParams};
use super::lfo::{Lfo, LfoParams};
use super::modulation::ModulationMatrix;
use super::oscillator::{Oscillator, SimpleOscillator, WaveformType};
use super::portamento::{PortamentoGlide, PortamentoParams};
use std::f32::consts::FRAC_PI_2;

pub struct Voice {
    oscillator: SimpleOscillator,
    envelope: AdsrEnvelope,
    lfo: Lfo,
    portamento: PortamentoGlide,
    note: u8,
    velocity: f32,
    aftertouch: f32,
    active: bool,
    waveform: WaveformType,
    sample_rate: f32,
    /// Pan, from -1.0 (left) to 1.0 (right)
    pan: f32,
    /// Age counter for voice stealing priority (higher = older)
    age: u64,
    /// Base frequency for the current note (before modulation and portamento)
    base_frequency: f32,
    /// Target frequency for portamento (after note change, before glide)
    target_frequency: f32,
}

impl Voice {
    pub fn new(sample_rate: f32) -> Self {
        let waveform = WaveformType::Sine;
        let adsr_params = AdsrParams::default();
        let lfo_params = LfoParams::default();
        let portamento_params = PortamentoParams::default();
        let initial_frequency = 440.0;

        Self {
            oscillator: SimpleOscillator::new(waveform, sample_rate),
            envelope: AdsrEnvelope::new(adsr_params, sample_rate),
            lfo: Lfo::new(lfo_params, sample_rate),
            portamento: PortamentoGlide::new(portamento_params, initial_frequency, sample_rate),
            note: 0,
            velocity: 0.0,
            aftertouch: 0.0,
            active: false,
            waveform,
            sample_rate,
            pan: 0.0, // Center pan by default
            age: 0,
            base_frequency: initial_frequency,
            target_frequency: initial_frequency,
        }
    }

    pub fn note_on(&mut self, note: u8, velocity: u8, age: u64) {
        self.note = note;
        self.velocity = velocity as f32 / 127.0;
        self.active = true;
        self.age = age;

        // Convert MIDI note to frequency: 440 * 2^((note - 69) / 12)
        self.target_frequency = 440.0 * 2_f32.powf((self.note as f32 - 69.0) / 12.0);

        // Set portamento target (will glide if portamento is active)
        self.portamento.set_target(self.target_frequency);

        // Reset oscillator phase for consistent note start
        self.oscillator.reset();

        // Trigger ADSR envelope
        self.envelope.note_on();

        // Reset LFO phase for consistent modulation
        self.lfo.reset();
    }

    /// Change pitch without retriggering envelope (for legato mode)
    ///
    /// This allows smooth pitch transitions while maintaining the current envelope state.
    /// Used in legato monophonic mode where we don't want to retrigger the attack phase.
    /// Portamento will apply if enabled, creating smooth glides between notes.
    pub fn change_pitch_legato(&mut self, note: u8, velocity: u8, age: u64) {
        self.note = note;
        self.velocity = velocity as f32 / 127.0;
        self.age = age;

        // Convert MIDI note to frequency
        self.target_frequency = 440.0 * 2_f32.powf((self.note as f32 - 69.0) / 12.0);

        // Set portamento target (will glide if portamento is active)
        self.portamento.set_target(self.target_frequency);

        // Note: oscillator is NOT reset, allowing phase continuity
        // Note: envelope is NOT retriggered, allowing smooth transitions
        // Note: LFO is NOT reset, maintaining modulation continuity
        // Note: portamento handles the frequency glide smoothly
    }

    pub fn note_off(&mut self) {
        self.active = false;
        // Trigger envelope release
        self.envelope.note_off();
    }

    /// Immediately stop the voice without release phase
    ///
    /// Used in mono/legato modes to forcibly cut notes without waiting for release.
    /// This resets the envelope and marks the voice as inactive immediately.
    pub fn force_stop(&mut self) {
        self.active = false;
        self.envelope.reset();
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

    /// Set current channel aftertouch value for this voice (0.0 .. 1.0)
    pub fn set_aftertouch(&mut self, value: f32) {
        self.aftertouch = value.clamp(0.0, 1.0);
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

    pub fn set_lfo(&mut self, params: LfoParams) {
        self.lfo.set_params(params);
    }

    pub fn get_lfo_params(&self) -> LfoParams {
        self.lfo.params()
    }

    pub fn set_portamento(&mut self, params: PortamentoParams) {
        self.portamento.set_params(params);
    }

    pub fn get_portamento_params(&self) -> PortamentoParams {
        self.portamento.params()
    }

    /// Returns a stereo sample `(left, right)`
    pub fn next_sample(&mut self) -> (f32, f32) {
        use super::lfo::LfoDestination;

        // Process portamento to get current (glided) frequency
        self.base_frequency = self.portamento.process(self.target_frequency);

        // Process LFO
        let lfo_value = self.lfo.process();

        // Apply LFO modulation based on destination
        match self.lfo.destination() {
            LfoDestination::None => {
                // No modulation, oscillator uses portamento frequency
                self.oscillator.set_frequency(self.base_frequency);
            }
            LfoDestination::Pitch => {
                // Pitch modulation (vibrato) on top of portamento
                // LFO value is in range [-depth, +depth]
                // Scale to semitones: ±depth semitones (max ±2 semitones with depth=1.0)
                let semitone_offset = lfo_value * 2.0; // Scale to ±2 semitones max
                let frequency_multiplier = 2_f32.powf(semitone_offset / 12.0);
                let modulated_frequency = self.base_frequency * frequency_multiplier;
                self.oscillator.set_frequency(modulated_frequency);
            }
            LfoDestination::Volume => {
                // Volume modulation handled below
                self.oscillator.set_frequency(self.base_frequency);
            }
            LfoDestination::FilterCutoff => {
                // Filter cutoff modulation - not implemented yet (Phase 3a)
                self.oscillator.set_frequency(self.base_frequency);
            }
        }

        // Process envelope
        let envelope_value = self.envelope.process();

        // Generate oscillator sample
        let mut sample = self.oscillator.next_sample();

        // Apply volume modulation if selected
        if matches!(self.lfo.destination(), LfoDestination::Volume) {
            // Tremolo: modulate amplitude
            // LFO value is in range [-depth, +depth]
            // Convert to multiplier: 1.0 ± lfo_value (so range is [1-depth, 1+depth])
            let volume_multiplier = 1.0 + lfo_value;
            sample *= volume_multiplier;
        }

        // Apply envelope and velocity
        sample *= self.velocity * envelope_value;

        // Apply panning
        let angle = (self.pan.clamp(-1.0, 1.0) * 0.5 + 0.5) * FRAC_PI_2;
        let left = sample * angle.cos();
        let right = sample * angle.sin();

        (left, right)
    }

    /// Render next sample using the modulation matrix (MVP)
    /// Returns a stereo sample `(left, right)`
    pub fn next_sample_with_matrix(&mut self, matrix: &ModulationMatrix) -> (f32, f32) {
        use super::lfo::LfoDestination;

        // Process portamento to get base (glided) frequency
        self.base_frequency = self.portamento.process(self.target_frequency);

        // Process LFO and cache value for this sample
        let lfo_value = self.lfo.process(); // in [-depth, +depth]

        // Process envelope and cache value
        let envelope_value = self.envelope.process();

        // Legacy LFO destination: compute semitone offset if targeting pitch
        let legacy_lfo_semitones = if matches!(self.lfo.destination(), LfoDestination::Pitch) {
            // Scale to ±2 semitones max as before
            lfo_value * 2.0
        } else {
            0.0
        };

        // Apply legacy pitch modulation first
        let mut frequency = if legacy_lfo_semitones != 0.0 {
            let mult = 2_f32.powf(legacy_lfo_semitones / 12.0);
            self.base_frequency * mult
        } else {
            self.base_frequency
        };

        // Apply matrix: compute pitch delta, amplitude multiplier, and pan
        let (pitch_semitones, amp_mult, pan_mod) = matrix.apply(
            self.velocity,
            self.aftertouch,
            &[lfo_value],
            self.envelope.current_value(),
        );

        if pitch_semitones != 0.0 {
            let mult = 2_f32.powf(pitch_semitones / 12.0);
            frequency *= mult;
        }

        // Update oscillator frequency
        self.oscillator.set_frequency(frequency);

        // Generate oscillator sample
        let mut sample = self.oscillator.next_sample();

        // Apply legacy LFO volume modulation if selected
        if matches!(self.lfo.destination(), LfoDestination::Volume) {
            let volume_multiplier = 1.0 + lfo_value;
            sample *= volume_multiplier;
        }

        // Apply envelope and velocity
        sample *= self.velocity * envelope_value;

        // Apply matrix amplitude multiplier
        sample *= amp_mult;

        // Apply base pan + pan modulation
        let final_pan = (self.pan + pan_mod).clamp(-1.0, 1.0);

        // Apply constant-power panning law
        let angle = (final_pan * 0.5 + 0.5) * FRAC_PI_2;
        let left = sample * angle.cos();
        let right = sample * angle.sin();

        (left, right)
    }
}

