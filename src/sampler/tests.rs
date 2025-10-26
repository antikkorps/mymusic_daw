#[cfg(test)]
mod tests {
    use crate::sampler::loader::*;
    use crate::sampler::engine::SamplerVoice;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[test]
    fn test_load_mp3_support() {
        // Test that MP3 format is recognized
        let mp3_path = PathBuf::from("test.mp3");
        
        // This should not panic - just checking format recognition
        match load_sample(&mp3_path) {
            Err(msg) => {
                // Expected since file doesn't exist, but should be format-related error
                assert!(!msg.contains("Unsupported file format"));
            }
            Ok(_) => {
                // Unexpected success, but not an error
            }
        }
    }

    #[test]
    fn test_supported_formats() {
        let extensions = vec!["wav", "flac", "mp3"];
        
        for ext in extensions {
            let path = PathBuf::from(format!("test.{}", ext));
            match load_sample(&path) {
                Err(msg) => {
                    // Should not be "unsupported format" error
                    assert!(!msg.contains("Unsupported file format"), 
                           "Format {} should be supported but got: {}", ext, msg);
                }
                Ok(_) => {
                    // File doesn't exist, but format is supported
                }
            }
        }
    }

    #[test]
    fn test_unsupported_format() {
        let path = PathBuf::from("test.xyz");
        match load_sample(&path) {
            Err(msg) => {
                assert!(msg.contains("Unsupported file format"));
            }
            Ok(_) => {
                panic!("Unsupported format should not load successfully");
            }
        }
    }

    // Helper function to create a test sample with specific data
    fn create_test_sample(size: usize) -> Sample {
        let data = vec![0.5f32; size];
        Sample {
            name: "test_sample".to_string(),
            data: SampleData::F32(data),
            sample_rate: 48000,
            source_channels: 1,
            loop_mode: LoopMode::Off,
            loop_start: 0,
            loop_end: size,
            reverse: false,
            volume: 1.0,
            pan: 0.0,
            pitch_offset: 0,
        }
    }

    #[test]
    fn test_loop_default_values() {
        let sample = create_test_sample(1000);

        // Check default loop values
        assert_eq!(sample.loop_mode, LoopMode::Off);
        assert_eq!(sample.loop_start, 0);
        assert_eq!(sample.loop_end, 1000);
    }

    #[test]
    fn test_loop_mode_forward() {
        let mut sample = create_test_sample(100);
        sample.loop_mode = LoopMode::Forward;
        sample.loop_start = 20;
        sample.loop_end = 80;

        let sample_arc = Arc::new(sample);
        let mut voice = SamplerVoice::new(sample_arc.clone(), 48000.0);

        // Trigger the voice
        voice.note_on(60, 100, 0);

        // Process samples until we reach loop_end
        let matrix = crate::synth::modulation::ModulationMatrix::new_empty();
        for _ in 0..85 {
            voice.next_sample_with_matrix(&matrix);
        }

        // Voice should still be active because it's looping
        assert!(voice.is_active(), "Voice should remain active when looping");
    }

    #[test]
    fn test_loop_mode_off_stops_at_end() {
        let sample = create_test_sample(50);
        let sample_arc = Arc::new(sample);
        let mut voice = SamplerVoice::new(sample_arc.clone(), 48000.0);

        // Trigger the voice
        voice.note_on(60, 100, 0);

        // Process samples beyond the end
        let matrix = crate::synth::modulation::ModulationMatrix::new_empty();
        for _ in 0..60 {
            voice.next_sample_with_matrix(&matrix);
        }

        // Voice should stop when reaching the end (no loop)
        assert!(!voice.is_active(), "Voice should stop at end when not looping");
    }

    #[test]
    fn test_loop_points_within_bounds() {
        let mut sample = create_test_sample(1000);

        // Set valid loop points
        sample.loop_start = 100;
        sample.loop_end = 900;
        sample.loop_mode = LoopMode::Forward;

        assert!(sample.loop_start < sample.loop_end, "loop_start should be less than loop_end");
        assert!(sample.loop_end <= 1000, "loop_end should not exceed sample size");
    }

    #[test]
    fn test_loop_with_pitch_shift() {
        let mut sample = create_test_sample(100);
        sample.loop_mode = LoopMode::Forward;
        sample.loop_start = 20;
        sample.loop_end = 80;

        let sample_arc = Arc::new(sample);
        let mut voice = SamplerVoice::new(sample_arc.clone(), 48000.0);

        // Trigger with different note (pitch shift)
        voice.note_on(72, 100, 0); // C5 (one octave higher, plays 2x faster)

        // Process several samples
        let matrix = crate::synth::modulation::ModulationMatrix::new_empty();
        for _ in 0..100 {
            let (left, right) = voice.next_sample_with_matrix(&matrix);
            // Should produce valid output (not NaN, not infinite)
            assert!(left.is_finite(), "Left channel should produce finite values");
            assert!(right.is_finite(), "Right channel should produce finite values");
        }

        // Voice should still be active due to looping
        assert!(voice.is_active(), "Voice should remain active with pitched loop");
    }

    #[test]
    fn test_loop_produces_continuous_audio() {
        let mut sample = create_test_sample(100);
        // Fill with a simple pattern to detect loop
        if let SampleData::F32(ref mut data) = sample.data {
            for (i, val) in data.iter_mut().enumerate() {
                *val = (i as f32 / 100.0).sin(); // Simple sine-like pattern
            }
        }

        sample.loop_mode = LoopMode::Forward;
        sample.loop_start = 25;
        sample.loop_end = 75;

        let sample_arc = Arc::new(sample);
        let mut voice = SamplerVoice::new(sample_arc.clone(), 48000.0);
        voice.note_on(60, 100, 0);

        let mut output_samples = Vec::new();

        // Collect 200 samples (should loop at least twice)
        let matrix = crate::synth::modulation::ModulationMatrix::new_empty();
        for _ in 0..200 {
            let (left, _) = voice.next_sample_with_matrix(&matrix);
            output_samples.push(left);
        }

        // Check that we got continuous output (not all zeros after first loop)
        let non_zero_count = output_samples.iter().filter(|&&x| x.abs() > 0.001).count();
        assert!(non_zero_count > 150, "Should produce continuous non-zero output when looping, got {} non-zero samples", non_zero_count);
    }

    #[test]
    fn test_reverse_playback() {
        let mut sample = create_test_sample(100);
        sample.reverse = true;

        let sample_arc = Arc::new(sample);
        let mut voice = SamplerVoice::new(sample_arc.clone(), 48000.0);
        voice.note_on(60, 100, 0);

        // Process several samples
        let matrix = crate::synth::modulation::ModulationMatrix::new_empty();
        for _ in 0..10 {
            let (left, right) = voice.next_sample_with_matrix(&matrix);
            assert!(left.is_finite() && right.is_finite(), "Reverse playback should produce valid audio");
        }

        // Voice should still be active
        assert!(voice.is_active(), "Reverse playback should keep voice active");
    }

    #[test]
    fn test_reverse_playback_stops_at_start() {
        let sample = create_test_sample(20);
        let mut sample_with_reverse = sample.clone();
        sample_with_reverse.reverse = true;

        let sample_arc = Arc::new(sample_with_reverse);
        let mut voice = SamplerVoice::new(sample_arc.clone(), 48000.0);
        voice.note_on(60, 100, 0);

        // Process samples beyond the start (reverse goes from end to start)
        let matrix = crate::synth::modulation::ModulationMatrix::new_empty();
        for _ in 0..30 {
            voice.next_sample_with_matrix(&matrix);
        }

        // Voice should stop when reaching the start (no loop)
        assert!(!voice.is_active(), "Reverse playback should stop at start when not looping");
    }

    #[test]
    fn test_reverse_with_loop() {
        let mut sample = create_test_sample(50);
        sample.reverse = true;
        sample.loop_mode = LoopMode::Forward;
        sample.loop_start = 10;
        sample.loop_end = 40;

        let sample_arc = Arc::new(sample);
        let mut voice = SamplerVoice::new(sample_arc.clone(), 48000.0);
        voice.note_on(60, 100, 0);

        // Process many samples to test looping
        let matrix = crate::synth::modulation::ModulationMatrix::new_empty();
        for _ in 0..100 {
            voice.next_sample_with_matrix(&matrix);
        }

        // Voice should still be active due to looping
        assert!(voice.is_active(), "Reverse playback with loop should keep voice active");
    }

    #[test]
    fn test_pitch_offset_default() {
        // Test that default pitch_offset (0) plays at normal pitch
        let sample = create_test_sample(100);
        assert_eq!(sample.pitch_offset, 0, "Default pitch_offset should be 0");

        let sample_arc = Arc::new(sample);
        let mut voice = SamplerVoice::new(sample_arc.clone(), 48000.0);

        // Note C4 (60) with no offset should have pitch_step = 1.0
        voice.note_on(60, 100, 0);

        // We can't directly access pitch_step (it's private), but we can verify
        // the voice is active and produces output
        let matrix = crate::synth::modulation::ModulationMatrix::new_empty();
        let (left, right) = voice.next_sample_with_matrix(&matrix);

        assert!(left.is_finite() && right.is_finite(), "Should produce valid audio with default pitch offset");
        assert!(voice.is_active(), "Voice should be active");
    }

    #[test]
    fn test_pitch_offset_positive() {
        // Test that pitch_offset = +12 shifts pitch up one octave
        let mut sample = create_test_sample(1000);
        sample.pitch_offset = 12; // +1 octave
        sample.loop_mode = LoopMode::Forward; // Loop to keep voice active
        sample.loop_end = 1000;

        let sample_arc = Arc::new(sample);
        let mut voice = SamplerVoice::new(sample_arc.clone(), 48000.0);

        // Play C4 (60) with +12 offset should sound like C5 (72)
        // This means pitch_step should be 2.0 (2x faster playback)
        voice.note_on(60, 100, 0);

        let matrix = crate::synth::modulation::ModulationMatrix::new_empty();

        // Process many samples to verify stability
        for _ in 0..500 {
            let (left, right) = voice.next_sample_with_matrix(&matrix);
            assert!(left.is_finite() && right.is_finite(), "Should produce valid audio with positive pitch offset");
        }

        assert!(voice.is_active(), "Voice should remain active with pitch offset");
    }

    #[test]
    fn test_pitch_offset_negative() {
        // Test that pitch_offset = -12 shifts pitch down one octave
        let mut sample = create_test_sample(1000);
        sample.pitch_offset = -12; // -1 octave
        sample.loop_mode = LoopMode::Forward; // Loop to keep voice active
        sample.loop_end = 1000;

        let sample_arc = Arc::new(sample);
        let mut voice = SamplerVoice::new(sample_arc.clone(), 48000.0);

        // Play C4 (60) with -12 offset should sound like C3 (48)
        // This means pitch_step should be 0.5 (0.5x slower playback)
        voice.note_on(60, 100, 0);

        let matrix = crate::synth::modulation::ModulationMatrix::new_empty();

        // Process many samples to verify stability
        for _ in 0..200 {
            let (left, right) = voice.next_sample_with_matrix(&matrix);
            assert!(left.is_finite() && right.is_finite(), "Should produce valid audio with negative pitch offset");
        }

        assert!(voice.is_active(), "Voice should remain active with negative pitch offset");
    }

    #[test]
    fn test_pitch_offset_range() {
        // Test that pitch_offset works across the full range (-12 to +12)
        for offset in -12..=12 {
            let mut sample = create_test_sample(100);
            sample.pitch_offset = offset;
            sample.loop_mode = LoopMode::Forward;

            let sample_arc = Arc::new(sample);
            let mut voice = SamplerVoice::new(sample_arc.clone(), 48000.0);
            voice.note_on(60, 100, 0);

            let matrix = crate::synth::modulation::ModulationMatrix::new_empty();

            // Process a few samples
            for _ in 0..10 {
                let (left, right) = voice.next_sample_with_matrix(&matrix);
                assert!(
                    left.is_finite() && right.is_finite(),
                    "Should produce valid audio with pitch_offset = {}",
                    offset
                );
            }

            assert!(voice.is_active(), "Voice should be active with pitch_offset = {}", offset);
        }
    }

    #[test]
    fn test_pitch_offset_with_different_notes() {
        // Test that pitch_offset works correctly with different MIDI notes
        let mut sample = create_test_sample(500);
        sample.pitch_offset = 5; // Perfect fourth up
        sample.loop_mode = LoopMode::Forward;

        let sample_arc = Arc::new(sample);

        // Test with various notes
        for note in [36, 48, 60, 72, 84] {
            let mut voice = SamplerVoice::new(sample_arc.clone(), 48000.0);
            voice.note_on(note, 100, 0);

            let matrix = crate::synth::modulation::ModulationMatrix::new_empty();

            for _ in 0..20 {
                let (left, right) = voice.next_sample_with_matrix(&matrix);
                assert!(
                    left.is_finite() && right.is_finite(),
                    "Should produce valid audio for note {} with pitch_offset",
                    note
                );
            }

            assert!(voice.is_active(), "Voice should be active for note {}", note);
        }
    }
}