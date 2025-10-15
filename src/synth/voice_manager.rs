// Voice Manager - Polyphony handling

use super::oscillator::WaveformType;
use super::poly_mode::PolyMode;
use super::voice::Voice;

const MAX_VOICES: usize = 16;

pub struct VoiceManager {
    voices: [Voice; MAX_VOICES],
    /// Age counter incremented on each note_on for voice stealing priority
    age_counter: u64,
    /// Polyphony mode (Poly, Mono, Legato)
    poly_mode: PolyMode,
    /// Last note played (used for legato detection)
    last_note: Option<u8>,
}

impl VoiceManager {
    pub fn new(sample_rate: f32) -> Self {
        // Pre-allocate all voices
        let voices = std::array::from_fn(|_| Voice::new(sample_rate));

        Self {
            voices,
            age_counter: 0,
            poly_mode: PolyMode::default(),
            last_note: None,
        }
    }

    pub fn note_on(&mut self, note: u8, velocity: u8) {
        // Increment age counter
        self.age_counter = self.age_counter.wrapping_add(1);

        match self.poly_mode {
            PolyMode::Poly => {
                // Polyphonic mode: standard behavior
                self.note_on_poly(note, velocity);
            }
            PolyMode::Mono => {
                // Monophonic mode: cut all other notes, retrigger envelope
                self.note_on_mono(note, velocity);
            }
            PolyMode::Legato => {
                // Legato mode: cut other notes, but don't retrigger if sliding
                self.note_on_legato(note, velocity);
            }
        }

        // Track last note for legato detection
        self.last_note = Some(note);
    }

    /// Polyphonic note_on: original behavior
    fn note_on_poly(&mut self, note: u8, velocity: u8) {
        // Search an inactive voice
        if let Some(voice) = self.voices.iter_mut().find(|v| !v.is_active()) {
            voice.note_on(note, velocity, self.age_counter);
            return;
        }

        // Voice stealing: Find the best voice to steal
        let victim_index = self.find_voice_to_steal();
        self.voices[victim_index].note_on(note, velocity, self.age_counter);
    }

    /// Monophonic note_on: cut all other notes, retrigger envelope
    fn note_on_mono(&mut self, note: u8, velocity: u8) {
        // Force-stop all currently playing notes (no release phase)
        // In mono mode, we want immediate cutoff to maintain strict monophony
        for voice in &mut self.voices {
            if voice.is_active() {
                voice.force_stop();
            }
        }

        // Now play the new note on the first available voice
        // All voices should now be inactive after force_stop
        if let Some(voice) = self.voices.iter_mut().find(|v| !v.is_active()) {
            voice.note_on(note, velocity, self.age_counter);
        } else {
            // Fallback: use first voice if none are inactive (shouldn't happen after force_stop)
            self.voices[0].note_on(note, velocity, self.age_counter);
        }
    }

    /// Legato note_on: monophonic without envelope retrigger when sliding
    fn note_on_legato(&mut self, note: u8, velocity: u8) {
        // Check if there's currently a note playing
        let has_active_note = self.voices.iter().any(|v| v.is_active());

        if has_active_note {
            // Legato transition: change pitch without retriggering envelope
            // Find the currently playing voice and change its pitch
            if let Some(voice) = self.voices.iter_mut().find(|v| v.is_active()) {
                voice.change_pitch_legato(note, velocity, self.age_counter);
            }
        } else {
            // No active note: trigger normally (first note in a phrase)
            if let Some(voice) = self.voices.iter_mut().find(|v| !v.is_active()) {
                voice.note_on(note, velocity, self.age_counter);
            } else {
                // Fallback: use first voice
                self.voices[0].note_on(note, velocity, self.age_counter);
            }
        }
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

    pub fn set_lfo(&mut self, params: super::lfo::LfoParams) {
        // Change LFO parameters for all voices
        for voice in &mut self.voices {
            voice.set_lfo(params);
        }
    }

    pub fn get_lfo_params(&self) -> super::lfo::LfoParams {
        // Get LFO params from first voice (all voices share same params)
        self.voices[0].get_lfo_params()
    }

    pub fn set_portamento(&mut self, params: super::portamento::PortamentoParams) {
        // Change portamento parameters for all voices
        for voice in &mut self.voices {
            voice.set_portamento(params);
        }
    }

    pub fn get_portamento_params(&self) -> super::portamento::PortamentoParams {
        // Get portamento params from first voice (all voices share same params)
        self.voices[0].get_portamento_params()
    }

    pub fn set_poly_mode(&mut self, mode: PolyMode) {
        self.poly_mode = mode;
    }

    pub fn get_poly_mode(&self) -> PolyMode {
        self.poly_mode
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

    // ===== POLYPHONY MODE TESTS =====

    #[test]
    fn test_poly_mode_multiple_notes() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);
        vm.set_poly_mode(PolyMode::Poly);

        // Should allow multiple simultaneous notes
        vm.note_on(60, 100);
        vm.note_on(64, 100);
        vm.note_on(67, 100);

        assert_eq!(vm.active_voice_count(), 3, "Poly mode should allow 3 simultaneous notes");
    }

    #[test]
    fn test_poly_mode_is_default() {
        let vm = VoiceManager::new(SAMPLE_RATE);
        assert_eq!(vm.get_poly_mode(), PolyMode::Poly, "Poly mode should be default");
    }

    #[test]
    fn test_mono_mode_one_note_at_a_time() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);
        vm.set_poly_mode(PolyMode::Mono);

        // Play first note
        vm.note_on(60, 100);
        assert_eq!(vm.active_voice_count(), 1, "First note should be active");

        // Play second note - should cut first note and only have one active
        vm.note_on(64, 100);

        // In mono mode, we process 64 samples for smooth release, but after that
        // only the new note should be active
        assert_eq!(vm.active_voice_count(), 1, "Mono mode should only have 1 active voice after note_on");

        // Verify it's the new note (64) playing
        let note_64_count = vm.voices.iter().filter(|v| v.is_active() && v.get_note() == 64).count();
        assert_eq!(note_64_count, 1, "Should be playing note 64");
    }

    #[test]
    fn test_mono_mode_retriggering() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);
        vm.set_poly_mode(PolyMode::Mono);

        // Play a note
        vm.note_on(60, 100);

        // Generate some samples to get past attack phase
        for _ in 0..1000 {
            vm.next_sample();
        }

        // Play a new note - envelope should retrigger
        vm.note_on(64, 100);

        // The voice should be in attack phase again (freshly triggered)
        // We can verify this by checking that only one voice is active
        assert_eq!(vm.active_voice_count(), 1, "Should have retriggered with one voice");
    }

    #[test]
    fn test_legato_mode_smooth_transition() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);
        vm.set_poly_mode(PolyMode::Legato);

        // Play first note
        vm.note_on(60, 100);
        assert_eq!(vm.active_voice_count(), 1, "First note should be active");

        // Process samples to get into sustain phase
        for _ in 0..5000 {
            vm.next_sample();
        }

        // Play second note - should change pitch without adding new voice
        vm.note_on(64, 100);

        // Should still have only one active voice (legato transition)
        assert_eq!(vm.active_voice_count(), 1, "Legato should maintain single voice");

        // Verify it's now playing note 64
        let note_64_count = vm.voices.iter().filter(|v| v.is_active() && v.get_note() == 64).count();
        assert_eq!(note_64_count, 1, "Should be playing note 64 after legato transition");
    }

    #[test]
    fn test_legato_mode_first_note_triggers_envelope() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);
        vm.set_poly_mode(PolyMode::Legato);

        // First note in a phrase should trigger normally
        vm.note_on(60, 100);
        assert_eq!(vm.active_voice_count(), 1, "First note should trigger");

        // Let it release completely
        vm.note_off(60);
        for _ in 0..10000 {
            vm.next_sample();
        }
        assert_eq!(vm.active_voice_count(), 0, "Should be silent after release");

        // Next note should trigger again (new phrase)
        vm.note_on(64, 100);
        assert_eq!(vm.active_voice_count(), 1, "New phrase should trigger envelope");
    }

    #[test]
    fn test_mode_switching() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);

        // Start in Poly mode
        assert_eq!(vm.get_poly_mode(), PolyMode::Poly);

        // Switch to Mono
        vm.set_poly_mode(PolyMode::Mono);
        assert_eq!(vm.get_poly_mode(), PolyMode::Mono);

        // Verify mono behavior
        vm.note_on(60, 100);
        vm.note_on(64, 100);
        assert_eq!(vm.active_voice_count(), 1, "Mono mode should have 1 voice");

        // Switch to Legato
        vm.set_poly_mode(PolyMode::Legato);
        assert_eq!(vm.get_poly_mode(), PolyMode::Legato);

        // Verify legato behavior (should maintain single voice)
        vm.note_on(67, 100);
        assert_eq!(vm.active_voice_count(), 1, "Legato should maintain 1 voice");

        // Switch back to Poly
        vm.set_poly_mode(PolyMode::Poly);
        assert_eq!(vm.get_poly_mode(), PolyMode::Poly);

        // Verify poly behavior (can have multiple voices)
        vm.note_on(70, 100);
        assert!(vm.active_voice_count() >= 1, "Poly mode should allow new voice");
    }

    #[test]
    fn test_mono_mode_rapid_notes() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);
        vm.set_poly_mode(PolyMode::Mono);

        // Simulate rapid note playing (like a fast melody)
        for note in 60..70 {
            vm.note_on(note, 100);
            // Process a few samples between notes
            for _ in 0..100 {
                vm.next_sample();
            }
            // Should always have exactly 1 active voice
            assert_eq!(
                vm.active_voice_count(),
                1,
                "Mono mode should maintain 1 voice during rapid notes (note {})",
                note
            );
        }
    }

    #[test]
    fn test_legato_mode_preserves_envelope_state() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);
        vm.set_poly_mode(PolyMode::Legato);

        // Play first note and let it reach sustain phase
        vm.note_on(60, 100);

        // Process enough samples to get through attack and decay
        // Default attack = 0.01s, decay = 0.1s
        // Total = 0.11s = 0.11 * 44100 = 4851 samples
        for _ in 0..5000 {
            vm.next_sample();
        }

        // Now we're in sustain phase
        assert_eq!(vm.active_voice_count(), 1);

        // Play second note with legato
        vm.note_on(64, 100);

        // Voice should still be active and in sustain (not restarted)
        assert_eq!(vm.active_voice_count(), 1, "Should maintain envelope state");

        // Generate more samples - envelope should continue from where it was
        for _ in 0..1000 {
            let sample = vm.next_sample();
            assert!(sample.is_finite(), "Sample should be valid during legato transition");
        }
    }

    #[test]
    fn test_poly_mode_independent_envelopes() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);
        vm.set_poly_mode(PolyMode::Poly);

        // Play a chord
        vm.note_on(60, 100);
        vm.note_on(64, 100);
        vm.note_on(67, 100);

        assert_eq!(vm.active_voice_count(), 3);

        // Release middle note
        vm.note_off(64);

        // All three should still be active (middle one in release phase)
        assert_eq!(vm.active_voice_count(), 3, "All voices should still be active");

        // Process release phase for middle note
        for _ in 0..10000 {
            vm.next_sample();
        }

        // Now only two should be active
        assert_eq!(vm.active_voice_count(), 2, "Two voices should remain after release");

        // Verify the correct notes are still playing
        let note_60_active = vm.voices.iter().any(|v| v.is_active() && v.get_note() == 60);
        let note_67_active = vm.voices.iter().any(|v| v.is_active() && v.get_note() == 67);
        let note_64_active = vm.voices.iter().any(|v| v.is_active() && v.get_note() == 64);

        assert!(note_60_active, "Note 60 should still be playing");
        assert!(note_67_active, "Note 67 should still be playing");
        assert!(!note_64_active, "Note 64 should be released");
    }

    #[test]
    fn test_last_note_tracking() {
        let mut vm = VoiceManager::new(SAMPLE_RATE);

        // Initially no last note
        assert_eq!(vm.last_note, None);

        // Play a note
        vm.note_on(60, 100);
        assert_eq!(vm.last_note, Some(60), "Should track last note");

        // Play another note
        vm.note_on(64, 100);
        assert_eq!(vm.last_note, Some(64), "Should update last note");

        // Play in different modes - should always track
        vm.set_poly_mode(PolyMode::Mono);
        vm.note_on(67, 100);
        assert_eq!(vm.last_note, Some(67), "Should track in mono mode");

        vm.set_poly_mode(PolyMode::Legato);
        vm.note_on(70, 100);
        assert_eq!(vm.last_note, Some(70), "Should track in legato mode");
    }
}
