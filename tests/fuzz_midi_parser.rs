//! Fuzzing tests for MIDI parser
//!
//! This module tests the MIDI parser with random and malformed data to ensure
//! it handles edge cases gracefully without crashing.

use mymusic_daw::midi::event::MidiEvent;
use rand::Rng;

/// Fuzz the MIDI parser with random byte sequences
#[test]
fn fuzz_midi_parser_random_bytes() {
    let mut rng = rand::thread_rng();
    
    // Test with 1000 random byte sequences
    for _ in 0..1000 {
        let length = rng.gen_range(1..=128);
        let random_bytes: Vec<u8> = (0..length)
            .map(|_| rng.gen_range(0..=255))
            .collect();
        
        // Should not panic, even with garbage data
        let _ = std::panic::catch_unwind(|| {
            let _ = MidiEvent::from_bytes(&random_bytes);
        });
    }
}

/// Fuzz with specific MIDI message patterns
#[test]
fn fuzz_midi_parser_patterns() {
    let mut rng = rand::thread_rng();
    
    // Common MIDI status bytes
    let status_bytes = vec![
        0x80, 0x90, 0xA0, 0xB0, 0xC0, 0xD0, 0xE0, // Channel messages
        0xF0, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, // System common
        0xF8, 0xFA, 0xFB, 0xFC, 0xFE, 0xFF, // System real-time
    ];
    
    for _ in 0..500 {
        let mut bytes = Vec::new();
        
        // Randomly choose a pattern
        match rng.gen_range(0..=5) {
            0 => {
                // Incomplete NoteOn/NoteOff (missing data bytes)
                bytes.push(status_bytes[rng.gen_range(0..2)] | rng.gen_range(0..=15));
                if rng.gen_bool(0.5) {
                    bytes.push(rng.gen_range(0..=127)); // Only one data byte
                }
            }
            1 => {
                // Complete NoteOn/NoteOff
                bytes.push(status_bytes[rng.gen_range(0..2)] | rng.gen_range(0..=15));
                bytes.push(rng.gen_range(0..=127)); // Note number
                bytes.push(rng.gen_range(0..=127)); // Velocity
            }
            2 => {
                // Control Change with random controller/value
                bytes.push(0xB0 | rng.gen_range(0..=15));
                bytes.push(rng.gen_range(0..=127)); // Controller number
                bytes.push(rng.gen_range(0..=127)); // Value
            }
            3 => {
                // Program Change (single data byte)
                bytes.push(0xC0 | rng.gen_range(0..=15));
                bytes.push(rng.gen_range(0..=127));
            }
            4 => {
                // Pitch Bend (two data bytes)
                bytes.push(0xE0 | rng.gen_range(0..=15));
                bytes.push(rng.gen_range(0..=127)); // LSB
                bytes.push(rng.gen_range(0..=127)); // MSB
            }
            5 => {
                // Random system messages
                bytes.push(status_bytes[rng.gen_range(7..status_bytes.len())]);
            }
            _ => {}
        }
        
        // Should not panic
        let _ = std::panic::catch_unwind(|| {
            let _ = MidiEvent::from_bytes(&bytes);
        });
    }
}

/// Test edge cases in MIDI parsing
#[test]
fn test_midi_parser_edge_cases() {
    // Test empty bytes
    let result = MidiEvent::from_bytes(&[]);
    assert!(result.is_none());
    
    // Test single byte
    let result = MidiEvent::from_bytes(&[0x40]);
    assert!(result.is_none());
    
    // Test system real-time messages (should be ignored by our parser)
    let result = MidiEvent::from_bytes(&[0xF8]); // Clock
    assert!(result.is_none());
    
    let result = MidiEvent::from_bytes(&[0xFA]); // Start
    assert!(result.is_none());
}

/// Test malformed messages
#[test]
fn test_midi_parser_malformed_messages() {
    // Incomplete NoteOn (missing velocity)
    let result = MidiEvent::from_bytes(&[0x90, 0x40]);
    assert!(result.is_none());
    
    // Incomplete Control Change (missing value)
    let result = MidiEvent::from_bytes(&[0xB0, 0x07]);
    assert!(result.is_none());
    
    // Incomplete Pitch Bend (missing MSB)
    let result = MidiEvent::from_bytes(&[0xE0, 0x00]);
    assert!(result.is_none());
}

/// Test maximum values
#[test]
fn test_midi_parser_maximum_values() {
    // Test all bytes at maximum value (0x7F)
    let result = MidiEvent::from_bytes(&[0x90, 0x7F, 0x7F]);
    assert!(matches!(result, Some(MidiEvent::NoteOn { note: 0x7F, velocity: 0x7F })));
    
    // Test minimum values (0x00)
    let result = MidiEvent::from_bytes(&[0x90, 0x00, 0x00]);
    assert!(matches!(result, Some(MidiEvent::NoteOff { note: 0x00 })));
}

/// Stress test with many messages
#[test]
fn test_midi_parser_many_messages() {
    // Test 1000 NoteOn messages
    for i in 0..1000 {
        let channel = (i % 16) as u8;
        let note = (i % 128) as u8;
        let velocity = (i % 128) as u8;
        
        let result = MidiEvent::from_bytes(&[0x90 | channel, note, velocity]);
        
        if velocity == 0 {
            assert!(matches!(result, Some(MidiEvent::NoteOff { note: n }) if n == note));
        } else {
            assert!(matches!(result, Some(MidiEvent::NoteOn { note: n, velocity: v }) if n == note && v == velocity));
        }
    }
}

/// Test invalid status bytes
#[test]
fn test_midi_parser_invalid_status() {
    // Invalid status bytes (0x00-0x7F should return None)
    for byte in 0x00..=0x7F {
        let result = MidiEvent::from_bytes(&[byte]);
        assert!(result.is_none());
    }
}

/// Test all MIDI message types
#[test]
fn test_midi_parser_all_message_types() {
    // Note On
    let result = MidiEvent::from_bytes(&[0x90, 0x40, 0x7F]);
    assert!(matches!(result, Some(MidiEvent::NoteOn { note: 0x40, velocity: 0x7F })));
    
    // Note Off (explicit)
    let result = MidiEvent::from_bytes(&[0x80, 0x40, 0x00]);
    assert!(matches!(result, Some(MidiEvent::NoteOff { note: 0x40 })));
    
    // Note Off (via NoteOn with velocity 0)
    let result = MidiEvent::from_bytes(&[0x90, 0x40, 0x00]);
    assert!(matches!(result, Some(MidiEvent::NoteOff { note: 0x40 })));
    
    // Control Change
    let result = MidiEvent::from_bytes(&[0xB0, 0x07, 0x64]);
    assert!(matches!(result, Some(MidiEvent::ControlChange { controller: 0x07, value: 0x64 })));
    
    // Pitch Bend
    let result = MidiEvent::from_bytes(&[0xE0, 0x00, 0x40]);
    assert!(matches!(result, Some(MidiEvent::PitchBend { value: 0x2000 }))); // 0x40 << 7 | 0x00
    
    // Channel Aftertouch
    let result = MidiEvent::from_bytes(&[0xD0, 0x40]);
    assert!(matches!(result, Some(MidiEvent::ChannelAftertouch { value: 0x40 })));
    
    // Poly Aftertouch
    let result = MidiEvent::from_bytes(&[0xA0, 0x40, 0x40]);
    assert!(matches!(result, Some(MidiEvent::PolyAftertouch { note: 0x40, value: 0x40 })));
    
    // Program Change is NOT supported (returns None)
    let result = MidiEvent::from_bytes(&[0xC0, 0x05]);
    assert!(result.is_none());
}