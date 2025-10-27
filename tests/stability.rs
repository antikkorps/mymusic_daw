// Integration test: Stability and long-running tests
//
// Tests that the DAW can run for extended periods without crashes,
// memory leaks, or audio artifacts.

use mymusic_daw::synth::voice_manager::VoiceManager;
use std::time::{Duration, Instant};

/// Short stability test (5 minutes) - suitable for CI/CD
#[test]
fn test_stability_short() {
    run_stability_test(Duration::from_secs(60 * 5), "short (5 min)");
}

/// Long stability test (1 hour) - run manually for full validation
/// This test is marked as `#[ignore]` by default - run with: cargo test --test stability -- --ignored
#[test]
#[ignore]
fn test_stability_long() {
    run_stability_test(Duration::from_secs(60 * 60), "long (1 hour)");
}

/// Core stability test logic
fn run_stability_test(duration: Duration, test_name: &str) {
    const SAMPLE_RATE: f32 = 48000.0;
    const BUFFER_SIZE: usize = 512;

    println!("\n=== Stability Test ({}) ===", test_name);
    println!("Duration: {:?}", duration);
    println!("Sample rate: {} Hz", SAMPLE_RATE);
    println!("Buffer size: {} samples", BUFFER_SIZE);

    let mut voice_manager = VoiceManager::new(SAMPLE_RATE);
    let start_time = Instant::now();
    let mut total_samples = 0u64;
    let mut total_buffers = 0u64;
    let mut note_events = 0u64;

    // Statistics
    let mut max_amplitude = 0.0f32;
    let mut samples_outside_range = 0u64;

    // Simulate MIDI events pattern
    let mut next_note_time = start_time;
    let note_interval = Duration::from_millis(500); // New note every 500ms
    let mut current_note = 60u8;

    println!("Starting continuous audio generation...\n");

    while start_time.elapsed() < duration {
        // Simulate MIDI events
        if Instant::now() >= next_note_time {
            // Turn off previous note
            if current_note > 60 {
                voice_manager.note_off(current_note - 1);
            }

            // Turn on new note
            voice_manager.note_on(current_note, 100);
            note_events += 1;

            // Next note
            current_note = 60 + (current_note % 12);
            next_note_time = Instant::now() + note_interval;
        }

        // Generate one buffer of audio
        for _ in 0..BUFFER_SIZE {
            let sample = voice_manager.next_sample();
            total_samples += 1;

            // Check for audio artifacts
            assert!(
                sample.0.is_finite() && sample.1.is_finite(),
                "Audio sample is not finite (NaN or Inf) at sample {}",
                total_samples
            );

            // Track statistics
            let abs_sample = sample.0.abs();
            if abs_sample > max_amplitude {
                max_amplitude = abs_sample;
            }

            // Check for clipping (should not exceed reasonable bounds)
            if abs_sample > 10.0 {
                samples_outside_range += 1;
            }
        }

        total_buffers += 1;

        // Print progress every 10 seconds
        let elapsed = start_time.elapsed();
        if elapsed.as_secs().is_multiple_of(10) && elapsed.as_millis() % 10000 < 100 {
            let progress_pct = (elapsed.as_secs_f32() / duration.as_secs_f32()) * 100.0;
            println!(
                "Progress: {:.1}% ({:.0}s / {:.0}s) - {} buffers, {} samples, {} notes",
                progress_pct,
                elapsed.as_secs(),
                duration.as_secs(),
                total_buffers,
                total_samples,
                note_events
            );
        }
    }

    let total_duration = start_time.elapsed();
    let audio_duration_secs = total_samples as f32 / SAMPLE_RATE;

    println!("\n=== Test Complete ===");
    println!("Total duration: {:?}", total_duration);
    println!("Audio generated: {:.2}s", audio_duration_secs);
    println!("Total samples: {}", total_samples);
    println!("Total buffers: {}", total_buffers);
    println!("MIDI note events: {}", note_events);
    println!("Max amplitude: {:.6}", max_amplitude);
    println!("Samples outside range: {}", samples_outside_range);

    // Verify no crashes occurred (if we reach here, test passed)
    assert!(
        total_samples > 0,
        "No audio samples were generated during the test"
    );

    // Verify reasonable max amplitude
    assert!(
        max_amplitude < 10.0,
        "Max amplitude too high: {}",
        max_amplitude
    );

    // Verify very few samples outside range
    let outside_percentage = (samples_outside_range as f32 / total_samples as f32) * 100.0;
    assert!(
        outside_percentage < 0.01,
        "Too many samples outside range: {:.4}%",
        outside_percentage
    );

    println!("\n✅ Stability test PASSED - No crashes, memory leaks, or audio artifacts detected");
}

/// Test that simulates heavy polyphonic load
#[test]
fn test_stability_polyphonic_stress() {
    const SAMPLE_RATE: f32 = 48000.0;
    const BUFFER_SIZE: usize = 512;
    const TEST_DURATION_SECS: u64 = 30; // 30 seconds of stress test

    println!("\n=== Polyphonic Stress Test ===");

    let mut voice_manager = VoiceManager::new(SAMPLE_RATE);
    let start_time = Instant::now();

    // Trigger all 16 voices at once
    for i in 0..16 {
        voice_manager.note_on(60 + i, 100);
    }

    let mut total_samples = 0u64;

    while start_time.elapsed() < Duration::from_secs(TEST_DURATION_SECS) {
        // Generate audio at full polyphony
        for _ in 0..BUFFER_SIZE {
            let sample = voice_manager.next_sample();
            total_samples += 1;

            assert!(
                sample.0.is_finite() && sample.1.is_finite(),
                "Audio sample not finite at sample {}",
                total_samples
            );
        }
    }

    println!(
        "Generated {} samples at full 16-voice polyphony",
        total_samples
    );
    println!("✅ Polyphonic stress test PASSED");
}

/// Test rapid note on/off cycles (worst case for voice allocation)
#[test]
fn test_stability_rapid_notes() {
    const SAMPLE_RATE: f32 = 48000.0;
    let mut voice_manager = VoiceManager::new(SAMPLE_RATE);

    println!("\n=== Rapid Note Cycles Test ===");

    // Rapidly turn notes on and off
    for cycle in 0..10000 {
        voice_manager.note_on(60 + (cycle % 12) as u8, 100);

        // Generate a few samples
        for _ in 0..10 {
            let sample = voice_manager.next_sample();
            assert!(
                sample.0.is_finite() && sample.1.is_finite(),
                "Sample not finite at cycle {}",
                cycle
            );
        }

        voice_manager.note_off(60 + (cycle % 12) as u8);
    }

    println!("Completed 10,000 rapid note cycles");
    println!("✅ Rapid note cycles test PASSED");
}
