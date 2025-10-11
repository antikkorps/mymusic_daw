// MIDI types events

#[derive(Debug, Clone, Copy)]
pub enum MidiEvent {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8 },
    ControlChange { controller: u8, value: u8 },
    PitchBend { value: i16 },
}

/// MIDI event with sample-accurate timing
/// `samples_from_now` represents when this event should be processed
/// relative to the current audio callback's first sample
#[derive(Debug, Clone, Copy)]
pub struct MidiEventTimed {
    pub event: MidiEvent,
    pub samples_from_now: u32,
}

impl MidiEvent {
    /// Parse un RAW MIDI message
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() {
            return None;
        }

        let status = bytes[0];
        let message_type = status & 0xF0;

        match message_type {
            0x90 => {
                // Note On
                if bytes.len() >= 3 {
                    let note = bytes[1];
                    let velocity = bytes[2];
                    // Velocity 0 = Note Off
                    if velocity == 0 {
                        Some(MidiEvent::NoteOff { note })
                    } else {
                        Some(MidiEvent::NoteOn { note, velocity })
                    }
                } else {
                    None
                }
            }
            0x80 => {
                // Note Off
                if bytes.len() >= 3 {
                    Some(MidiEvent::NoteOff { note: bytes[1] })
                } else {
                    None
                }
            }
            0xB0 => {
                // Control Change
                if bytes.len() >= 3 {
                    Some(MidiEvent::ControlChange {
                        controller: bytes[1],
                        value: bytes[2],
                    })
                } else {
                    None
                }
            }
            0xE0 => {
                // Pitch Bend
                if bytes.len() >= 3 {
                    let lsb = bytes[1] as i16;
                    let msb = bytes[2] as i16;
                    let value = (msb << 7) | lsb;
                    Some(MidiEvent::PitchBend { value })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_on() {
        let bytes = [0x90, 60, 100]; // Note On, note 60 (C4), velocity 100
        let event = MidiEvent::from_bytes(&bytes).unwrap();

        match event {
            MidiEvent::NoteOn { note, velocity } => {
                assert_eq!(note, 60);
                assert_eq!(velocity, 100);
            }
            _ => panic!("Expected NoteOn event"),
        }
    }

    #[test]
    fn test_note_off_explicit() {
        let bytes = [0x80, 60, 0]; // Note Off, note 60
        let event = MidiEvent::from_bytes(&bytes).unwrap();

        match event {
            MidiEvent::NoteOff { note } => {
                assert_eq!(note, 60);
            }
            _ => panic!("Expected NoteOff event"),
        }
    }

    #[test]
    fn test_note_off_velocity_zero() {
        // Note On avec velocity 0 = Note Off
        let bytes = [0x90, 64, 0];
        let event = MidiEvent::from_bytes(&bytes).unwrap();

        match event {
            MidiEvent::NoteOff { note } => {
                assert_eq!(note, 64);
            }
            _ => panic!("Expected NoteOff event (velocity 0)"),
        }
    }

    #[test]
    fn test_control_change() {
        let bytes = [0xB0, 7, 127]; // CC, controller 7 (volume), value 127
        let event = MidiEvent::from_bytes(&bytes).unwrap();

        match event {
            MidiEvent::ControlChange { controller, value } => {
                assert_eq!(controller, 7);
                assert_eq!(value, 127);
            }
            _ => panic!("Expected ControlChange event"),
        }
    }

    #[test]
    fn test_pitch_bend() {
        let bytes = [0xE0, 0x00, 0x40]; // Pitch Bend, valeur centrée
        let event = MidiEvent::from_bytes(&bytes).unwrap();

        match event {
            MidiEvent::PitchBend { value } => {
                // 0x40 << 7 | 0x00 = 8192 (centre)
                assert_eq!(value, 8192);
            }
            _ => panic!("Expected PitchBend event"),
        }
    }

    #[test]
    fn test_invalid_empty_message() {
        let bytes = [];
        let event = MidiEvent::from_bytes(&bytes);
        assert!(event.is_none());
    }

    #[test]
    fn test_invalid_incomplete_message() {
        let bytes = [0x90, 60]; // Note On sans velocity
        let event = MidiEvent::from_bytes(&bytes);
        assert!(event.is_none());
    }

    #[test]
    fn test_invalid_unknown_status() {
        let bytes = [0xF0, 0x00, 0x00]; // Status inconnu
        let event = MidiEvent::from_bytes(&bytes);
        assert!(event.is_none());
    }

    #[test]
    fn test_midi_channel_ignored() {
        // Le channel (4 bits de poids faible) doit être ignoré
        let bytes1 = [0x90, 60, 100]; // Channel 0
        let bytes2 = [0x9F, 60, 100]; // Channel 15

        let event1 = MidiEvent::from_bytes(&bytes1).unwrap();
        let event2 = MidiEvent::from_bytes(&bytes2).unwrap();

        // Les deux doivent être des NoteOn identiques
        match (event1, event2) {
            (MidiEvent::NoteOn { note: n1, velocity: v1 }, MidiEvent::NoteOn { note: n2, velocity: v2 }) => {
                assert_eq!(n1, n2);
                assert_eq!(v1, v2);
            }
            _ => panic!("Expected both to be NoteOn events"),
        }
    }

    #[test]
    fn test_valid_note_range() {
        // Tester différentes notes MIDI valides
        for note_num in [0, 60, 127] {
            let bytes = [0x90, note_num, 100];
            let event = MidiEvent::from_bytes(&bytes).unwrap();

            match event {
                MidiEvent::NoteOn { note, .. } => {
                    assert_eq!(note, note_num);
                }
                _ => panic!("Expected NoteOn"),
            }
        }
    }

    #[test]
    fn test_velocity_range() {
        // Tester différentes vélocités valides
        for vel in [1, 64, 127] {
            let bytes = [0x90, 60, vel];
            let event = MidiEvent::from_bytes(&bytes).unwrap();

            match event {
                MidiEvent::NoteOn { velocity, .. } => {
                    assert_eq!(velocity, vel);
                }
                _ => panic!("Expected NoteOn"),
            }
        }
    }
}
