// Types de commandes - Communication UI → Audio

use crate::midi::event::MidiEventTimed;
use crate::synth::envelope::AdsrParams;
use crate::synth::lfo::LfoParams;
use crate::synth::oscillator::WaveformType;
use crate::synth::poly_mode::PolyMode;
use crate::synth::portamento::PortamentoParams;

#[derive(Debug, Clone, Copy)]
pub enum Command {
    Midi(MidiEventTimed),
    SetVolume(f32),
    SetWaveform(WaveformType),
    SetAdsr(AdsrParams),
    SetLfo(LfoParams),
    SetPolyMode(PolyMode),
    SetPortamento(PortamentoParams),
    Quit,
}
