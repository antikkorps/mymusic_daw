// Pattern - Collection of MIDI notes forming a sequence
// A pattern is like a "clip" in other DAWs

use crate::sequencer::note::{Note, NoteId};
use crate::sequencer::timeline::{Position, Tempo, TimeSignature};
use std::sync::atomic::{AtomicU64, Ordering};

/// Unique identifier for patterns
pub type PatternId = u64;

/// Global note ID generator (atomic for thread-safety)
static NEXT_NOTE_ID: AtomicU64 = AtomicU64::new(1);

/// Generate a unique note ID
pub fn generate_note_id() -> NoteId {
    NEXT_NOTE_ID.fetch_add(1, Ordering::Relaxed)
}

/// A pattern containing MIDI notes
///
/// A pattern is a reusable sequence of notes that can be placed on the timeline.
/// For Phase 4 MVP, we'll have a single active pattern.
#[derive(Debug, Clone)]
pub struct Pattern {
    /// Unique identifier
    pub id: PatternId,

    /// Pattern name
    pub name: String,

    /// All notes in this pattern
    notes: Vec<Note>,

    /// Length of the pattern in bars
    /// Determines when the pattern loops
    pub length_bars: u32,
}

impl Pattern {
    /// Create a new empty pattern
    pub fn new(id: PatternId, name: String, length_bars: u32) -> Self {
        assert!(length_bars > 0, "Pattern length must be at least 1 bar");

        Self {
            id,
            name,
            notes: Vec::new(),
            length_bars,
        }
    }

    /// Create a new pattern with default length (4 bars)
    pub fn new_default(id: PatternId, name: String) -> Self {
        Self::new(id, name, 4)
    }

    /// Get all notes
    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    /// Add a note to the pattern
    pub fn add_note(&mut self, note: Note) {
        // Keep notes sorted by start position for efficient playback
        let insert_pos = self
            .notes
            .binary_search_by(|n| n.start.samples.cmp(&note.start.samples))
            .unwrap_or_else(|pos| pos);

        self.notes.insert(insert_pos, note);
    }

    /// Remove a note by ID
    pub fn remove_note(&mut self, note_id: NoteId) -> Option<Note> {
        if let Some(index) = self.notes.iter().position(|n| n.id == note_id) {
            Some(self.notes.remove(index))
        } else {
            None
        }
    }

    /// Get a note by ID
    pub fn get_note(&self, note_id: NoteId) -> Option<&Note> {
        self.notes.iter().find(|n| n.id == note_id)
    }

    /// Get a mutable note by ID
    pub fn get_note_mut(&mut self, note_id: NoteId) -> Option<&mut Note> {
        self.notes.iter_mut().find(|n| n.id == note_id)
    }

    /// Find notes at a given sample position
    pub fn notes_at_sample(&self, sample: u64) -> Vec<&Note> {
        self.notes
            .iter()
            .filter(|n| n.contains_sample(sample))
            .collect()
    }

    /// Find notes in a sample range
    pub fn notes_in_range(&self, start_sample: u64, end_sample: u64) -> Vec<&Note> {
        self.notes
            .iter()
            .filter(|n| {
                // Note overlaps with range if:
                // - Note starts before range ends AND
                // - Note ends after range starts
                n.start.samples < end_sample && n.end_sample() > start_sample
            })
            .collect()
    }

    /// Get the length of the pattern in samples
    pub fn length_samples(
        &self,
        sample_rate: f64,
        tempo: &Tempo,
        time_signature: &TimeSignature,
    ) -> u64 {
        let bar_duration = tempo.bar_duration_samples(sample_rate, time_signature);
        (bar_duration * self.length_bars as f64) as u64
    }

    /// Clear all notes
    pub fn clear(&mut self) {
        self.notes.clear();
    }

    /// Get the number of notes
    pub fn note_count(&self) -> usize {
        self.notes.len()
    }

    /// Check if pattern is empty
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    /// Quantize all notes to a given subdivision
    ///
    /// # Arguments
    /// * `subdivision` - Number of subdivisions per quarter note (e.g., 4 = sixteenth notes)
    /// * `sample_rate` - Audio sample rate
    /// * `tempo` - Current tempo
    /// * `time_signature` - Current time signature
    pub fn quantize_all(
        &mut self,
        subdivision: u16,
        sample_rate: f64,
        tempo: &Tempo,
        time_signature: &TimeSignature,
    ) {
        for note in self.notes.iter_mut() {
            // Quantize start position
            let quantized_musical = note
                .start
                .musical
                .quantize_to_subdivision(time_signature, subdivision);

            // Create new position from quantized musical time
            note.start =
                Position::from_musical(quantized_musical, sample_rate, tempo, time_signature);
        }

        // Re-sort after quantization
        self.notes
            .sort_by(|a, b| a.start.samples.cmp(&b.start.samples));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequencer::timeline::{MusicalTime, Position, Tempo, TimeSignature};

    #[test]
    fn test_pattern_creation() {
        let pattern = Pattern::new(1, "Test Pattern".to_string(), 4);

        assert_eq!(pattern.id, 1);
        assert_eq!(pattern.name, "Test Pattern");
        assert_eq!(pattern.length_bars, 4);
        assert!(pattern.is_empty());
    }

    #[test]
    fn test_add_note() {
        let mut pattern = Pattern::new_default(1, "Test".to_string());
        let pos = Position::zero();
        let note = Note::new(generate_note_id(), 60, pos, 24000, 100);

        pattern.add_note(note);

        assert_eq!(pattern.note_count(), 1);
        assert!(!pattern.is_empty());
    }

    #[test]
    fn test_remove_note() {
        let mut pattern = Pattern::new_default(1, "Test".to_string());
        let pos = Position::zero();
        let note_id = generate_note_id();
        let note = Note::new(note_id, 60, pos, 24000, 100);

        pattern.add_note(note);
        assert_eq!(pattern.note_count(), 1);

        let removed = pattern.remove_note(note_id);
        assert!(removed.is_some());
        assert_eq!(pattern.note_count(), 0);
    }

    #[test]
    fn test_notes_sorted_by_position() {
        let mut pattern = Pattern::new_default(1, "Test".to_string());

        // Add notes out of order
        let note1 = Note::new(generate_note_id(), 60, Position::zero(), 1000, 100);
        let note2 = Note::new(
            generate_note_id(),
            64,
            Position::new(48000, MusicalTime::zero()),
            1000,
            100,
        );
        let note3 = Note::new(
            generate_note_id(),
            67,
            Position::new(24000, MusicalTime::zero()),
            1000,
            100,
        );

        // Add in reverse order
        pattern.add_note(note2);
        pattern.add_note(note3);
        pattern.add_note(note1);

        // Should be sorted by start position
        let notes = pattern.notes();
        assert_eq!(notes[0].start.samples, 0);
        assert_eq!(notes[1].start.samples, 24000);
        assert_eq!(notes[2].start.samples, 48000);
    }

    #[test]
    fn test_notes_at_sample() {
        let mut pattern = Pattern::new_default(1, "Test".to_string());

        let note1 = Note::new(generate_note_id(), 60, Position::zero(), 24000, 100);
        let note2 = Note::new(
            generate_note_id(),
            64,
            Position::new(12000, MusicalTime::zero()),
            24000,
            100,
        );

        pattern.add_note(note1);
        pattern.add_note(note2);

        // At sample 0: only note1
        let notes_at_0 = pattern.notes_at_sample(0);
        assert_eq!(notes_at_0.len(), 1);
        assert_eq!(notes_at_0[0].pitch, 60);

        // At sample 15000: both notes
        let notes_at_15000 = pattern.notes_at_sample(15000);
        assert_eq!(notes_at_15000.len(), 2);

        // At sample 50000: no notes
        let notes_at_50000 = pattern.notes_at_sample(50000);
        assert_eq!(notes_at_50000.len(), 0);
    }

    #[test]
    fn test_notes_in_range() {
        let mut pattern = Pattern::new_default(1, "Test".to_string());

        let note1 = Note::new(generate_note_id(), 60, Position::zero(), 10000, 100);
        let note2 = Note::new(
            generate_note_id(),
            64,
            Position::new(20000, MusicalTime::zero()),
            10000,
            100,
        );
        let note3 = Note::new(
            generate_note_id(),
            67,
            Position::new(40000, MusicalTime::zero()),
            10000,
            100,
        );

        pattern.add_note(note1);
        pattern.add_note(note2);
        pattern.add_note(note3);

        // Range 0-15000: should find note1
        let notes = pattern.notes_in_range(0, 15000);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].pitch, 60);

        // Range 15000-35000: should find note2
        let notes = pattern.notes_in_range(15000, 35000);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].pitch, 64);

        // Range 0-50000: should find all notes
        let notes = pattern.notes_in_range(0, 50000);
        assert_eq!(notes.len(), 3);
    }

    #[test]
    fn test_pattern_length() {
        let pattern = Pattern::new(1, "Test".to_string(), 4);
        let sample_rate = 48000.0;
        let tempo = Tempo::new(120.0);
        let time_signature = TimeSignature::four_four();

        // At 120 BPM, 4/4 time:
        // 1 beat = 0.5s = 24000 samples
        // 1 bar = 4 beats = 96000 samples
        // 4 bars = 384000 samples
        let length = pattern.length_samples(sample_rate, &tempo, &time_signature);
        assert_eq!(length, 384000);
    }

    #[test]
    fn test_clear_pattern() {
        let mut pattern = Pattern::new_default(1, "Test".to_string());

        pattern.add_note(Note::new(
            generate_note_id(),
            60,
            Position::zero(),
            1000,
            100,
        ));
        pattern.add_note(Note::new(
            generate_note_id(),
            64,
            Position::zero(),
            1000,
            100,
        ));

        assert_eq!(pattern.note_count(), 2);

        pattern.clear();
        assert!(pattern.is_empty());
    }

    #[test]
    fn test_quantize_all() {
        let mut pattern = Pattern::new_default(1, "Test".to_string());
        let sample_rate = 48000.0;
        let tempo = Tempo::new(120.0);
        let time_signature = TimeSignature::four_four();

        // Add a note slightly off-grid (100 samples after beat 1)
        let off_grid_pos = Position::new(100, MusicalTime::new(1, 1, 0));
        let note = Note::new(generate_note_id(), 60, off_grid_pos, 1000, 100);
        pattern.add_note(note);

        // Quantize to sixteenth notes (subdivision = 4)
        pattern.quantize_all(4, sample_rate, &tempo, &time_signature);

        // Note should now be exactly at bar 1, beat 1, tick 0
        let quantized_note = &pattern.notes()[0];
        assert_eq!(quantized_note.start.samples, 0);
    }
}
