// MIDI types events

#[derive(Debug, Clone, Copy)]
pub enum MidiEvent {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8 },
    ControlChange { controller: u8, value: u8 },
    PitchBend { value: i16 },
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
