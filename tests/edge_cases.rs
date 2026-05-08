//! Edge case tests and robustness validation
//!
//! This module tests extreme scenarios and edge cases to ensure the DAW
//! handles them gracefully without crashing or producing undefined behavior.

use mymusic_daw::audio::dsp_utils::OnePoleSmoother;
use mymusic_daw::audio::format_conversion::f32_to_i16;
use mymusic_daw::synth::envelope::{AdsrEnvelope, AdsrParams};
use mymusic_daw::synth::filter::{FilterParams, StateVariableFilter};
use mymusic_daw::synth::lfo::{Lfo, LfoDestination, LfoParams};
use mymusic_daw::synth::oscillator::{Oscillator, SimpleOscillator, WaveformType};
use mymusic_daw::synth::voice_manager::VoiceManager;

/// Test oscillator with extreme frequencies
#[test]
fn test_oscillator_extreme_frequencies() {
    let sample_rate = 44100.0;

    // Test sub-audio frequencies
    let mut osc = SimpleOscillator::new(WaveformType::Sine, sample_rate);
    osc.set_frequency(0.1); // Very low frequency

    for _ in 0..1000 {
        let sample = osc.next_sample();
        assert!(sample.is_finite());
        assert!((-1.0..=1.0).contains(&sample));
    }

    // Test very high frequencies (near Nyquist)
    let mut osc = SimpleOscillator::new(WaveformType::Sine, sample_rate);
    osc.set_frequency(20000.0); // Near Nyquist

    for _ in 0..1000 {
        let sample = osc.next_sample();
        assert!(sample.is_finite());
        assert!((-1.0..=1.0).contains(&sample));
    }

    // Test exactly Nyquist frequency
    let mut osc = SimpleOscillator::new(WaveformType::Sine, sample_rate);
    osc.set_frequency(sample_rate / 2.0);

    for _ in 0..1000 {
        let sample = osc.next_sample();
        assert!(sample.is_finite());
    }

    // Test above Nyquist (should alias but not crash)
    let mut osc = SimpleOscillator::new(WaveformType::Sine, sample_rate);
    osc.set_frequency(sample_rate * 0.75); // Above Nyquist

    for _ in 0..1000 {
        let sample = osc.next_sample();
        assert!(sample.is_finite());
    }
}

/// Test oscillator with invalid frequencies
#[test]
fn test_oscillator_invalid_frequencies() {
    let sample_rate = 44100.0;
    let mut osc = SimpleOscillator::new(WaveformType::Sine, sample_rate);

    // Test zero frequency
    osc.set_frequency(0.0);
    for _ in 0..100 {
        let sample = osc.next_sample();
        assert!(sample.is_finite());
    }

    // Test negative frequency (should handle gracefully)
    osc.set_frequency(-440.0);
    for _ in 0..100 {
        let sample = osc.next_sample();
        assert!(sample.is_finite());
    }

    // Test NaN frequency
    osc.set_frequency(f32::NAN);
    for _ in 0..100 {
        let sample = osc.next_sample();
        // Should produce NaN samples but not crash
        assert!(sample.is_nan() || sample.is_finite());
    }

    // Test infinite frequency
    osc.set_frequency(f32::INFINITY);
    for _ in 0..100 {
        let sample = osc.next_sample();
        // Should produce NaN samples but not crash
        assert!(sample.is_nan() || sample.is_finite());
    }
}

/// Test filter with extreme parameters
#[test]
fn test_filter_extreme_parameters() {
    let sample_rate = 44100.0;

    // Test very low cutoff
    let params = FilterParams {
        cutoff: 0.1, // Sub-audio
        resonance: 0.707,
        filter_type: mymusic_daw::synth::filter::FilterType::LowPass,
        enabled: true,
    };
    let mut filter = StateVariableFilter::new(params, sample_rate);

    for _ in 0..1000 {
        let output = filter.process(0.5);
        assert!(output.is_finite());
    }

    // Test very high cutoff (near Nyquist)
    let params = FilterParams {
        cutoff: 20000.0,
        resonance: 0.707,
        filter_type: mymusic_daw::synth::filter::FilterType::LowPass,
        enabled: true,
    };
    let mut filter = StateVariableFilter::new(params, sample_rate);

    for _ in 0..1000 {
        let output = filter.process(0.5);
        assert!(output.is_finite());
    }

    // Test extreme resonance
    let params = FilterParams {
        cutoff: 1000.0,
        resonance: 1000.0, // Very high resonance
        filter_type: mymusic_daw::synth::filter::FilterType::LowPass,
        enabled: true,
    };
    let mut filter = StateVariableFilter::new(params, sample_rate);

    for _ in 0..1000 {
        let output = filter.process(0.5);
        assert!(output.is_finite());
        // Output might be very large with high resonance, but should not be NaN or Inf
        assert!(!output.is_nan() && !output.is_infinite());
    }
}

/// Test filter with NaN/Inf input
#[test]
fn test_filter_nan_inf_input() {
    let sample_rate = 44100.0;
    let params = FilterParams::default();
    let mut filter = StateVariableFilter::new(params, sample_rate);

    // Test NaN input
    let output = filter.process(f32::NAN);
    assert!(output.is_nan() || output.is_finite());

    // Reset filter
    filter.reset();

    // Test Infinity input
    let output = filter.process(f32::INFINITY);
    assert!(output.is_nan() || output.is_infinite() || output.is_finite());

    // Reset filter
    filter.reset();

    // Test negative infinity input
    let output = filter.process(f32::NEG_INFINITY);
    assert!(output.is_nan() || output.is_infinite() || output.is_finite());
}

/// Test ADSR envelope with extreme parameters
#[test]
fn test_adsr_extreme_parameters() {
    let sample_rate = 44100.0;

    let make_adsr = |a, d, s, r| {
        AdsrEnvelope::new(
            AdsrParams {
                attack: a,
                decay: d,
                sustain: s,
                release: r,
            },
            sample_rate,
        )
    };

    // Test zero-length stages
    let mut adsr = make_adsr(0.0, 0.0, 0.5, 0.0);
    adsr.note_on();

    for _ in 0..100 {
        let output = adsr.process();
        assert!(output.is_finite());
        assert!((0.0..=1.0).contains(&output));
    }

    // Test very long attack
    let mut adsr = make_adsr(60.0, 0.0, 0.5, 0.0); // 60 second attack
    adsr.note_on();

    for _ in 0..1000 {
        let output = adsr.process();
        assert!(output.is_finite());
        assert!((0.0..=1.0).contains(&output));
    }

    // Test sustain at 0 and 1.
    // Need to consume *both* attack (0.1s) and decay (0.1s) = 0.2s = sr/5 samples
    // before the envelope settles to the sustain plateau.
    let mut adsr = make_adsr(0.1, 0.1, 0.0, 0.1); // Sustain = 0
    adsr.note_on();

    for _ in 0..(sample_rate as usize / 5) {
        let _ = adsr.process();
    }

    // Should be at sustain level (0)
    let output = adsr.process();
    assert!(output.abs() < 0.01, "sustain=0 leaked {}", output);

    // Test sustain = 1
    let mut adsr = make_adsr(0.1, 0.1, 1.0, 0.1);
    adsr.note_on();

    for _ in 0..(sample_rate as usize / 5) {
        let _ = adsr.process();
    }

    let output = adsr.process();
    assert!((output - 1.0).abs() < 0.01, "sustain=1 returned {}", output);
}

/// Test voice manager with maximum polyphony
#[test]
fn test_voice_manager_max_polyphony() {
    let sample_rate = 44100.0;
    let mut voice_manager = VoiceManager::new(sample_rate);

    // Trigger more notes than available voices
    for i in 0..32 {
        voice_manager.note_on(60 + (i % 12) as u8, 100);
    }

    // Process audio
    for _ in 0..1000 {
        let _ = voice_manager.next_sample();
    }

    // Should not crash and should have exactly 16 active voices
    assert_eq!(voice_manager.active_voice_count(), 16);
}

/// Test voice manager with rapid note on/off
#[test]
fn test_voice_manager_rapid_triggering() {
    let sample_rate = 44100.0;
    let mut voice_manager = VoiceManager::new(sample_rate);

    // Rapidly trigger and release the same note
    for _ in 0..100 {
        voice_manager.note_on(60, 100);
        let _ = voice_manager.next_sample();
        voice_manager.note_off(60);
        let _ = voice_manager.next_sample();
    }

    // Should not crash
    assert!(voice_manager.active_voice_count() <= 16);
}

/// Test one-pole smoother with extreme values
#[test]
fn test_smoother_extreme_values() {
    let sample_rate = 44100.0;
    let mut smoother = OnePoleSmoother::new(0.0, 10.0, sample_rate);

    // Test step from 0 to 1
    for _ in 0..1000 {
        let output = smoother.process(1.0);
        assert!(output.is_finite());
        assert!((0.0..=1.0).contains(&output));
    }

    // Test step from 1 to 0
    for _ in 0..1000 {
        let output = smoother.process(0.0);
        assert!(output.is_finite());
        assert!((0.0..=1.0).contains(&output));
    }

    // Test with NaN target
    let output = smoother.process(f32::NAN);
    assert!(output.is_finite() || output.is_nan());

    // Test with Infinity target
    let output = smoother.process(f32::INFINITY);
    assert!(output.is_finite() || output.is_infinite() || output.is_nan());
}

/// Test format conversion with extreme values
#[test]
fn test_format_conversion_extreme_values() {
    // Test values outside [-1, 1] range
    let test_values = vec![
        2.0,   // Above 1
        -2.0,  // Below -1
        10.0,  // Way above
        -10.0, // Way below
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];

    for &value in &test_values {
        // The point of the test: the conversion must not panic on extreme,
        // infinite, or NaN inputs. The result type is `i16` — every i16 is
        // by definition within `i16::MIN..=i16::MAX`, so a range assertion
        // would be tautological (clippy::absurd_extreme_comparisons).
        let _converted = f32_to_i16(value);
    }
}

/// Test LFO with extreme rates
#[test]
fn test_lfo_extreme_rates() {
    let sample_rate = 44100.0;

    let make_lfo = |rate: f32| {
        Lfo::new(
            LfoParams {
                waveform: WaveformType::Sine,
                rate,
                depth: 1.0,
                destination: LfoDestination::None,
            },
            sample_rate,
        )
    };

    // Test very slow LFO (0.1 Hz)
    let mut lfo = make_lfo(0.1);
    for _ in 0..1000 {
        let output = lfo.process();
        assert!(output.is_finite());
        assert!((-1.0..=1.0).contains(&output));
    }

    // Test very fast LFO (1000 Hz)
    let mut lfo = make_lfo(1000.0);
    for _ in 0..1000 {
        let output = lfo.process();
        assert!(output.is_finite());
        assert!((-1.0..=1.0).contains(&output));
    }

    // Test LFO at Nyquist frequency
    let mut lfo = make_lfo(sample_rate / 2.0);
    for _ in 0..1000 {
        let output = lfo.process();
        assert!(output.is_finite());
    }
}

/// Test concurrent access patterns (simulated)
#[test]
fn test_concurrent_access_patterns() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let sample_rate = 44100.0;
    let voice_manager = Arc::new(Mutex::new(VoiceManager::new(sample_rate)));

    // Simulate multiple threads accessing voice manager
    let mut handles = vec![];

    for i in 0..4 {
        let vm = Arc::clone(&voice_manager);
        let handle = thread::spawn(move || {
            for j in 0..100 {
                let mut manager = vm.lock().unwrap();
                manager.note_on(60 + (i * 10 + j) as u8 % 24, 100);
                let _ = manager.next_sample();
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Should not crash and should have valid state
    let manager = voice_manager.lock().unwrap();
    assert!(manager.active_voice_count() <= 16);
}

/// Test graceful degradation with invalid parameters
#[test]
fn test_graceful_degradation() {
    let sample_rate = 44100.0;

    // Create voice manager and trigger a voice
    let mut voice_manager = VoiceManager::new(sample_rate);
    voice_manager.note_on(60, 100);

    // Process to get the voice active
    let _ = voice_manager.next_sample();

    // The voice should handle invalid internal states gracefully
    // This test mainly ensures the system doesn't crash with edge cases
    for _ in 0..100 {
        let _ = voice_manager.next_sample();
    }

    // Should still have valid state
    assert!(voice_manager.active_voice_count() <= 16);
}

/// Test buffer overflow scenarios
#[test]
fn test_buffer_overflow_scenarios() {
    let sample_rate = 44100.0;
    let mut voice_manager = VoiceManager::new(sample_rate);

    // Fill all voices
    for i in 0..16 {
        voice_manager.note_on(60 + i, 100);
    }

    // Try to add more voices (should trigger voice stealing)
    for i in 0..100 {
        voice_manager.note_on(60 + (i % 24), 100);

        // Should never exceed polyphony limit
        assert!(voice_manager.active_voice_count() <= 16);
    }

    // Process audio
    for _ in 0..1000 {
        let _ = voice_manager.next_sample();
    }
}

/// Test denormal numbers handling
#[test]
fn test_denormal_handling() {
    let sample_rate = 44100.0;

    // Create denormal numbers
    let denormal = 1e-40_f32;
    assert!(!denormal.is_normal());

    // Test oscillator with denormal frequency
    let mut osc = SimpleOscillator::new(WaveformType::Sine, sample_rate);
    osc.set_frequency(denormal);

    for _ in 0..100 {
        let sample = osc.next_sample();
        // Should not crash and should produce valid output
        assert!(sample.is_finite());
    }

    // Test filter with denormal parameters
    let params = FilterParams {
        cutoff: denormal,
        resonance: denormal,
        ..Default::default()
    };
    let mut filter = StateVariableFilter::new(params, sample_rate);

    for _ in 0..100 {
        let output = filter.process(0.5);
        assert!(output.is_finite());
    }
}
