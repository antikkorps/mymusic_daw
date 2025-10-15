// DawState - Centralized mutable state for the DAW
//
// This struct holds all the mutable state that commands can modify.
// It also holds the communication channels to send messages to the audio thread.

use crate::messaging::channels::CommandProducer;
use crate::synth::envelope::AdsrParams;
use crate::synth::lfo::LfoParams;
use crate::synth::oscillator::WaveformType;
use crate::synth::poly_mode::PolyMode;
use crate::synth::portamento::PortamentoParams;
use std::sync::{Arc, Mutex};

/// Central state of the DAW that can be modified by commands
///
/// This struct acts as the single source of truth for UI state.
/// Commands modify this state, and changes are propagated to the audio thread
/// via the command sender.
pub struct DawState {
    /// Current volume (0.0 to 1.0)
    pub volume: f32,

    /// Current waveform type
    pub waveform: WaveformType,

    /// ADSR envelope parameters
    pub adsr: AdsrParams,

    /// LFO parameters
    pub lfo: LfoParams,

    /// Polyphony mode
    pub poly_mode: PolyMode,

    /// Portamento parameters
    pub portamento: PortamentoParams,

    /// Command sender to communicate with audio thread (UI channel)
    /// Wrapped in Arc<Mutex<>> to allow sharing between DawApp and commands
    pub command_sender: Arc<Mutex<CommandProducer>>,

    // Future parameters will be added here:
    // pub filter: FilterParams,
    // etc.
}

impl DawState {
    /// Create a new DawState with default values
    pub fn new(command_sender: Arc<Mutex<CommandProducer>>) -> Self {
        Self {
            volume: 0.5,
            waveform: WaveformType::Sine,
            adsr: AdsrParams::default(),
            lfo: LfoParams::default(),
            poly_mode: PolyMode::default(),
            portamento: PortamentoParams::default(),
            command_sender,
        }
    }

    /// Send a command to the audio thread
    ///
    /// This is used internally by commands to propagate changes to the audio engine.
    /// Returns true if the message was sent successfully, false if the ringbuffer is full.
    pub fn send_to_audio(&mut self, command: crate::messaging::command::Command) -> bool {
        if let Ok(mut sender) = self.command_sender.lock() {
            ringbuf::traits::Producer::try_push(&mut *sender, command).is_ok()
        } else {
            false
        }
    }
}
