// MyMusic DAW - Library exports for tests and benchmarks

pub mod audio;
pub mod command;
pub mod connection;
pub mod messaging;
pub mod midi;
pub mod sampler;
pub mod sequencer;
pub mod synth;
pub mod ui;

// Re-export commonly used types for convenience
pub use audio::engine::AudioEngine;
pub use audio::timing::AudioTiming;
pub use command::{CommandManager, DawState, UndoableCommand};
pub use messaging::channels::{create_command_channel, create_notification_channel};
pub use midi::event::{MidiEvent, MidiEventTimed};
pub use midi::manager::MidiConnectionManager;
pub use sequencer::{
    ClickType, Metronome, MetronomeScheduler, MusicalTime, Position, Tempo, TimeSignature,
    Transport, TransportState,
};
pub use synth::envelope::AdsrParams;
pub use synth::oscillator::{Oscillator, SimpleOscillator, WaveformType};
pub use synth::voice::Voice;
pub use synth::voice_manager::VoiceManager;
