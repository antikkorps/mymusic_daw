// MIDI Recorder - Minimal implementation for Phase 4
// Captures NoteOn/NoteOff events and converts them to notes

use crate::midi::event::MidiEvent;
use crate::sequencer::note::Note;
use crate::sequencer::pattern::generate_note_id;
use crate::sequencer::timeline::{Position, Tempo, TimeSignature};
use std::collections::HashMap;

/// MIDI recorder with proper timing context
pub struct MidiRecorder {
    active_notes: HashMap<u8, (u8, u64)>, // note -> (velocity, start_sample)
    recorded_notes: Vec<Note>,
    #[allow(dead_code)]
    recording_start_sample: u64,

    // Timing context for proper Position conversion
    sample_rate: f64,
    tempo: Tempo,
    time_signature: TimeSignature,
}

impl MidiRecorder {
    pub fn new(
    #[allow(dead_code)]
    recording_start_sample: u64,
        sample_rate: f64,
        tempo: Tempo,
        time_signature: TimeSignature,
    ) -> Self {
        Self {
            active_notes: HashMap::new(),
            recorded_notes: Vec::new(),
            recording_start_sample,
            sample_rate,
            tempo,
            time_signature,
        }
    }

    pub fn process_event(&mut self, event: MidiEvent, current_sample: u64) {
        match event {
            MidiEvent::NoteOn { note, velocity } => {
                if velocity > 0 {
                    self.active_notes.insert(note, (velocity, current_sample));
                }
            }
            MidiEvent::NoteOff { note } => {
                if let Some((velocity, start_sample)) = self.active_notes.remove(&note) {
                    let duration = (current_sample - start_sample).max(1);
                    let note = Note::new(
                        generate_note_id(),
                        note,
                        self.sample_to_position(start_sample),
                        duration,
                        velocity,
                    );
                    self.recorded_notes.push(note);
                }
            }
            _ => {}
        }
    }

    /// Convert sample position to musical time with proper context
    fn sample_to_position(&self, sample: u64) -> Position {
        Position::from_samples(sample, self.sample_rate, &self.tempo, &self.time_signature)
    }

    pub fn get_recorded_notes(&self) -> Vec<Note> {
        self.recorded_notes.clone()
    }

    /// Finalize recording by closing all active notes
    /// Returns the notes that were active at recording stop
    pub fn finalize_recording(&mut self) -> Vec<Note> {
        // Extract active notes to avoid borrow conflicts
        let active_notes = std::mem::take(&mut self.active_notes);

        let mut final_notes = Vec::new();

        // Close all currently active notes with a default duration
        let default_duration = 1000u64; // 1000 samples = ~21ms at 48kHz
        for (note, (velocity, start_sample)) in active_notes {
            let recorded_note = Note::new(
                generate_note_id(),
                note,
                self.sample_to_position(start_sample),
                default_duration,
                velocity,
            );
            final_notes.push(recorded_note);
        }

        // Return final notes plus already recorded notes
        let mut all_notes = std::mem::take(&mut self.recorded_notes);
        all_notes.extend(final_notes);
        all_notes
    }

    pub fn clear(&mut self) {
        self.active_notes.clear();
        self.recorded_notes.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequencer::timeline::{Tempo, TimeSignature};

    #[test]
    fn test_basic_recording() {
        let tempo = Tempo::new(120.0);
        let time_signature = TimeSignature::four_four();
        let mut recorder = MidiRecorder::new(1000, 48000.0, tempo, time_signature);
        recorder.process_event(
            MidiEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
            2000,
        );
        recorder.process_event(MidiEvent::NoteOff { note: 60 }, 3000);

        let notes = recorder.get_recorded_notes();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].pitch, 60);
        assert_eq!(notes[0].velocity, 100);
    }

    #[test]
    fn test_active_notes_closure() {
        let tempo = Tempo::new(120.0);
        let time_signature = TimeSignature::four_four();
        let mut recorder = MidiRecorder::new(1000, 48000.0, tempo, time_signature);

        // Start note but don't end it
        recorder.process_event(
            MidiEvent::NoteOn {
                note: 60,
                velocity: 100,
            },
            2000,
        );
        assert_eq!(recorder.active_notes.len(), 1);

        // Finalize without NoteOff
        let final_notes = recorder.finalize_recording();
        assert_eq!(final_notes.len(), 1);
        assert_eq!(recorder.active_notes.len(), 0); // Active notes should be cleared

        // Next recording should start fresh
        recorder.process_event(
            MidiEvent::NoteOn {
                note: 65,
                velocity: 80,
            },
            4000,
        );
        assert_eq!(recorder.active_notes.len(), 1);
    }
}
