// Note representation for the sequencer
// A note is a MIDI event with position, pitch, duration, and velocity

use crate::sequencer::timeline::{MusicalTime, Position};

/// Unique identifier for notes
pub type NoteId = u64;

/// A musical note in the sequencer
///
/// Notes are stored with both sample-accurate and musical time representations.
/// Duration is stored in samples for audio callback efficiency.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Note {
    /// Unique identifier for this note
    pub id: NoteId,

    /// MIDI note number (0-127, where 60 = C4)
    pub pitch: u8,

    /// Start position (samples + musical time)
    pub start: Position,

    /// Duration in samples
    /// We store in samples for efficiency in audio callback
    pub duration_samples: u64,

    /// MIDI velocity (0-127, where 127 = maximum)
    pub velocity: u8,
}

impl Note {
    /// Creates a new note
    pub fn new(
        id: NoteId,
        pitch: u8,
        start: Position,
        duration_samples: u64,
        velocity: u8,
    ) -> Self {
        assert!(pitch <= 127, "MIDI pitch must be 0-127");
        assert!(velocity <= 127, "MIDI velocity must be 0-127");
        assert!(duration_samples > 0, "Note duration must be > 0");

        Self {
            id,
            pitch,
            start,
            duration_samples,
            velocity,
        }
    }

    /// Get the end position of this note (in samples)
    pub fn end_sample(&self) -> u64 {
        self.start.samples + self.duration_samples
    }

    /// Check if this note contains a given sample position
    pub fn contains_sample(&self, sample: u64) -> bool {
        sample >= self.start.samples && sample < self.end_sample()
    }

    /// Get duration in musical time (ticks)
    pub fn duration_ticks(
        &self,
        sample_rate: f64,
        tempo: &crate::sequencer::timeline::Tempo,
    ) -> u64 {
        // Convert duration samples to seconds
        let duration_seconds = self.duration_samples as f64 / sample_rate;

        // Convert seconds to beats
        let beats = duration_seconds / tempo.beat_duration_seconds();

        // Convert beats to ticks
        (beats * MusicalTime::TICKS_PER_QUARTER as f64) as u64
    }

    /// Get the note name (e.g., "C4", "A#5")
    pub fn note_name(&self) -> String {
        const NOTE_NAMES: [&str; 12] = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];

        let octave = (self.pitch / 12) as i32 - 1;
        let note_index = (self.pitch % 12) as usize;

        format!("{}{}", NOTE_NAMES[note_index], octave)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequencer::timeline::{Position, Tempo};

    #[test]
    fn test_note_creation() {
        let pos = Position::zero();
        let note = Note::new(1, 60, pos, 48000, 100);

        assert_eq!(note.id, 1);
        assert_eq!(note.pitch, 60);
        assert_eq!(note.velocity, 100);
        assert_eq!(note.duration_samples, 48000);
    }

    #[test]
    fn test_note_end_position() {
        let pos = Position::zero();
        let note = Note::new(1, 60, pos, 24000, 100);

        assert_eq!(note.end_sample(), 24000);
    }

    #[test]
    fn test_note_contains_sample() {
        let pos = Position::zero();
        let note = Note::new(1, 60, pos, 24000, 100);

        assert!(note.contains_sample(0));
        assert!(note.contains_sample(12000));
        assert!(note.contains_sample(23999));
        assert!(!note.contains_sample(24000));
        assert!(!note.contains_sample(30000));
    }

    #[test]
    fn test_note_name() {
        let pos = Position::zero();

        // Middle C (C4) = MIDI note 60
        let note_c4 = Note::new(1, 60, pos, 1000, 100);
        assert_eq!(note_c4.note_name(), "C4");

        // A4 (440 Hz) = MIDI note 69
        let note_a4 = Note::new(2, 69, pos, 1000, 100);
        assert_eq!(note_a4.note_name(), "A4");

        // C#5 = MIDI note 73
        let note_cs5 = Note::new(3, 73, pos, 1000, 100);
        assert_eq!(note_cs5.note_name(), "C#5");
    }

    #[test]
    fn test_note_duration_conversion() {
        let sample_rate = 48000.0;
        let tempo = Tempo::new(120.0);
        let pos = Position::zero();

        // At 120 BPM, one beat = 0.5s = 24000 samples
        // So 24000 samples = 480 ticks (one quarter note)
        let note = Note::new(1, 60, pos, 24000, 100);
        let duration_ticks = note.duration_ticks(sample_rate, &tempo);

        assert_eq!(duration_ticks, 480);
    }

    #[test]
    #[should_panic(expected = "MIDI pitch must be 0-127")]
    fn test_invalid_pitch() {
        let pos = Position::zero();
        Note::new(1, 128, pos, 1000, 100);
    }

    #[test]
    #[should_panic(expected = "MIDI velocity must be 0-127")]
    fn test_invalid_velocity() {
        let pos = Position::zero();
        Note::new(1, 60, pos, 1000, 128);
    }

    #[test]
    #[should_panic(expected = "Note duration must be > 0")]
    fn test_zero_duration() {
        let pos = Position::zero();
        Note::new(1, 60, pos, 0, 100);
    }
}
