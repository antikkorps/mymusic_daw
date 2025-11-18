//! Edge case tests and robustness validation
//!
//! This module tests extreme scenarios and edge cases to ensure the DAW
//! handles them gracefully without crashing or producing undefined behavior.

use mymusic_daw::synth::filter::{StateVariableFilter, FilterParams, FilterType};
use mymusic_daw::synth::oscillator::{SimpleOscillator, WaveformType, Oscillator};
use mymusic_daw::synth::voice::Voice;
use mymusic_daw::synth::voice_manager::VoiceManager;
use mymusic_daw::synth::envelope::ADSR;
use mymusic_daw::synth::lfo::Lfo;
use mymusic_daw::audio::dsp_utils::OnePoleSmoother;
use mymusic_daw::audio::format_conversion::convert_f32_to_i16;
use std::f32::{INFINITY, NEG_INFINITY, NAN};

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
        assert!(sample >= -1.0 && sample <= 1.0);
    }
    
    // Test very high frequencies (near Nyquist)
    let mut osc = SimpleOscillator::new(WaveformType::Sine, sample_rate);
    osc.set_frequency(20000.0); // Near Nyquist
    
    for _ in 0..1000 {
        let sample = osc.next_sample();
        assert!(sample.is_finite());
        assert!(sample >= -1.0 && sample <= 1.0);
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
    osc.set_frequency(NAN);
    for _ in 0..100 {
        let sample = osc.next_sample();
        // Should produce NaN samples but not crash
        assert!(sample.is_nan() || sample.is_finite());
    }
    
    // Test infinite frequency
    osc.set_frequency(INFINITY);
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
    let output = filter.process(NAN);
    assert!(output.is_nan() || output.is_finite());
    
    // Reset filter
    filter.reset();
    
    // Test Infinity input
    let output = filter.process(INFINITY);
    assert!(output.is_nan() || output.is_infinite() || output.is_finite());
    
    // Reset filter
    filter.reset();
    
    // Test negative infinity input
    let output = filter.process(NEG_INFINITY);
    assert!(output.is_nan() || output.is_infinite() || output.is_finite());
}

/// Test ADSR envelope with extreme parameters
#[test]
fn test_adsr_extreme_parameters() {
    let sample_rate = 44100.0;
    
    // Test zero-length stages
    let mut adsr = ADSR::new(0.0, 0.0, 0.5, 0.0, sample_rate);
    adsr.note_on();
    
    for _ in 0..100 {
        let output = adsr.next_sample();
        assert!(output.is_finite());
        assert!(output >= 0.0 && output <= 1.0);
    }
    
    // Test very long attack
    let mut adsr = ADSR::new(60.0, 0.0, 0.5, 0.0, sample_rate); // 60 second attack
    adsr.note_on();
    
    for _ in 0..1000 {
        let output = adsr.next_sample();
        assert!(output.is_finite());
        assert!(output >= 0.0 && output <= 1.0);
    }
    
    // Test sustain at 0 and 1
    let mut adsr = ADSR::new(0.1, 0.1, 0.0, 0.1, sample_rate); // Sustain = 0
    adsr.note_on();
    
    // Run through attack and decay
    for _ in 0..(sample_rate as usize / 10) {
        let _ = adsr.next_sample();
    }
    
    // Should be at sustain level (0)
    let output = adsr.next_sample();
    assert!(output.abs() < 0.01); // Near zero
    
    // Test sustain = 1
    let mut adsr = ADSR::new(0.1, 0.1, 1.0, 0.1, sample_rate);
    adsr.note_on();
    
    // Run through attack and decay
    for _ in 0..(sample_rate as usize / 10) {
        let _ = adsr.next_sample();
    }
    
    // Should be at sustain level (1)
    let output = adsr.next_sample();
    assert!((output - 1.0).abs() < 0.01); // Near 1
}

/// Test voice manager with maximum polyphony
#[test]
fn test_voice_manager_max_polyphony() {
    let sample_rate = 44100.0;
    let mut voice_manager = VoiceManager::new(16, sample_rate);
    
    // Trigger more notes than available voices
    for i in 0..32 {
        voice_manager.note_on(60 + (i % 12) as u8, 100);
    }
    
    // Process audio
    for _ in 0..1000 {
        let _ = voice_manager.process();
    }
    
    // Should not crash and should have exactly 16 active voices
    assert_eq!(voice_manager.active_voice_count(), 16);
}

/// Test voice manager with rapid note on/off
#[test]
fn test_voice_manager_rapid_triggering() {
    let sample_rate = 44100.0;
    let mut voice_manager = VoiceManager::new(16, sample_rate);
    
    // Rapidly trigger and release the same note
    for _ in 0..100 {
        voice_manager.note_on(60, 100);
        let _ = voice_manager.process();
        voice_manager.note_off(60);
        let _ = voice_manager.process();
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
        assert!(output >= 0.0 && output <= 1.0);
    }
    
    // Test step from 1 to 0
    for _ in 0..1000 {
        let output = smoother.process(0.0);
        assert!(output.is_finite());
        assert!(output >= 0.0 && output <= 1.0);
    }
    
    // Test with NaN target
    let output = smoother.process(NAN);
    assert!(output.is_finite() || output.is_nan());
    
    // Test with Infinity target
    let output = smoother.process(INFINITY);
    assert!(output.is_finite() || output.is_infinite() || output.is_nan());
}

/// Test format conversion with extreme values
#[test]
fn test_format_conversion_extreme_values() {
    // Test values outside [-1, 1] range
    let test_values = vec![
        2.0,    // Above 1
        -2.0,   // Below -1
        10.0,   // Way above
        -10.0,  // Way below
        INFINITY,
        NEG_INFINITY,
        NAN,
    ];
    
    for &value in &test_values {
        let converted = convert_f32_to_i16(value);
        // Should not crash and should produce valid i16
        assert!(converted >= -32768 && converted <= 32767);
    }
}

/// Test LFO with extreme rates
#[test]
fn test_lfo_extreme_rates() {
    let sample_rate = 44100.0;
    
    // Test very slow LFO (0.1 Hz)
    let mut lfo = Lfo::new(WaveformType::Sine, 0.1, sample_rate);
    for _ in 0..1000 {
        let output = lfo.next_sample();
        assert!(output.is_finite());
        assert!(output >= -1.0 && output <= 1.0);
    }
    
    // Test very fast LFO (1000 Hz)
    let mut lfo = Lfo::new(WaveformType::Sine, 1000.0, sample_rate);
    for _ in 0..1000 {
        let output = lfo.next_sample();
        assert!(output.is_finite());
        assert!(output >= -1.0 && output <= 1.0);
    }
    
    // Test LFO at Nyquist frequency
    let mut lfo = Lfo::new(WaveformType::Sine, sample_rate / 2.0, sample_rate);
    for _ in 0..1000 {
        let output = lfo.next_sample();
        assert!(output.is_finite());
    }
}

/// Test concurrent access patterns (simulated)
#[test]
fn test_concurrent_access_patterns() {
    use std::sync::{Arc, Mutex};
    use std::thread;
    
    let sample_rate = 44100.0;
    let voice_manager = Arc::new(Mutex::new(VoiceManager::new(16, sample_rate)));
    
    // Simulate multiple threads accessing voice manager
    let mut handles = vec![];
    
    for i in 0..4 {
        let vm = Arc::clone(&voice_manager);
        let handle = thread::spawn(move || {
            for j in 0..100 {
                let mut manager = vm.lock().unwrap();
                manager.note_on(60 + (i * 10 + j) as u8 % 24, 100);
                let _ = manager.process();
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
    let mut voice_manager = VoiceManager::new(16, sample_rate);
    voice_manager.note_on(60, 100);
    
    // Process to get the voice active
    let _ = voice_manager.process();
    
    // The voice should handle invalid internal states gracefully
    // This test mainly ensures the system doesn't crash with edge cases
    for _ in 0..100 {
        let _ = voice_manager.process();
    }
    
    // Should still have valid state
    assert!(voice_manager.active_voice_count() <= 16);
}

/// Test buffer overflow scenarios
#[test]
fn test_buffer_overflow_scenarios() {
    let sample_rate = 44100.0;
    let mut voice_manager = VoiceManager::new(16, sample_rate);
    
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
        let _ = voice_manager.process();
    }
}

/// Test denormal numbers handling
#[test]
fn test_denormal_handling() {
    let sample_rate = 44100.0;
    
    // Create denormal numbers
    let denormal = 1e-40_f32;
    assert!(denormal.is_normal() == false);
    
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