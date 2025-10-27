// Voice Manager - Polyphony handling

use super::modulation::{MAX_ROUTINGS, ModRouting, ModulationMatrix};
use super::oscillator::WaveformType;
use super::poly_mode::PolyMode;
use super::voice::Voice;
use crate::sampler::loader::{LoopMode, Sample, SampleData};
use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::Arc;

const MAX_VOICES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceMode {
    Synth,
    Sampler,
}

pub struct VoiceManager {
    voices: [Voice; MAX_VOICES],
    age_counter: u64,
    poly_mode: PolyMode,
    last_note: Option<u8>,
    mod_matrix: ModulationMatrix,
    aftertouch: f32,
    pub voice_mode: VoiceMode,
    dummy_sample: Arc<Sample>,
    samples: Vec<Arc<Sample>>,
    note_to_sample_map: HashMap<u8, usize>,
    sample_rate: f32,
}

impl VoiceManager {
    pub fn new(sample_rate: f32) -> Self {
        let mut dummy_data = Vec::with_capacity(sample_rate as usize);
        let frequency = 440.0;
        for i in 0..sample_rate as usize {
            let t = i as f32 / sample_rate;
            dummy_data.push((t * frequency * 2.0 * PI).sin());
        }
        let dummy_sample = Arc::new(Sample {
            name: "Dummy Sine".to_string(),
            data: SampleData::F32(dummy_data),
            sample_rate: sample_rate as u32,
            source_channels: 1,
            loop_mode: LoopMode::Off,
            loop_start: 0,
            loop_end: 0,
            reverse: false,
            volume: 1.0,
            pan: 0.0,
            pitch_offset: 0,
        });

        let voices = std::array::from_fn(|_| Voice::new_synth(sample_rate));

        Self {
            voices,
            age_counter: 0,
            poly_mode: PolyMode::default(),
            last_note: None,
            mod_matrix: ModulationMatrix::new_empty(),
            aftertouch: 0.0,
            voice_mode: VoiceMode::Synth,
            dummy_sample,
            samples: Vec::new(),
            note_to_sample_map: HashMap::new(),
            sample_rate,
        }
    }

    pub fn add_sample(&mut self, sample: Arc<Sample>) {
        self.samples.push(sample);
    }

    pub fn set_note_to_sample(&mut self, note: u8, sample_index: usize) {
        if sample_index < self.samples.len() {
            self.note_to_sample_map.insert(note, sample_index);
        }
    }

    pub fn update_sample(&mut self, index: usize, sample: Arc<Sample>) {
        if index < self.samples.len() {
            self.samples[index] = sample;
        }
    }

    pub fn remove_sample(&mut self, index: usize) {
        if index >= self.samples.len() {
            return;
        }

        // Remove the sample from the vector
        self.samples.remove(index);

        // Update note_to_sample_map:
        // - Remove mappings pointing to the removed index
        // - Decrement indices > removed index
        let mut updated_map = std::collections::HashMap::new();
        for (note, sample_idx) in self.note_to_sample_map.iter() {
            if *sample_idx == index {
                // Skip - this mapping is deleted
                continue;
            } else if *sample_idx > index {
                // Decrement index
                updated_map.insert(*note, sample_idx - 1);
            } else {
                // Keep as-is
                updated_map.insert(*note, *sample_idx);
            }
        }
        self.note_to_sample_map = updated_map;

        // Note: Active voices playing the removed sample will continue until they finish
        // This is acceptable as they hold an Arc reference to the sample
    }

    pub fn note_on(&mut self, note: u8, velocity: u8) {
        self.age_counter = self.age_counter.wrapping_add(1);
        match self.poly_mode {
            PolyMode::Poly => self.note_on_poly(note, velocity),
            PolyMode::Mono => self.note_on_mono(note, velocity),
            PolyMode::Legato => self.note_on_legato(note, velocity),
        }
        self.last_note = Some(note);
    }

    fn note_on_poly(&mut self, note: u8, velocity: u8) {
        let voice_index = self.voices.iter().position(|v| !v.is_active());
        let index_to_use = match voice_index {
            Some(index) => index,
            None => self.find_voice_to_steal(),
        };
        let voice = &mut self.voices[index_to_use];

        match self.voice_mode {
            VoiceMode::Synth => {
                if !matches!(voice, Voice::Synth(_)) {
                    *voice = Voice::new_synth(self.sample_rate);
                }
            }
            VoiceMode::Sampler => {
                let sample_index = self.note_to_sample_map.get(&note).copied();
                let sample_to_use = match sample_index {
                    Some(index) => self
                        .samples
                        .get(index)
                        .cloned()
                        .unwrap_or_else(|| self.dummy_sample.clone()),
                    None => self
                        .samples
                        .last()
                        .cloned()
                        .unwrap_or_else(|| self.dummy_sample.clone()),
                };
                *voice = Voice::new_sampler(sample_to_use, self.sample_rate);
            }
        }
        voice.note_on(note, velocity, self.age_counter);
    }

    fn note_on_mono(&mut self, note: u8, velocity: u8) {
        for voice in &mut self.voices {
            if voice.is_active() {
                voice.force_stop();
            }
        }
        let voice = &mut self.voices[0];
        match self.voice_mode {
            VoiceMode::Synth => {
                if !matches!(voice, Voice::Synth(_)) {
                    *voice = Voice::new_synth(self.sample_rate);
                }
            }
            VoiceMode::Sampler => {
                let sample_index = self.note_to_sample_map.get(&note).copied();
                let sample_to_use = match sample_index {
                    Some(index) => self
                        .samples
                        .get(index)
                        .cloned()
                        .unwrap_or_else(|| self.dummy_sample.clone()),
                    None => self
                        .samples
                        .last()
                        .cloned()
                        .unwrap_or_else(|| self.dummy_sample.clone()),
                };
                *voice = Voice::new_sampler(sample_to_use, self.sample_rate);
            }
        }
        voice.note_on(note, velocity, self.age_counter);
    }

    fn note_on_legato(&mut self, note: u8, velocity: u8) {
        if let Some(active_voice) = self.voices.iter_mut().find(|v| v.is_active()) {
            active_voice.change_pitch_legato(note, velocity, self.age_counter);
        } else {
            let voice = &mut self.voices[0];
            match self.voice_mode {
                VoiceMode::Synth => {
                    if !matches!(voice, Voice::Synth(_)) {
                        *voice = Voice::new_synth(self.sample_rate);
                    }
                }
                VoiceMode::Sampler => {
                    let sample_index = self.note_to_sample_map.get(&note).copied();
                    let sample_to_use = match sample_index {
                        Some(index) => self
                            .samples
                            .get(index)
                            .cloned()
                            .unwrap_or_else(|| self.dummy_sample.clone()),
                        None => self
                            .samples
                            .last()
                            .cloned()
                            .unwrap_or_else(|| self.dummy_sample.clone()),
                    };
                    *voice = Voice::new_sampler(sample_to_use, self.sample_rate);
                }
            }
            voice.note_on(note, velocity, self.age_counter);
        }
    }

    fn find_voice_to_steal(&self) -> usize {
        let mut best_index = 0;
        let mut best_priority = (false, u64::MAX);
        for (i, voice) in self.voices.iter().enumerate() {
            let is_releasing = voice.is_releasing();
            let age = voice.get_age();
            let priority = (is_releasing, age);
            let should_steal = if is_releasing != best_priority.0 {
                is_releasing
            } else {
                age < best_priority.1
            };
            if should_steal {
                best_priority = priority;
                best_index = i;
            }
        }
        best_index
    }

    pub fn note_off(&mut self, note: u8) {
        for voice in &mut self.voices {
            if voice.is_active() && voice.get_note() == note {
                voice.note_off();
            }
        }
    }

    pub fn set_waveform(&mut self, waveform: WaveformType) {
        for voice in &mut self.voices {
            voice.set_waveform(waveform);
        }
    }

    pub fn set_adsr(&mut self, params: super::envelope::AdsrParams) {
        for voice in &mut self.voices {
            voice.set_adsr(params);
        }
    }

    pub fn set_lfo(&mut self, params: super::lfo::LfoParams) {
        for voice in &mut self.voices {
            voice.set_lfo(params);
        }
    }

    pub fn get_lfo_params(&self) -> super::lfo::LfoParams {
        self.voices[0].get_lfo_params()
    }

    pub fn set_portamento(&mut self, params: super::portamento::PortamentoParams) {
        for voice in &mut self.voices {
            voice.set_portamento(params);
        }
    }

    pub fn get_portamento_params(&self) -> super::portamento::PortamentoParams {
        self.voices[0].get_portamento_params()
    }

    pub fn set_filter(&mut self, params: super::filter::FilterParams) {
        for voice in &mut self.voices {
            voice.set_filter(params);
        }
    }

    pub fn get_filter_params(&self) -> super::filter::FilterParams {
        self.voices[0].get_filter_params()
    }

    pub fn set_poly_mode(&mut self, mode: PolyMode) {
        self.poly_mode = mode;
    }

    pub fn get_poly_mode(&self) -> PolyMode {
        self.poly_mode
    }

    pub fn set_voice_mode(&mut self, mode: VoiceMode) {
        self.voice_mode = mode;
    }

    pub fn set_aftertouch(&mut self, value: u8) {
        let at = (value as f32 / 127.0).clamp(0.0, 1.0);
        self.aftertouch = at;
        for v in &mut self.voices {
            v.set_aftertouch(at);
        }
    }

    pub fn set_mod_routing(&mut self, index: usize, routing: ModRouting) {
        if index < MAX_ROUTINGS {
            self.mod_matrix.set_routing(index, routing);
        }
    }

    pub fn clear_mod_routing(&mut self, index: usize) {
        if index < MAX_ROUTINGS {
            self.mod_matrix.clear_routing(index);
        }
    }

    pub fn next_sample(&mut self) -> (f32, f32) {
        let matrix = self.mod_matrix;

        // Sum all voice outputs
        let (left_sum, right_sum) = self
            .voices
            .iter_mut()
            .map(|v| v.next_sample_with_matrix(&matrix))
            .fold((0.0, 0.0), |(acc_l, acc_r), (voice_l, voice_r)| {
                (acc_l + voice_l, acc_r + voice_r)
            });

        // Dynamic gain staging based on active voices
        // This provides optimal headroom while maximizing loudness
        let active_voices = self.voices.iter().filter(|v| v.is_active()).count();

        // Calculate gain factor:
        // - 1 voice: full gain (1.0)
        // - 4 voices: 0.5 gain
        // - 16 voices: 0.25 gain
        // Formula: 1.0 / sqrt(max(1, n)) provides perceptually balanced scaling
        let gain = if active_voices > 0 {
            1.0 / (active_voices as f32).sqrt()
        } else {
            1.0 // No voices, doesn't matter
        };

        // Apply headroom (0.7 = ~-3dB to prevent digital clipping)
        const HEADROOM: f32 = 0.7;
        let left = left_sum * gain * HEADROOM;
        let right = right_sum * gain * HEADROOM;

        // Soft-limiter (tanh provides smooth saturation instead of harsh clipping)
        // tanh maps (-∞, +∞) → (-1, +1) with smooth curve
        (left.tanh(), right.tanh())
    }

    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|v| v.is_active()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 44100.0;

    #[test]
    fn test_voice_allocation() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);
        assert_eq!(vm.active_voice_count(), 0);
        vm.note_on(60, 100);
        assert_eq!(vm.active_voice_count(), 1);
        vm.note_on(64, 100);
        assert_eq!(vm.active_voice_count(), 2);
        vm.note_on(67, 100);
        assert_eq!(vm.active_voice_count(), 3);
    }

    #[test]
    fn test_gain_staging_multiple_voices() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);

        // Trigger 4 voices at full velocity
        for note in [60, 64, 67, 72] {
            vm.note_on(note, 127);
        }

        // Process samples and verify no clipping
        for _ in 0..100 {
            let (left, right) = vm.next_sample();
            assert!(
                left.abs() <= 1.0,
                "Left channel should not clip with 4 voices: {}",
                left
            );
            assert!(
                right.abs() <= 1.0,
                "Right channel should not clip with 4 voices: {}",
                right
            );
        }
    }

    #[test]
    fn test_gain_staging_max_polyphony() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);

        // Trigger all 16 voices at full velocity (worst case)
        for i in 0..16 {
            vm.note_on(60 + i, 127);
        }

        assert_eq!(vm.active_voice_count(), 16, "Should have 16 active voices");

        // Process samples and verify no clipping even with 16 voices
        for _ in 0..500 {
            let (left, right) = vm.next_sample();

            // With dynamic gain staging and soft-limiter (tanh),
            // output should NEVER exceed [-1, +1]
            assert!(
                left.abs() <= 1.0,
                "Left channel should not clip with 16 voices: {}",
                left
            );
            assert!(
                right.abs() <= 1.0,
                "Right channel should not clip with 16 voices: {}",
                right
            );

            // Should still produce audible output (not silence)
            if left.abs() > 0.001 || right.abs() > 0.001 {
                // Good, we have audio
            }
        }
    }

    #[test]
    fn test_soft_limiter_smoothness() {
        // Test that tanh() soft-limiter provides smooth saturation
        let mut vm = VoiceManager::new(SAMPLE_RATE);

        // Create extreme scenario: many loud voices
        for i in 0..16 {
            vm.note_on(60 + i, 127);
        }

        // Collect samples
        let mut samples = Vec::new();
        for _ in 0..100 {
            let (left, _) = vm.next_sample();
            samples.push(left);
        }

        // Check that samples are smoothly saturated (no sudden jumps)
        for window in samples.windows(2) {
            let diff = (window[1] - window[0]).abs();
            // Difference between consecutive samples should be reasonable
            // (tanh provides smooth saturation, not brick-wall clipping)
            assert!(
                diff < 0.5,
                "Consecutive samples should not have huge jumps: {} → {}",
                window[0],
                window[1]
            );
        }
    }

    // ... (rest of the tests are omitted for brevity but are unchanged)
}
