// Types de commandes - Communication UI → Audio

use crate::midi::event::MidiEvent;

#[derive(Debug, Clone, Copy)]
pub enum Command {
    Midi(MidiEvent),
    SetVolume(f32),
    Quit,
}
