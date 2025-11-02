// Sequencer Player - Reads patterns and triggers notes
// Phase 4: Audio playback for sequencer

use crate::midi::event::{MidiEvent, MidiEventTimed};
use crate::sequencer::{NoteId, Pattern, Tempo, TimeSignature};
use std::collections::HashMap;

/// Tracks active notes (NoteOn sent, waiting for NoteOff)
#[derive(Debug, Clone)]
struct ActiveNote {
    _note_id: NoteId,
    midi_pitch: u8,
    end_sample: u64,
}

/// Sequencer player - converts pattern notes to MIDI events
pub struct SequencerPlayer {
    /// Currently active notes (waiting for NoteOff)
    active_notes: HashMap<NoteId, ActiveNote>,

    /// Sample rate for timing calculations
    sample_rate: f64,

    /// Last processed position (to detect new notes)
    last_position_samples: u64,
}

impl SequencerPlayer {
    /// Create a new sequencer player
    pub fn new(sample_rate: f64) -> Self {
        Self {
            active_notes: HashMap::new(),
            sample_rate,
            last_position_samples: 0,
        }
    }

    /// Process a buffer and generate MIDI events for notes in the pattern
    ///
    /// Returns a vector of MIDI events to be sent to the audio engine
    pub fn process(
        &mut self,
        pattern: &Pattern,
        current_position: u64,
        is_playing: bool,
        tempo: &Tempo,
        time_signature: &TimeSignature,
        buffer_size: usize,
    ) -> Vec<MidiEventTimed> {
        let mut events = Vec::new();

        // If not playing, stop all active notes and return
        if !is_playing {
            // Send NoteOff for all active notes
            for (_, active_note) in self.active_notes.drain() {
                events.push(MidiEventTimed {
                    event: MidiEvent::NoteOff {
                        note: active_note.midi_pitch,
                    },
                    samples_from_now: 0,
                });
            }
            self.last_position_samples = current_position;
            return events;
        }

        // Handle loop wrapping
        let pattern_length_samples =
            pattern.length_samples(self.sample_rate, tempo, time_signature);

        // If pattern is empty or very short, bail out
        if pattern_length_samples == 0 || pattern.is_empty() {
            return events;
        }

        // Normalize positions within pattern length to avoid overflow
        let current_position_normalized = current_position % pattern_length_samples;

        // Check for notes that should start in this buffer
        for note in pattern.notes() {
            let note_start = note.start.samples % pattern_length_samples;

            // Check if this note should start in the current buffer
            let should_trigger = self.should_trigger_note(
                note_start,
                current_position_normalized,
                current_position_normalized + buffer_size as u64,
                pattern_length_samples,
            );

            if should_trigger && !self.active_notes.contains_key(&note.id) {
                // Calculate sample offset within buffer
                let sample_offset = if note_start >= current_position_normalized {
                    note_start - current_position_normalized
                } else {
                    // Loop wrap case
                    pattern_length_samples - current_position_normalized + note_start
                };

                // Send NoteOn
                events.push(MidiEventTimed {
                    event: MidiEvent::NoteOn {
                        note: note.pitch,
                        velocity: note.velocity,
                    },
                    samples_from_now: sample_offset.min(buffer_size as u64) as u32,
                });

                // Track this note as active
                self.active_notes.insert(
                    note.id,
                    ActiveNote {
                        _note_id: note.id,
                        midi_pitch: note.pitch,
                        end_sample: note.start.samples + note.duration_samples,
                    },
                );
            }
        }

        // Check for notes that should end in this buffer
        let mut notes_to_stop = Vec::new();

        for (note_id, active_note) in &self.active_notes {
            let note_end = active_note.end_sample % pattern_length_samples;

            let should_stop = self.should_trigger_note(
                note_end,
                current_position_normalized,
                current_position_normalized + buffer_size as u64,
                pattern_length_samples,
            );

            if should_stop {
                // Calculate sample offset within buffer
                let sample_offset = if note_end >= current_position_normalized {
                    note_end - current_position_normalized
                } else {
                    // Loop wrap case
                    pattern_length_samples - current_position_normalized + note_end
                };

                // Send NoteOff
                events.push(MidiEventTimed {
                    event: MidiEvent::NoteOff {
                        note: active_note.midi_pitch,
                    },
                    samples_from_now: sample_offset.min(buffer_size as u64) as u32,
                });

                notes_to_stop.push(*note_id);
            }
        }

        // Remove stopped notes
        for note_id in notes_to_stop {
            self.active_notes.remove(&note_id);
        }

        self.last_position_samples = current_position;

        events
    }

    /// Check if a note event (start or end) should trigger in the current buffer
    fn should_trigger_note(
        &self,
        event_sample: u64,
        buffer_start: u64,
        buffer_end: u64,
        pattern_length: u64,
    ) -> bool {
        // Normalize positions within pattern length
        let event_pos = event_sample % pattern_length;
        let start_pos = buffer_start % pattern_length;
        let end_pos = buffer_end % pattern_length;

        // Handle normal case (no loop wrap within buffer)
        if start_pos < end_pos {
            event_pos >= start_pos && event_pos < end_pos
        } else {
            // Loop wrap within buffer
            event_pos >= start_pos || event_pos < end_pos
        }
    }

    /// Stop all currently playing notes (called when transport stops)
    pub fn stop_all_notes(&mut self) -> Vec<MidiEventTimed> {
        let mut events = Vec::new();

        for (_, active_note) in self.active_notes.drain() {
            events.push(MidiEventTimed {
                event: MidiEvent::NoteOff {
                    note: active_note.midi_pitch,
                },
                samples_from_now: 0,
            });
        }

        events
    }

    /// Reset player state (called when transport position changes)
    pub fn reset(&mut self) {
        self.active_notes.clear();
        self.last_position_samples = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequencer::{Note, Position, Tempo, TimeSignature};

    #[test]
    fn test_sequencer_player_creation() {
        let player = SequencerPlayer::new(48000.0);
        assert_eq!(player.active_notes.len(), 0);
    }

    #[test]
    fn test_note_triggering() {
        let mut player = SequencerPlayer::new(48000.0);
        let mut pattern = Pattern::new_default(1, "Test".to_string());

        // Add a note at the start
        let note = Note::new(
            1,
            60,
            Position::zero(),
            24000, // 0.5s duration at 48kHz
            100,
        );
        pattern.add_note(note);

        let tempo = Tempo::new(120.0);
        let time_signature = TimeSignature::four_four();

        // Process first buffer (should trigger NoteOn)
        let events = player.process(&pattern, 0, true, &tempo, &time_signature, 512);

        // Should have one NoteOn event
        assert_eq!(events.len(), 1);
        match events[0].event {
            MidiEvent::NoteOn { note, velocity } => {
                assert_eq!(note, 60);
                assert_eq!(velocity, 100);
            }
            _ => panic!("Expected NoteOn"),
        }
    }

    #[test]
    fn test_stop_all_notes() {
        let mut player = SequencerPlayer::new(48000.0);

        // Manually add an active note
        player.active_notes.insert(
            1,
            ActiveNote {
                _note_id: 1,
                midi_pitch: 60,
                end_sample: 10000,
            },
        );

        let events = player.stop_all_notes();

        assert_eq!(events.len(), 1);
        match events[0].event {
            MidiEvent::NoteOff { note } => assert_eq!(note, 60),
            _ => panic!("Expected NoteOff"),
        }

        assert_eq!(player.active_notes.len(), 0);
    }
}
