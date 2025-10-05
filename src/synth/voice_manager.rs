// Voice Manager - Polyphony handling

use super::voice::Voice;

const MAX_VOICES: usize = 16;

pub struct VoiceManager {
    voices: [Voice; MAX_VOICES],
}

impl VoiceManager {
    pub fn new(sample_rate: f32) -> Self {
        // Pre-allocate all voices
        let voices = std::array::from_fn(|_| Voice::new(sample_rate));

        Self { voices }
    }

    pub fn note_on(&mut self, note: u8, velocity: u8) {
        // Search an inactive voice
        if let Some(voice) = self.voices.iter_mut().find(|v| !v.is_active()) {
            voice.note_on(note, velocity);
            return;
        }

        // Voice stealing: Take the first active voice (simple strategy)
        if let Some(voice) = self.voices.first_mut() {
            voice.note_on(note, velocity);
        }
    }

    pub fn note_off(&mut self, note: u8) {
        for voice in &mut self.voices {
            if voice.is_active() && voice.get_note() == note {
                voice.note_off();
            }
        }
    }

    pub fn next_sample(&mut self) -> f32 {
        // Mix all the active voices
        self.voices.iter_mut().map(|v| v.next_sample()).sum::<f32>() / MAX_VOICES as f32 // simple normalization
    }
}
