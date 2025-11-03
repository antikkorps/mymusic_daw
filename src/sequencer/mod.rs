// Sequencer module - Phase 4
// Timeline, musical time representation, and sequencing infrastructure

pub mod metronome;
pub mod midi_recorder;
pub mod note;
pub mod pattern;
pub mod player;
pub mod timeline;
pub mod transport;

pub use metronome::{ClickType, Metronome, MetronomeScheduler, MetronomeSound};
pub use midi_recorder::MidiRecorder;
pub use note::{Note, NoteId};
pub use pattern::{Pattern, PatternId, generate_note_id};
pub use player::SequencerPlayer;
pub use timeline::{MusicalTime, Position, Tempo, TimeSignature};
pub use transport::{Transport, TransportState};
