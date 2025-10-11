// Types de commandes - Communication UI → Audio

use crate::midi::event::MidiEventTimed;
use crate::synth::oscillator::WaveformType;

#[derive(Debug, Clone, Copy)]
pub enum Command {
    Midi(MidiEventTimed),
    SetVolume(f32),
    SetWaveform(WaveformType),
    Quit,
}
