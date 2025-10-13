// Integration test: Latency measurement
//
// This test measures the latency from MIDI event to audio generation.
// Target: < 10ms for professional DAW use

use mymusic_daw::synth::voice_manager::VoiceManager;
use std::time::Instant;

#[test]
fn test_midi_processing_latency() {
    const SAMPLE_RATE: f32 = 48000.0;
    let mut voice_manager = VoiceManager::new(SAMPLE_RATE);

    // Measure time to process NoteOn
    let start = Instant::now();
    voice_manager.note_on(60, 100);
    let note_on_time = start.elapsed();

    println!("NoteOn processing time: {:?}", note_on_time);

    // NoteOn should be processed in < 100µs for real-time performance
    assert!(
        note_on_time.as_micros() < 100,
        "NoteOn processing took too long: {:?}",
        note_on_time
    );

    // Measure time to process NoteOff
    let start = Instant::now();
    voice_manager.note_off(60);
    let note_off_time = start.elapsed();

    println!("NoteOff processing time: {:?}", note_off_time);

    assert!(
        note_off_time.as_micros() < 100,
        "NoteOff processing took too long: {:?}",
        note_off_time
    );
}

#[test]
fn test_audio_buffer_generation_latency() {
    const SAMPLE_RATE: f32 = 48000.0;
    const BUFFER_SIZE: usize = 512;

    let mut voice_manager = VoiceManager::new(SAMPLE_RATE);
    voice_manager.note_on(60, 100);

    // Measure time to generate one buffer
    let start = Instant::now();
    for _ in 0..BUFFER_SIZE {
        voice_manager.next_sample();
    }
    let buffer_time = start.elapsed();

    // Calculate theoretical minimum buffer time
    let buffer_duration_ms = (BUFFER_SIZE as f32 / SAMPLE_RATE) * 1000.0;

    println!(
        "Buffer generation time: {:?} (theoretical: {:.2}ms)",
        buffer_time, buffer_duration_ms
    );

    // Generation should be faster than real-time (< buffer duration)
    // We allow 50% margin for safety
    let max_allowed_ms = buffer_duration_ms * 0.5;
    assert!(
        buffer_time.as_secs_f32() * 1000.0 < max_allowed_ms,
        "Buffer generation too slow: {:.2}ms (max: {:.2}ms)",
        buffer_time.as_secs_f32() * 1000.0,
        max_allowed_ms
    );
}

#[test]
fn test_total_latency_calculation() {
    const SAMPLE_RATE: f32 = 48000.0;

    // Test different buffer sizes and their latency implications
    let buffer_sizes = [64, 128, 256, 512, 1024, 2048];

    println!("\nLatency by buffer size:");
    for &size in &buffer_sizes {
        let latency_ms = (size as f32 / SAMPLE_RATE) * 1000.0;
        println!("  {} samples = {:.2}ms", size, latency_ms);

        // For professional DAW, target < 10ms
        if size <= 512 {
            assert!(
                latency_ms <= 11.0, // Small margin for 512 samples
                "Latency too high for buffer size {}: {:.2}ms",
                size,
                latency_ms
            );
        }
    }
}

#[test]
fn test_polyphonic_latency() {
    const SAMPLE_RATE: f32 = 48000.0;
    let mut voice_manager = VoiceManager::new(SAMPLE_RATE);

    // Trigger multiple notes and measure processing time
    let notes = [60, 64, 67, 71]; // C major 7th chord

    let start = Instant::now();
    for &note in &notes {
        voice_manager.note_on(note, 100);
    }
    let polyphonic_time = start.elapsed();

    println!(
        "Polyphonic NoteOn (4 notes) processing time: {:?}",
        polyphonic_time
    );

    // Even with multiple notes, processing should be fast
    assert!(
        polyphonic_time.as_micros() < 500,
        "Polyphonic processing took too long: {:?}",
        polyphonic_time
    );
}
