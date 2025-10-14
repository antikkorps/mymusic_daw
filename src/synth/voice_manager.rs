// Voice Manager - Polyphony handling

use super::oscillator::WaveformType;
use super::voice::Voice;

const MAX_VOICES: usize = 16;

pub struct VoiceManager {
    voices: [Voice; MAX_VOICES],
    /// Age counter incremented on each note_on for voice stealing priority
    age_counter: u64,
}

impl VoiceManager {
    pub fn new(sample_rate: f32) -> Self {
        // Pre-allocate all voices
        let voices = std::array::from_fn(|_| Voice::new(sample_rate));

        Self {
            voices,
            age_counter: 0,
        }
    }

    pub fn note_on(&mut self, note: u8, velocity: u8) {
        // Increment age counter
        self.age_counter = self.age_counter.wrapping_add(1);

        // Search an inactive voice
        if let Some(voice) = self.voices.iter_mut().find(|v| !v.is_active()) {
            voice.note_on(note, velocity, self.age_counter);
            return;
        }

        // Voice stealing: Find the best voice to steal
        let victim_index = self.find_voice_to_steal();
        self.voices[victim_index].note_on(note, velocity, self.age_counter);
    }

    /// Find the best voice to steal using intelligent priority
    ///
    /// Priority (best to worst):
    /// 1. Voice in release phase (already fading out - least perceptible)
    /// 2. Oldest voice (played longest ago - less likely to be noticed)
    fn find_voice_to_steal(&self) -> usize {
        let mut best_index = 0;
        let mut best_priority = (false, std::u64::MAX);

        for (i, voice) in self.voices.iter().enumerate() {
            // Priority calculation:
            // - Tuple (is_releasing, age)
            // - Prioritize releasing voices first (true > false in reverse)
            // - Then prioritize older voices (lower age number)
            let is_releasing = voice.is_releasing();
            let age = voice.get_age();
            let priority = (is_releasing, age);

            // Steal voices with:
            // 1. is_releasing = true first (releasing voices)
            // 2. If tied, steal lowest age (oldest voice)
            // We want: (true, _) > (false, _) and then lowest age
            let should_steal = if is_releasing != best_priority.0 {
                // Prioritize releasing over non-releasing
                is_releasing
            } else {
                // Same release status, pick oldest (lowest age)
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
        // Change waveform for all voices
        for voice in &mut self.voices {
            voice.set_waveform(waveform);
        }
    }

    pub fn set_adsr(&mut self, params: super::envelope::AdsrParams) {
        // Change ADSR parameters for all voices
        for voice in &mut self.voices {
            voice.set_adsr(params);
        }
    }

    pub fn next_sample(&mut self) -> f32 {
        // Mix all the active voices
        self.voices.iter_mut().map(|v| v.next_sample()).sum::<f32>() / 4.0 // gain constant raisonnable
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

        // Au départ, aucune voix active
        assert_eq!(vm.active_voice_count(), 0);

        // Activer une voix
        vm.note_on(60, 100);
        assert_eq!(vm.active_voice_count(), 1);

        // Activer une deuxième voix
        vm.note_on(64, 100);
        assert_eq!(vm.active_voice_count(), 2);

        // Activer une troisième voix
        vm.note_on(67, 100);
        assert_eq!(vm.active_voice_count(), 3);
    }

    #[test]
    fn test_note_off() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);

        // Activer 3 notes
        vm.note_on(60, 100); // C
        vm.note_on(64, 100); // E
        vm.note_on(67, 100); // G
        assert_eq!(vm.active_voice_count(), 3);

        // Désactiver la note du milieu - avec ADSR, la voix reste active pendant release
        vm.note_off(64);
        assert_eq!(vm.active_voice_count(), 3); // Still in release phase

        // Process samples to let envelope finish release
        // Default release is 0.2s = 0.2 * 44100 = 8820 samples
        for _ in 0..10000 {
            vm.next_sample();
        }

        // Now the released voice should be idle
        assert_eq!(vm.active_voice_count(), 2);

        // Désactiver les deux autres
        vm.note_off(60);
        vm.note_off(67);

        // Process release phase
        for _ in 0..10000 {
            vm.next_sample();
        }

        assert_eq!(vm.active_voice_count(), 0);
    }

    #[test]
    fn test_voice_stealing() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);

        // Remplir toutes les 16 voix
        for i in 0..MAX_VOICES {
            vm.note_on(60 + i as u8, 100);
        }
        assert_eq!(vm.active_voice_count(), MAX_VOICES);

        // Activer une 17ème note (doit voler une voix)
        vm.note_on(80, 100);

        // Toujours 16 voix actives (voice stealing)
        assert_eq!(vm.active_voice_count(), MAX_VOICES);
    }

    #[test]
    fn test_voice_stealing_prioritizes_releasing() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);

        // Fill all 16 voices
        for i in 0..MAX_VOICES {
            vm.note_on(60 + i as u8, 100);
        }

        // Release the first voice (note 60)
        vm.note_off(60);

        // The first voice should now be in release phase (still active but releasing)
        assert_eq!(vm.active_voice_count(), MAX_VOICES);

        // Activate a 17th note - should steal the releasing voice (note 60)
        vm.note_on(80, 127);

        // Should still be 16 voices active
        assert_eq!(vm.active_voice_count(), MAX_VOICES);

        // The stolen voice should now be playing note 80, not note 60
        // Count how many voices are playing each note
        let note_60_count = vm.voices.iter().filter(|v| v.is_active() && v.get_note() == 60).count();
        let note_80_count = vm.voices.iter().filter(|v| v.is_active() && v.get_note() == 80).count();

        assert_eq!(note_60_count, 0, "Note 60 should have been stolen (was in release)");
        assert_eq!(note_80_count, 1, "Note 80 should be playing");
    }

    #[test]
    fn test_voice_stealing_oldest_first() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);

        // Fill all 16 voices
        for i in 0..MAX_VOICES {
            vm.note_on(60 + i as u8, 100);
        }

        // All voices are active, no releasing
        assert_eq!(vm.active_voice_count(), MAX_VOICES);

        // Activate a 17th note - should steal the oldest voice (first one, note 60)
        vm.note_on(80, 127);

        // Should still be 16 voices active
        assert_eq!(vm.active_voice_count(), MAX_VOICES);

        // The oldest voice (note 60) should have been stolen
        let note_60_count = vm.voices.iter().filter(|v| v.is_active() && v.get_note() == 60).count();
        assert_eq!(note_60_count, 0, "Oldest voice (note 60) should have been stolen");
    }

    #[test]
    fn test_duplicate_notes() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);

        // Activer la même note plusieurs fois
        vm.note_on(60, 100);
        vm.note_on(60, 80);
        vm.note_on(60, 60);

        // Doit avoir 3 voix actives (polyphonie)
        assert_eq!(vm.active_voice_count(), 3);

        // note_off doit désactiver TOUTES les instances de cette note
        // But with ADSR, they stay active during release
        vm.note_off(60);
        assert_eq!(vm.active_voice_count(), 3); // Still releasing

        // Process release phase
        for _ in 0..10000 {
            vm.next_sample();
        }

        assert_eq!(vm.active_voice_count(), 0);
    }

    #[test]
    fn test_set_waveform() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);

        // Activer quelques notes
        vm.note_on(60, 100);
        vm.note_on(64, 100);

        // Changer la waveform ne doit pas crash
        vm.set_waveform(WaveformType::Square);
        vm.set_waveform(WaveformType::Saw);
        vm.set_waveform(WaveformType::Triangle);

        // Les voix doivent toujours être actives
        assert_eq!(vm.active_voice_count(), 2);
    }

    #[test]
    fn test_next_sample_no_crash() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);

        // Sans voix actives, doit retourner 0
        let sample = vm.next_sample();
        assert_eq!(sample, 0.0);

        // Avec voix actives
        vm.note_on(60, 100);

        // Générer des samples ne doit pas crash
        for _ in 0..1000 {
            let sample = vm.next_sample();
            // Sample doit être fini (pas NaN ou infinity)
            assert!(sample.is_finite());
        }
    }

    #[test]
    fn test_mixed_output_amplitude() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);

        // Activer plusieurs voix
        vm.note_on(60, 127);
        vm.note_on(64, 127);
        vm.note_on(67, 127);

        // Générer des samples et vérifier qu'ils ne clippent pas
        for _ in 0..1000 {
            let sample = vm.next_sample();
            // Avec le gain /4.0, ça devrait rester raisonnable
            assert!(
                sample.abs() < 10.0,
                "Sample amplitude trop élevée: {}",
                sample
            );
        }
    }
}
