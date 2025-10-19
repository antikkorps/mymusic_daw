// Voice - Une note jouée

use super::effect::EffectChain;
use super::envelope::{AdsrEnvelope, AdsrParams};
use super::filter::{FilterParams, StateVariableFilter};
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
    filter: StateVariableFilter,
    /// Effect chain for additional effects (delay, reverb, etc.)
    /// Note: Filter is kept separate for modulation support
    effect_chain: EffectChain,
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
        let filter_params = FilterParams::default();
        let initial_frequency = 440.0;

        Self {
            oscillator: SimpleOscillator::new(waveform, sample_rate),
            envelope: AdsrEnvelope::new(adsr_params, sample_rate),
            lfo: Lfo::new(lfo_params, sample_rate),
            portamento: PortamentoGlide::new(portamento_params, initial_frequency, sample_rate),
            filter: StateVariableFilter::new(filter_params, sample_rate),
            effect_chain: EffectChain::with_capacity(4), // Pre-allocate for up to 4 effects
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

        // Reset filter state to avoid clicks
        self.filter.reset();

        // Reset effect chain to avoid residual delays/reverb
        self.effect_chain.reset();
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
        self.filter.reset();
        self.effect_chain.reset();
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

    pub fn set_filter(&mut self, params: FilterParams) {
        self.filter.set_params(params);
    }

    pub fn get_filter_params(&self) -> FilterParams {
        self.filter.params()
    }

    /// Get mutable reference to the effect chain
    ///
    /// Used for adding/removing effects and controlling individual effects.
    pub fn effect_chain_mut(&mut self) -> &mut EffectChain {
        &mut self.effect_chain
    }

    /// Get reference to the effect chain
    pub fn effect_chain(&self) -> &EffectChain {
        &self.effect_chain
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

        // Apply filter
        sample = self.filter.process(sample);

        // Apply effect chain (delay, reverb, etc.)
        sample = self.effect_chain.process(sample);

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

        // Apply matrix: compute pitch delta, amplitude multiplier, pan, and filter cutoff
        let (pitch_semitones, amp_mult, pan_mod, filter_cutoff_mult) = matrix.apply(
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

        // Apply filter with modulated cutoff
        let base_cutoff = self.filter.params().cutoff;
        let modulated_cutoff = base_cutoff * filter_cutoff_mult;
        sample = self.filter.process_modulated(sample, modulated_cutoff);

        // Apply effect chain (delay, reverb, etc.)
        sample = self.effect_chain.process(sample);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::filter::FilterType;
    use crate::synth::lfo::LfoDestination;
    use crate::synth::modulation::{ModDestination, ModRouting, ModSource, ModulationMatrix};
    use crate::synth::oscillator::WaveformType;

    #[test]
    fn test_filter_modulation_with_envelope() {
        let sample_rate = 44100.0;
        let mut voice = Voice::new(sample_rate);

        // Set up filter with low-pass mode and low cutoff
        let mut filter_params = FilterParams::default();
        filter_params.cutoff = 200.0; // Very low cutoff
        filter_params.resonance = 2.0; // Some resonance for audible effect
        filter_params.filter_type = FilterType::LowPass;
        filter_params.enabled = true;
        voice.set_filter(filter_params);

        // Set up fast envelope (short attack/decay for quick test)
        let envelope_params = AdsrParams {
            attack: 0.05,   // 50ms attack
            decay: 0.1,     // 100ms decay
            sustain: 0.5,
            release: 0.1,   // 100ms release
        };
        voice.set_adsr(envelope_params);

        // Create modulation matrix with Envelope → FilterCutoff routing
        let mut matrix = ModulationMatrix::new_empty();
        matrix.set_routing(
            0,
            ModRouting {
                source: ModSource::Envelope,
                destination: ModDestination::FilterCutoff,
                amount: 10.0, // Strong modulation: cutoff can go up to 10x
                enabled: true,
            },
        );

        // Trigger a note
        voice.note_on(60, 100, 0); // Middle C, velocity 100

        // Process samples during attack phase
        // During attack, envelope should rise from 0 to 1.0
        // Filter cutoff should rise from 200Hz to 200Hz * 11.0 = 2200Hz (1.0 + 10.0)
        let attack_samples = (0.05 * sample_rate) as usize; // 50ms

        let mut attack_outputs = Vec::new();
        for _ in 0..attack_samples {
            let (left, _right) = voice.next_sample_with_matrix(&matrix);
            attack_outputs.push(left);
        }

        // Verify that output is not silent (filter is working)
        let attack_max_amplitude = attack_outputs
            .iter()
            .map(|s| s.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        assert!(
            attack_max_amplitude > 0.01,
            "Attack phase should produce audible output"
        );

        // Process samples during decay phase
        let decay_samples = (0.1 * sample_rate) as usize; // 100ms

        let mut decay_outputs = Vec::new();
        for _ in 0..decay_samples {
            let (left, _right) = voice.next_sample_with_matrix(&matrix);
            decay_outputs.push(left);
        }

        // Verify that output continues during decay
        let decay_max_amplitude = decay_outputs
            .iter()
            .map(|s| s.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        assert!(
            decay_max_amplitude > 0.01,
            "Decay phase should produce audible output"
        );

        // Verify that all samples are finite (no NaN or Inf from filter instability)
        for sample in attack_outputs.iter().chain(decay_outputs.iter()) {
            assert!(sample.is_finite(), "All samples should be finite");
        }
    }

    #[test]
    fn test_filter_modulation_with_lfo() {
        let sample_rate = 44100.0;
        let mut voice = Voice::new(sample_rate);

        // Set up filter with low-pass mode
        let mut filter_params = FilterParams::default();
        filter_params.cutoff = 500.0;
        filter_params.resonance = 2.0;
        filter_params.filter_type = FilterType::LowPass;
        filter_params.enabled = true;
        voice.set_filter(filter_params);

        // Set up LFO with moderate speed
        let lfo_params = LfoParams {
            waveform: WaveformType::Sine,
            rate: 5.0, // 5Hz LFO
            depth: 1.0,
            destination: LfoDestination::None, // Using matrix instead
        };
        voice.set_lfo(lfo_params);

        // Set up envelope with sustain to keep note playing
        let envelope_params = AdsrParams {
            attack: 0.01,
            decay: 0.01,
            sustain: 1.0,
            release: 0.1,
        };
        voice.set_adsr(envelope_params);

        // Create modulation matrix with LFO → FilterCutoff routing
        let mut matrix = ModulationMatrix::new_empty();
        matrix.set_routing(
            0,
            ModRouting {
                source: ModSource::Lfo(0), // First (and only) LFO
                destination: ModDestination::FilterCutoff,
                amount: 3.0, // Moderate modulation
                enabled: true,
            },
        );

        // Trigger a note
        voice.note_on(60, 100, 0);

        // Process samples for one LFO cycle (200ms @ 5Hz)
        let cycle_samples = (0.2 * sample_rate) as usize;

        let mut outputs = Vec::new();
        for _ in 0..cycle_samples {
            let (left, _right) = voice.next_sample_with_matrix(&matrix);
            outputs.push(left);
        }

        // Verify that output is not silent
        let max_amplitude = outputs
            .iter()
            .map(|s| s.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        assert!(max_amplitude > 0.01, "Should produce audible output");

        // Verify all samples are finite
        for sample in outputs.iter() {
            assert!(sample.is_finite(), "All samples should be finite");
        }

        // Verify that there's variation in the output (LFO is modulating)
        // Compute variance of absolute values
        let mean: f32 = outputs.iter().map(|s| s.abs()).sum::<f32>() / outputs.len() as f32;
        let variance: f32 = outputs
            .iter()
            .map(|s| {
                let diff = s.abs() - mean;
                diff * diff
            })
            .sum::<f32>()
            / outputs.len() as f32;

        // There should be some variation due to LFO modulation
        assert!(variance > 0.0001, "LFO modulation should create variance in output");
    }

    #[test]
    fn test_filter_without_modulation() {
        let sample_rate = 44100.0;
        let mut voice = Voice::new(sample_rate);

        // Set up filter
        let mut filter_params = FilterParams::default();
        filter_params.cutoff = 1000.0;
        filter_params.resonance = 1.0;
        filter_params.filter_type = FilterType::LowPass;
        filter_params.enabled = true;
        voice.set_filter(filter_params);

        // Set up simple envelope
        let envelope_params = AdsrParams {
            attack: 0.01,
            decay: 0.01,
            sustain: 1.0,
            release: 0.1,
        };
        voice.set_adsr(envelope_params);

        // Empty modulation matrix (no modulation)
        let matrix = ModulationMatrix::new_empty();

        // Trigger a note
        voice.note_on(60, 100, 0);

        // Process some samples
        let mut outputs = Vec::new();
        for _ in 0..1000 {
            let (left, _right) = voice.next_sample_with_matrix(&matrix);
            outputs.push(left);
        }

        // Verify that output is audible
        let max_amplitude = outputs
            .iter()
            .map(|s| s.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        assert!(max_amplitude > 0.01, "Should produce audible output");

        // Verify all samples are finite
        for sample in outputs.iter() {
            assert!(sample.is_finite(), "All samples should be finite");
        }
    }

    #[test]
    fn test_filter_bypass() {
        let sample_rate = 44100.0;
        let mut voice = Voice::new(sample_rate);

        // Set up filter but disable it
        let mut filter_params = FilterParams::default();
        filter_params.enabled = false; // Bypass filter
        voice.set_filter(filter_params);

        // Set up simple envelope
        let envelope_params = AdsrParams {
            attack: 0.01,
            decay: 0.01,
            sustain: 1.0,
            release: 0.1,
        };
        voice.set_adsr(envelope_params);

        // Empty modulation matrix
        let matrix = ModulationMatrix::new_empty();

        // Trigger a note
        voice.note_on(60, 100, 0);

        // Process some samples
        let mut outputs = Vec::new();
        for _ in 0..1000 {
            let (left, _right) = voice.next_sample_with_matrix(&matrix);
            outputs.push(left);
        }

        // Verify that output is audible even with filter bypassed
        let max_amplitude = outputs
            .iter()
            .map(|s| s.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        assert!(
            max_amplitude > 0.01,
            "Should produce audible output even with filter bypassed"
        );

        // Verify all samples are finite
        for sample in outputs.iter() {
            assert!(sample.is_finite(), "All samples should be finite");
        }
    }
}

