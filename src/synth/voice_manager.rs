// Voice Manager - Polyphony handling

use super::oscillator::WaveformType;
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

    pub fn set_waveform(&mut self, waveform: WaveformType) {
        // Change waveform for all voices
        for voice in &mut self.voices {
            voice.set_waveform(waveform);
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

        // Désactiver la note du milieu
        vm.note_off(64);
        assert_eq!(vm.active_voice_count(), 2);

        // Désactiver les deux autres
        vm.note_off(60);
        vm.note_off(67);
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
    fn test_duplicate_notes() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);

        // Activer la même note plusieurs fois
        vm.note_on(60, 100);
        vm.note_on(60, 80);
        vm.note_on(60, 60);

        // Doit avoir 3 voix actives (polyphonie)
        assert_eq!(vm.active_voice_count(), 3);

        // note_off doit désactiver TOUTES les instances de cette note
        vm.note_off(60);
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
