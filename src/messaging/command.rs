// Types de commandes - Communication UI → Audio

use crate::midi::event::MidiEvent;
use crate::synth::oscillator::WaveformType;

#[derive(Debug, Clone, Copy)]
pub enum Command {
    Midi(MidiEvent),
    SetVolume(f32),
    SetWaveform(WaveformType),
    Quit,
}
