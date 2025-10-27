use crate::sampler::engine::SamplerVoice;
use crate::sampler::loader::Sample;
use std::sync::Arc;

use super::effect::EffectChain;
use super::envelope::{AdsrEnvelope, AdsrParams};
use super::filter::{FilterParams, StateVariableFilter};
use super::lfo::{Lfo, LfoParams};
use super::modulation::ModulationMatrix;
use super::oscillator::{Oscillator, SimpleOscillator, WaveformType};
use super::portamento::{PortamentoGlide, PortamentoParams};
use std::f32::consts::FRAC_PI_2;

pub enum Voice {
    Synth(SynthVoice),
    Sampler(SamplerVoice),
}

impl Voice {
    pub fn new_synth(sample_rate: f32) -> Self {
        Voice::Synth(SynthVoice::new(sample_rate))
    }

    pub fn new_sampler(sample: Arc<Sample>, sample_rate: f32) -> Self {
        Voice::Sampler(SamplerVoice::new(sample, sample_rate))
    }

    pub fn note_on(&mut self, note: u8, velocity: u8, age: u64) {
        match self {
            Voice::Synth(v) => v.note_on(note, velocity, age),
            Voice::Sampler(v) => v.note_on(note, velocity, age),
        }
    }

    pub fn note_off(&mut self) {
        match self {
            Voice::Synth(v) => v.note_off(),
            Voice::Sampler(v) => v.note_off(),
        }
    }

    pub fn force_stop(&mut self) {
        match self {
            Voice::Synth(v) => v.force_stop(),
            Voice::Sampler(v) => v.force_stop(),
        }
    }

    pub fn is_active(&self) -> bool {
        match self {
            Voice::Synth(v) => v.is_active(),
            Voice::Sampler(v) => v.is_active(),
        }
    }

    pub fn get_note(&self) -> u8 {
        match self {
            Voice::Synth(v) => v.get_note(),
            Voice::Sampler(v) => v.get_note(),
        }
    }

    pub fn get_age(&self) -> u64 {
        match self {
            Voice::Synth(v) => v.get_age(),
            Voice::Sampler(v) => v.get_age(),
        }
    }

    pub fn is_releasing(&self) -> bool {
        match self {
            Voice::Synth(v) => v.is_releasing(),
            Voice::Sampler(v) => v.is_releasing(),
        }
    }

    pub fn change_pitch_legato(&mut self, note: u8, velocity: u8, age: u64) {
        match self {
            Voice::Synth(v) => v.change_pitch_legato(note, velocity, age),
            Voice::Sampler(v) => v.change_pitch_legato(note, velocity, age),
        }
    }

    pub fn set_aftertouch(&mut self, value: f32) {
        match self {
            Voice::Synth(v) => v.set_aftertouch(value),
            Voice::Sampler(v) => v.set_aftertouch(value),
        }
    }

    pub fn next_sample_with_matrix(&mut self, matrix: &ModulationMatrix) -> (f32, f32) {
        match self {
            Voice::Synth(v) => v.next_sample_with_matrix(matrix),
            Voice::Sampler(v) => v.next_sample_with_matrix(matrix),
        }
    }

    // --- Synth-only methods ---
    pub fn set_waveform(&mut self, waveform: WaveformType) {
        if let Voice::Synth(v) = self {
            v.set_waveform(waveform);
        }
    }

    pub fn set_adsr(&mut self, params: AdsrParams) {
        if let Voice::Synth(v) = self {
            v.set_adsr(params);
        }
    }

    pub fn set_lfo(&mut self, params: LfoParams) {
        if let Voice::Synth(v) = self {
            v.set_lfo(params);
        }
    }

    pub fn get_lfo_params(&self) -> LfoParams {
        match self {
            Voice::Synth(v) => v.get_lfo_params(),
            Voice::Sampler(_) => LfoParams::default(),
        }
    }

    pub fn set_portamento(&mut self, params: PortamentoParams) {
        if let Voice::Synth(v) = self {
            v.set_portamento(params);
        }
    }

    pub fn get_portamento_params(&self) -> PortamentoParams {
        match self {
            Voice::Synth(v) => v.get_portamento_params(),
            Voice::Sampler(_) => PortamentoParams::default(),
        }
    }

    pub fn set_filter(&mut self, params: FilterParams) {
        if let Voice::Synth(v) = self {
            v.set_filter(params);
        }
    }

    pub fn get_filter_params(&self) -> FilterParams {
        match self {
            Voice::Synth(v) => v.get_filter_params(),
            Voice::Sampler(_) => FilterParams::default(),
        }
    }
}

pub struct SynthVoice {
    oscillator: SimpleOscillator,
    envelope: AdsrEnvelope,
    lfo: Lfo,
    portamento: PortamentoGlide,
    filter: StateVariableFilter,
    effect_chain: EffectChain,
    note: u8,
    velocity: f32,
    aftertouch: f32,
    active: bool,
    waveform: WaveformType,
    sample_rate: f32,
    pan: f32,
    age: u64,
    base_frequency: f32,
    target_frequency: f32,
}

impl SynthVoice {
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
            effect_chain: EffectChain::with_capacity(4),
            note: 0,
            velocity: 0.0,
            aftertouch: 0.0,
            active: false,
            waveform,
            sample_rate,
            pan: 0.0,
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
        self.target_frequency = 440.0 * 2_f32.powf((self.note as f32 - 69.0) / 12.0);
        self.portamento.set_target(self.target_frequency);
        self.oscillator.reset();
        self.envelope.note_on();
        self.lfo.reset();
        self.filter.reset();
        self.effect_chain.reset();
    }

    pub fn change_pitch_legato(&mut self, note: u8, velocity: u8, age: u64) {
        self.note = note;
        self.velocity = velocity as f32 / 127.0;
        self.age = age;
        self.target_frequency = 440.0 * 2_f32.powf((self.note as f32 - 69.0) / 12.0);
        self.portamento.set_target(self.target_frequency);
    }

    pub fn note_off(&mut self) {
        self.active = false;
        self.envelope.note_off();
    }

    pub fn force_stop(&mut self) {
        self.active = false;
        self.envelope.reset();
        self.filter.reset();
        self.effect_chain.reset();
    }

    pub fn is_active(&self) -> bool {
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

    pub fn set_aftertouch(&mut self, value: f32) {
        self.aftertouch = value.clamp(0.0, 1.0);
    }

    pub fn is_releasing(&self) -> bool {
        !self.active && self.envelope.is_active()
    }

    pub fn set_waveform(&mut self, waveform: WaveformType) {
        self.waveform = waveform;
        self.oscillator = SimpleOscillator::new(waveform, self.sample_rate);
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

    pub fn effect_chain_mut(&mut self) -> &mut EffectChain {
        &mut self.effect_chain
    }

    pub fn effect_chain(&self) -> &EffectChain {
        &self.effect_chain
    }

    pub fn next_sample(&mut self) -> (f32, f32) {
        use super::lfo::LfoDestination;
        self.base_frequency = self.portamento.process(self.target_frequency);
        let lfo_value = self.lfo.process();
        match self.lfo.destination() {
            LfoDestination::None => {
                self.oscillator.set_frequency(self.base_frequency);
            }
            LfoDestination::Pitch => {
                let semitone_offset = lfo_value * 2.0;
                let frequency_multiplier = 2_f32.powf(semitone_offset / 12.0);
                let modulated_frequency = self.base_frequency * frequency_multiplier;
                self.oscillator.set_frequency(modulated_frequency);
            }
            LfoDestination::Volume => {
                self.oscillator.set_frequency(self.base_frequency);
            }
            LfoDestination::FilterCutoff => {
                self.oscillator.set_frequency(self.base_frequency);
            }
        }
        let envelope_value = self.envelope.process();
        let mut sample = self.oscillator.next_sample();
        sample = self.filter.process(sample);
        sample = self.effect_chain.process(sample);
        if matches!(self.lfo.destination(), LfoDestination::Volume) {
            let volume_multiplier = 1.0 + lfo_value;
            sample *= volume_multiplier;
        }
        sample *= self.velocity * envelope_value;
        let angle = (self.pan.clamp(-1.0, 1.0) * 0.5 + 0.5) * FRAC_PI_2;
        let left = sample * angle.cos();
        let right = sample * angle.sin();
        (left, right)
    }

    pub fn next_sample_with_matrix(&mut self, matrix: &ModulationMatrix) -> (f32, f32) {
        use super::lfo::LfoDestination;
        self.base_frequency = self.portamento.process(self.target_frequency);
        let lfo_value = self.lfo.process();
        let envelope_value = self.envelope.process();
        let legacy_lfo_semitones = if matches!(self.lfo.destination(), LfoDestination::Pitch) {
            lfo_value * 2.0
        } else {
            0.0
        };
        let mut frequency = if legacy_lfo_semitones != 0.0 {
            let mult = 2_f32.powf(legacy_lfo_semitones / 12.0);
            self.base_frequency * mult
        } else {
            self.base_frequency
        };
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
        self.oscillator.set_frequency(frequency);
        let mut sample = self.oscillator.next_sample();
        let base_cutoff = self.filter.params().cutoff;
        let modulated_cutoff = base_cutoff * filter_cutoff_mult;
        sample = self.filter.process_modulated(sample, modulated_cutoff);
        sample = self.effect_chain.process(sample);
        if matches!(self.lfo.destination(), LfoDestination::Volume) {
            let volume_multiplier = 1.0 + lfo_value;
            sample *= volume_multiplier;
        }
        sample *= self.velocity * envelope_value;
        sample *= amp_mult;
        let final_pan = (self.pan + pan_mod).clamp(-1.0, 1.0);
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
        let mut voice = SynthVoice::new(sample_rate);
        let filter_params = FilterParams {
            cutoff: 200.0,
            resonance: 2.0,
            filter_type: FilterType::LowPass,
            enabled: true,
        };
        voice.set_filter(filter_params);
        let envelope_params = AdsrParams {
            attack: 0.05,
            decay: 0.1,
            sustain: 0.5,
            release: 0.1,
        };
        voice.set_adsr(envelope_params);
        let mut matrix = ModulationMatrix::new_empty();
        matrix.set_routing(
            0,
            ModRouting {
                source: ModSource::Envelope,
                destination: ModDestination::FilterCutoff,
                amount: 10.0,
                enabled: true,
            },
        );
        voice.note_on(60, 100, 0);
        let attack_samples = (0.05 * sample_rate) as usize;
        let mut attack_outputs = Vec::new();
        for _ in 0..attack_samples {
            let (left, _right) = voice.next_sample_with_matrix(&matrix);
            attack_outputs.push(left);
        }
        let attack_max_amplitude = attack_outputs
            .iter()
            .map(|s| s.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        assert!(
            attack_max_amplitude > 0.01,
            "Attack phase should produce audible output"
        );
        let decay_samples = (0.1 * sample_rate) as usize;
        let mut decay_outputs = Vec::new();
        for _ in 0..decay_samples {
            let (left, _right) = voice.next_sample_with_matrix(&matrix);
            decay_outputs.push(left);
        }
        let decay_max_amplitude = decay_outputs
            .iter()
            .map(|s| s.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        assert!(
            decay_max_amplitude > 0.01,
            "Decay phase should produce audible output"
        );
        for sample in attack_outputs.iter().chain(decay_outputs.iter()) {
            assert!(sample.is_finite(), "All samples should be finite");
        }
    }

    #[test]
    fn test_filter_modulation_with_lfo() {
        let sample_rate = 44100.0;
        let mut voice = SynthVoice::new(sample_rate);
        let filter_params = FilterParams {
            cutoff: 500.0,
            resonance: 2.0,
            filter_type: FilterType::LowPass,
            enabled: true,
        };
        voice.set_filter(filter_params);
        let lfo_params = LfoParams {
            waveform: WaveformType::Sine,
            rate: 5.0,
            depth: 1.0,
            destination: LfoDestination::None,
        };
        voice.set_lfo(lfo_params);
        let envelope_params = AdsrParams {
            attack: 0.01,
            decay: 0.01,
            sustain: 1.0,
            release: 0.1,
        };
        voice.set_adsr(envelope_params);
        let mut matrix = ModulationMatrix::new_empty();
        matrix.set_routing(
            0,
            ModRouting {
                source: ModSource::Lfo(0),
                destination: ModDestination::FilterCutoff,
                amount: 3.0,
                enabled: true,
            },
        );
        voice.note_on(60, 100, 0);
        let cycle_samples = (0.2 * sample_rate) as usize;
        let mut outputs = Vec::new();
        for _ in 0..cycle_samples {
            let (left, _right) = voice.next_sample_with_matrix(&matrix);
            outputs.push(left);
        }
        let max_amplitude = outputs
            .iter()
            .map(|s| s.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        assert!(max_amplitude > 0.01, "Should produce audible output");
        for sample in outputs.iter() {
            assert!(sample.is_finite(), "All samples should be finite");
        }
        let mean: f32 = outputs.iter().map(|s| s.abs()).sum::<f32>() / outputs.len() as f32;
        let variance: f32 = outputs
            .iter()
            .map(|s| {
                let diff = s.abs() - mean;
                diff * diff
            })
            .sum::<f32>()
            / outputs.len() as f32;
        assert!(
            variance > 0.0001,
            "LFO modulation should create variance in output"
        );
    }

    #[test]
    fn test_filter_without_modulation() {
        let sample_rate = 44100.0;
        let mut voice = SynthVoice::new(sample_rate);
        let filter_params = FilterParams {
            cutoff: 1000.0,
            resonance: 1.0,
            filter_type: FilterType::LowPass,
            enabled: true,
        };
        voice.set_filter(filter_params);
        let envelope_params = AdsrParams {
            attack: 0.01,
            decay: 0.01,
            sustain: 1.0,
            release: 0.1,
        };
        voice.set_adsr(envelope_params);
        let matrix = ModulationMatrix::new_empty();
        voice.note_on(60, 100, 0);
        let mut outputs = Vec::new();
        for _ in 0..1000 {
            let (left, _right) = voice.next_sample_with_matrix(&matrix);
            outputs.push(left);
        }
        let max_amplitude = outputs
            .iter()
            .map(|s| s.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        assert!(max_amplitude > 0.01, "Should produce audible output");
        for sample in outputs.iter() {
            assert!(sample.is_finite(), "All samples should be finite");
        }
    }

    #[test]
    fn test_filter_bypass() {
        let sample_rate = 44100.0;
        let mut voice = SynthVoice::new(sample_rate);
        let filter_params = FilterParams {
            enabled: false,
            ..Default::default()
        };
        voice.set_filter(filter_params);
        let envelope_params = AdsrParams {
            attack: 0.01,
            decay: 0.01,
            sustain: 1.0,
            release: 0.1,
        };
        voice.set_adsr(envelope_params);
        let matrix = ModulationMatrix::new_empty();
        voice.note_on(60, 100, 0);
        let mut outputs = Vec::new();
        for _ in 0..1000 {
            let (left, _right) = voice.next_sample_with_matrix(&matrix);
            outputs.push(left);
        }
        let max_amplitude = outputs
            .iter()
            .map(|s| s.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        assert!(
            max_amplitude > 0.01,
            "Should produce audible output even with filter bypassed"
        );
        for sample in outputs.iter() {
            assert!(sample.is_finite(), "All samples should be finite");
        }
    }
}
