// Types de commandes - Communication UI → Audio

use crate::midi::event::MidiEventTimed;
use crate::sampler::loader::Sample;
use crate::sequencer::Pattern;
use crate::synth::envelope::AdsrParams;
use crate::synth::filter::FilterParams;
use crate::synth::lfo::LfoParams;
use crate::synth::modulation::ModRouting;
use crate::synth::oscillator::WaveformType;
use crate::synth::poly_mode::PolyMode;
use crate::synth::portamento::PortamentoParams;
use crate::synth::voice_manager::VoiceMode;
use std::sync::Arc;

#[derive(Debug, Clone)]
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
    AddSample(Arc<Sample>),
    RemoveSample(usize),
    SetNoteSampleMapping {
        note: u8,
        sample_index: usize,
    },
    UpdateSample(usize, Arc<Sample>),
    /// Update a modulation routing slot (UI → Audio)
    SetModRouting {
        index: u8,
        routing: ModRouting,
    },
    /// Clear a modulation routing slot
    ClearModRouting {
        index: u8,
    },
    /// Enable/disable metronome
    SetMetronomeEnabled(bool),
    /// Set metronome volume (0.0 to 1.0)
    SetMetronomeVolume(f32),
    /// Set transport tempo (BPM)
    SetTempo(f64),
    /// Set transport time signature (numerator, denominator)
    SetTimeSignature(u8, u8),
    /// Set transport playing state (true = playing, false = stopped)
    SetTransportPlaying(bool),
    /// Set transport position in samples
    SetTransportPosition(u64),
    /// Update the active pattern for sequencer playback
    SetPattern(Pattern),
    Quit,
}
