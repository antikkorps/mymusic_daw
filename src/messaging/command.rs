// Types de commandes - Communication UI → Audio

use crate::midi::event::MidiEventTimed;
use crate::synth::envelope::AdsrParams;
use crate::synth::filter::FilterParams;
use crate::synth::lfo::LfoParams;
use crate::synth::oscillator::WaveformType;
use crate::synth::poly_mode::PolyMode;
use crate::synth::modulation::ModRouting;
use crate::synth::portamento::PortamentoParams;

use crate::synth::voice_manager::VoiceMode;

#[derive(Debug, Clone, Copy)]
pub enum Command {
    Midi(MidiEventTimed),
    SetVolume(f32),
    SetWaveform(WaveformType),
    SetAdsr(AdsrParams),
    SetLfo(LfoParams),
    SetPolyMode(PolyMode),
    SetPortamento(PortamentoParams),
    SetFilter(FilterParams),
    SetVoiceMode(VoiceMode),
    /// Update a modulation routing slot (UI → Audio)
    SetModRouting { index: u8, routing: ModRouting },
    /// Clear a modulation routing slot
    ClearModRouting { index: u8 },
    Quit,
}
