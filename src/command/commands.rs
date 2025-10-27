// Concrete command implementations

use crate::command::state::DawState;
use crate::command::trait_def::{CommandError, CommandResult, UndoableCommand};
use crate::messaging::command::Command;
use crate::synth::envelope::AdsrParams;
use crate::synth::filter::FilterParams;
use crate::synth::lfo::LfoParams;
use crate::synth::modulation::ModRouting;
use crate::synth::oscillator::WaveformType;
use crate::synth::poly_mode::PolyMode;
use crate::synth::portamento::PortamentoParams;
use crate::synth::voice_manager::VoiceMode;

/// Command to set the volume
///
/// This command changes the volume and sends the update to the audio thread.
/// It stores the old volume value to enable undo.
pub struct SetVolumeCommand {
    new_volume: f32,
    old_volume: Option<f32>,
}

impl SetVolumeCommand {
    /// Create a new SetVolumeCommand
    ///
    /// # Arguments
    /// * `volume` - The new volume value (should be between 0.0 and 1.0)
    pub fn new(volume: f32) -> Self {
        Self {
            new_volume: volume.clamp(0.0, 1.0),
            old_volume: None,
        }
    }
}

impl UndoableCommand for SetVolumeCommand {
    fn execute(&mut self, state: &mut DawState) -> CommandResult<()> {
        // Store old value for undo
        self.old_volume = Some(state.volume);

        // Update state
        state.volume = self.new_volume;

        // Send to audio thread
        if !state.send_to_audio(Command::SetVolume(self.new_volume)) {
            return Err(CommandError::ExecutionFailed(
                "Failed to send volume command to audio thread (ringbuffer full)".into(),
            ));
        }

        Ok(())
    }

    fn undo(&mut self, state: &mut DawState) -> CommandResult<()> {
        let old_volume = self
            .old_volume
            .ok_or_else(|| CommandError::UndoFailed("No previous volume stored".into()))?;

        // Restore old value
        state.volume = old_volume;

        // Send to audio thread
        if !state.send_to_audio(Command::SetVolume(old_volume)) {
            return Err(CommandError::UndoFailed(
                "Failed to send volume command to audio thread (ringbuffer full)".into(),
            ));
        }

        Ok(())
    }

    fn description(&self) -> String {
        format!("Set Volume to {:.2}", self.new_volume)
    }

    fn can_merge_with(&self, other: &dyn UndoableCommand) -> bool {
        // We can merge with other SetVolumeCommand to avoid cluttering history
        // when user drags a slider
        other.description().starts_with("Set Volume")
    }

    fn merge_with(&mut self, other: Box<dyn UndoableCommand>) -> CommandResult<()> {
        // Downcast to SetVolumeCommand
        // This is safe because can_merge_with already verified it's a SetVolumeCommand
        let other_any = Box::into_raw(other) as *mut SetVolumeCommand;

        unsafe {
            let other_cmd = Box::from_raw(other_any);
            // Update to the new value but keep the original old_volume
            self.new_volume = other_cmd.new_volume;
        }

        Ok(())
    }
}

/// Command to set the waveform type
///
/// This command changes the oscillator waveform and sends the update to the audio thread.
/// It stores the old waveform to enable undo.
pub struct SetWaveformCommand {
    new_waveform: WaveformType,
    old_waveform: Option<WaveformType>,
}

impl SetWaveformCommand {
    /// Create a new SetWaveformCommand
    ///
    /// # Arguments
    /// * `waveform` - The new waveform type
    pub fn new(waveform: WaveformType) -> Self {
        Self {
            new_waveform: waveform,
            old_waveform: None,
        }
    }
}

impl UndoableCommand for SetWaveformCommand {
    fn execute(&mut self, state: &mut DawState) -> CommandResult<()> {
        // Store old value for undo
        self.old_waveform = Some(state.waveform);

        // Update state
        state.waveform = self.new_waveform;

        // Send to audio thread
        if !state.send_to_audio(Command::SetWaveform(self.new_waveform)) {
            return Err(CommandError::ExecutionFailed(
                "Failed to send waveform command to audio thread (ringbuffer full)".into(),
            ));
        }

        Ok(())
    }

    fn undo(&mut self, state: &mut DawState) -> CommandResult<()> {
        let old_waveform = self
            .old_waveform
            .ok_or_else(|| CommandError::UndoFailed("No previous waveform stored".into()))?;

        // Restore old value
        state.waveform = old_waveform;

        // Send to audio thread
        if !state.send_to_audio(Command::SetWaveform(old_waveform)) {
            return Err(CommandError::UndoFailed(
                "Failed to send waveform command to audio thread (ringbuffer full)".into(),
            ));
        }

        Ok(())
    }

    fn description(&self) -> String {
        format!("Set Waveform to {:?}", self.new_waveform)
    }
}

/// Command to set ADSR envelope parameters
///
/// This command changes the ADSR parameters for all voices and sends the update to the audio thread.
/// It stores the old parameters to enable undo.
pub struct SetAdsrCommand {
    new_params: AdsrParams,
    old_params: Option<AdsrParams>,
}

impl SetAdsrCommand {
    /// Create a new SetAdsrCommand
    ///
    /// # Arguments
    /// * `params` - The new ADSR parameters
    pub fn new(params: AdsrParams) -> Self {
        Self {
            new_params: params,
            old_params: None,
        }
    }

    /// Create a command for a specific ADSR parameter
    pub fn attack(attack: f32) -> Self {
        let params = AdsrParams::new(attack, 0.1, 0.7, 0.2); // Use defaults for other params
        Self::new(params)
    }

    pub fn decay(decay: f32) -> Self {
        let params = AdsrParams::new(0.01, decay, 0.7, 0.2);
        Self::new(params)
    }

    pub fn sustain(sustain: f32) -> Self {
        let params = AdsrParams::new(0.01, 0.1, sustain, 0.2);
        Self::new(params)
    }

    pub fn release(release: f32) -> Self {
        let params = AdsrParams::new(0.01, 0.1, 0.7, release);
        Self::new(params)
    }
}

impl UndoableCommand for SetAdsrCommand {
    fn execute(&mut self, state: &mut DawState) -> CommandResult<()> {
        // Store old value for undo
        self.old_params = Some(state.adsr);

        // Update state
        state.adsr = self.new_params;

        // Send to audio thread
        if !state.send_to_audio(Command::SetAdsr(self.new_params)) {
            return Err(CommandError::ExecutionFailed(
                "Failed to send ADSR command to audio thread (ringbuffer full)".into(),
            ));
        }

        Ok(())
    }

    fn undo(&mut self, state: &mut DawState) -> CommandResult<()> {
        let old_params = self
            .old_params
            .ok_or_else(|| CommandError::UndoFailed("No previous ADSR parameters stored".into()))?;

        // Restore old value
        state.adsr = old_params;

        // Send to audio thread
        if !state.send_to_audio(Command::SetAdsr(old_params)) {
            return Err(CommandError::UndoFailed(
                "Failed to send ADSR command to audio thread (ringbuffer full)".into(),
            ));
        }

        Ok(())
    }

    fn description(&self) -> String {
        format!(
            "Set ADSR (A:{:.3}s D:{:.3}s S:{:.2} R:{:.3}s)",
            self.new_params.attack,
            self.new_params.decay,
            self.new_params.sustain,
            self.new_params.release
        )
    }
}

/// Command to set LFO parameters
///
/// This command changes the LFO parameters for all voices and sends the update to the audio thread.
/// It stores the old parameters to enable undo.
pub struct SetLfoCommand {
    new_params: LfoParams,
    old_params: Option<LfoParams>,
}

impl SetLfoCommand {
    /// Create a new SetLfoCommand
    ///
    /// # Arguments
    /// * `params` - The new LFO parameters
    pub fn new(params: LfoParams) -> Self {
        Self {
            new_params: params,
            old_params: None,
        }
    }
}

impl UndoableCommand for SetLfoCommand {
    fn execute(&mut self, state: &mut DawState) -> CommandResult<()> {
        // Store old value for undo
        self.old_params = Some(state.lfo);

        // Update state
        state.lfo = self.new_params;

        // Send to audio thread
        if !state.send_to_audio(Command::SetLfo(self.new_params)) {
            return Err(CommandError::ExecutionFailed(
                "Failed to send LFO command to audio thread (ringbuffer full)".into(),
            ));
        }

        Ok(())
    }

    fn undo(&mut self, state: &mut DawState) -> CommandResult<()> {
        let old_params = self
            .old_params
            .ok_or_else(|| CommandError::UndoFailed("No previous LFO parameters stored".into()))?;

        // Restore old value
        state.lfo = old_params;

        // Send to audio thread
        if !state.send_to_audio(Command::SetLfo(old_params)) {
            return Err(CommandError::UndoFailed(
                "Failed to send LFO command to audio thread (ringbuffer full)".into(),
            ));
        }

        Ok(())
    }

    fn description(&self) -> String {
        format!(
            "Set LFO ({:?} {:.1}Hz depth:{:.2} → {:?})",
            self.new_params.waveform,
            self.new_params.rate,
            self.new_params.depth,
            self.new_params.destination
        )
    }

    fn can_merge_with(&self, other: &dyn UndoableCommand) -> bool {
        // We can merge with other SetLfoCommand to avoid cluttering history
        // when user adjusts LFO parameters
        other.description().starts_with("Set LFO")
    }

    fn merge_with(&mut self, other: Box<dyn UndoableCommand>) -> CommandResult<()> {
        // Downcast to SetLfoCommand
        let other_any = Box::into_raw(other) as *mut SetLfoCommand;

        unsafe {
            let other_cmd = Box::from_raw(other_any);
            // Update to the new value but keep the original old_params
            self.new_params = other_cmd.new_params;
        }

        Ok(())
    }
}

/// Command to set a modulation routing (MVP)
pub struct SetModRoutingCommand {
    index: u8,
    new_routing: ModRouting,
    old_routing: Option<ModRouting>,
}

impl SetModRoutingCommand {
    pub fn new(index: u8, routing: ModRouting) -> Self {
        Self {
            index,
            new_routing: routing,
            old_routing: None,
        }
    }

    /// Provide the previous routing so undo can fully restore it
    pub fn new_with_old(index: u8, new_routing: ModRouting, old_routing: ModRouting) -> Self {
        Self {
            index,
            new_routing,
            old_routing: Some(old_routing),
        }
    }
}

impl UndoableCommand for SetModRoutingCommand {
    fn execute(&mut self, state: &mut DawState) -> CommandResult<()> {
        // Update UI/command state mirror
        let idx = self.index as usize;
        if idx < state.mod_routings.len() {
            // Save previous routing if not already provided
            if self.old_routing.is_none() {
                self.old_routing = Some(state.mod_routings[idx]);
            }
            state.mod_routings[idx] = self.new_routing;
        }

        // Send to audio thread
        if !state.send_to_audio(Command::SetModRouting {
            index: self.index,
            routing: self.new_routing,
        }) {
            return Err(CommandError::ExecutionFailed(
                "Failed to send SetModRouting to audio thread (ringbuffer full)".into(),
            ));
        }

        Ok(())
    }

    fn undo(&mut self, state: &mut DawState) -> CommandResult<()> {
        if let Some(old) = self.old_routing {
            // Restore mirror state
            let idx = self.index as usize;
            if idx < state.mod_routings.len() {
                state.mod_routings[idx] = old;
            }
            if !state.send_to_audio(Command::SetModRouting {
                index: self.index,
                routing: old,
            }) {
                return Err(CommandError::UndoFailed(
                    "Failed to send SetModRouting (undo) to audio thread".into(),
                ));
            }
            Ok(())
        } else {
            // If we don't know the old routing, clear the slot as a safe fallback
            let idx = self.index as usize;
            if idx < state.mod_routings.len() {
                state.mod_routings[idx].enabled = false;
                state.mod_routings[idx].amount = 0.0;
            }
            if !state.send_to_audio(Command::ClearModRouting { index: self.index }) {
                return Err(CommandError::UndoFailed(
                    "Failed to send ClearModRouting (undo fallback) to audio thread".into(),
                ));
            }
            Ok(())
        }
    }

    fn description(&self) -> String {
        format!("Set Mod Routing #{}", self.index)
    }

    fn can_merge_with(&self, other: &dyn UndoableCommand) -> bool {
        other.description().starts_with("Set Mod Routing #")
    }

    fn merge_with(&mut self, other: Box<dyn UndoableCommand>) -> CommandResult<()> {
        // Try to downcast by description convention; keep last routing value
        // Safety: we do not perform actual downcast; just replace new_routing if other shares the description prefix
        let _ = other; // not used in MVP
        Ok(())
    }
}

/// Command to set polyphony mode
///
/// This command changes the polyphony mode (Poly, Mono, Legato) and sends the update to the audio thread.
/// It stores the old mode to enable undo.
pub struct SetPolyModeCommand {
    new_mode: PolyMode,
    old_mode: Option<PolyMode>,
}

impl SetPolyModeCommand {
    /// Create a new SetPolyModeCommand
    ///
    /// # Arguments
    /// * `mode` - The new polyphony mode
    pub fn new(mode: PolyMode) -> Self {
        Self {
            new_mode: mode,
            old_mode: None,
        }
    }
}

impl UndoableCommand for SetPolyModeCommand {
    fn execute(&mut self, state: &mut DawState) -> CommandResult<()> {
        // Store old value for undo
        self.old_mode = Some(state.poly_mode);

        // Update state
        state.poly_mode = self.new_mode;

        // Send to audio thread
        if !state.send_to_audio(Command::SetPolyMode(self.new_mode)) {
            return Err(CommandError::ExecutionFailed(
                "Failed to send PolyMode command to audio thread (ringbuffer full)".into(),
            ));
        }

        Ok(())
    }

    fn undo(&mut self, state: &mut DawState) -> CommandResult<()> {
        let old_mode = self
            .old_mode
            .ok_or_else(|| CommandError::UndoFailed("No previous polyphony mode stored".into()))?;

        // Restore old value
        state.poly_mode = old_mode;

        // Send to audio thread
        if !state.send_to_audio(Command::SetPolyMode(old_mode)) {
            return Err(CommandError::UndoFailed(
                "Failed to send PolyMode command to audio thread (ringbuffer full)".into(),
            ));
        }

        Ok(())
    }

    fn description(&self) -> String {
        format!("Set Polyphony Mode to {:?}", self.new_mode)
    }
}

/// Command to set portamento parameters
///
/// This command changes the portamento glide time for all voices and sends the update to the audio thread.
/// It stores the old parameters to enable undo.
pub struct SetPortamentoCommand {
    new_params: PortamentoParams,
    old_params: Option<PortamentoParams>,
}

impl SetPortamentoCommand {
    /// Create a new SetPortamentoCommand
    ///
    /// # Arguments
    /// * `params` - The new portamento parameters
    pub fn new(params: PortamentoParams) -> Self {
        Self {
            new_params: params,
            old_params: None,
        }
    }
}

impl UndoableCommand for SetPortamentoCommand {
    fn execute(&mut self, state: &mut DawState) -> CommandResult<()> {
        // Store old value for undo
        self.old_params = Some(state.portamento);

        // Update state
        state.portamento = self.new_params;

        // Send to audio thread
        if !state.send_to_audio(Command::SetPortamento(self.new_params)) {
            return Err(CommandError::ExecutionFailed(
                "Failed to send Portamento command to audio thread (ringbuffer full)".into(),
            ));
        }

        Ok(())
    }

    fn undo(&mut self, state: &mut DawState) -> CommandResult<()> {
        let old_params = self.old_params.ok_or_else(|| {
            CommandError::UndoFailed("No previous portamento parameters stored".into())
        })?;

        // Restore old value
        state.portamento = old_params;

        // Send to audio thread
        if !state.send_to_audio(Command::SetPortamento(old_params)) {
            return Err(CommandError::UndoFailed(
                "Failed to send Portamento command to audio thread (ringbuffer full)".into(),
            ));
        }

        Ok(())
    }

    fn description(&self) -> String {
        if self.new_params.time == 0.0 {
            "Set Portamento (Off)".to_string()
        } else {
            format!("Set Portamento ({:.2}s)", self.new_params.time)
        }
    }

    fn can_merge_with(&self, other: &dyn UndoableCommand) -> bool {
        // We can merge with other SetPortamentoCommand to avoid cluttering history
        // when user adjusts the portamento slider
        other.description().starts_with("Set Portamento")
    }

    fn merge_with(&mut self, other: Box<dyn UndoableCommand>) -> CommandResult<()> {
        // Downcast to SetPortamentoCommand
        let other_any = Box::into_raw(other) as *mut SetPortamentoCommand;

        unsafe {
            let other_cmd = Box::from_raw(other_any);
            // Update to the new value but keep the original old_params
            self.new_params = other_cmd.new_params;
        }

        Ok(())
    }
}

/// Command to set filter parameters
///
/// This command changes the filter parameters for all voices and sends the update to the audio thread.
/// It stores the old parameters to enable undo.
pub struct SetFilterCommand {
    new_params: FilterParams,
    old_params: Option<FilterParams>,
}

impl SetFilterCommand {
    /// Create a new SetFilterCommand
    ///
    /// # Arguments
    /// * `params` - The new filter parameters
    pub fn new(params: FilterParams) -> Self {
        Self {
            new_params: params,
            old_params: None,
        }
    }
}

impl UndoableCommand for SetFilterCommand {
    fn execute(&mut self, state: &mut DawState) -> CommandResult<()> {
        // Store old value for undo
        self.old_params = Some(state.filter);

        // Update state
        state.filter = self.new_params;

        // Send to audio thread
        if !state.send_to_audio(Command::SetFilter(self.new_params)) {
            return Err(CommandError::ExecutionFailed(
                "Failed to send Filter command to audio thread (ringbuffer full)".into(),
            ));
        }

        Ok(())
    }

    fn undo(&mut self, state: &mut DawState) -> CommandResult<()> {
        let old_params = self.old_params.ok_or_else(|| {
            CommandError::UndoFailed("No previous filter parameters stored".into())
        })?;

        // Restore old value
        state.filter = old_params;

        // Send to audio thread
        if !state.send_to_audio(Command::SetFilter(old_params)) {
            return Err(CommandError::UndoFailed(
                "Failed to send Filter command to audio thread (ringbuffer full)".into(),
            ));
        }

        Ok(())
    }

    fn description(&self) -> String {
        format!(
            "Set Filter ({:?} cutoff:{:.0}Hz Q:{:.2})",
            self.new_params.filter_type, self.new_params.cutoff, self.new_params.resonance
        )
    }

    fn can_merge_with(&self, other: &dyn UndoableCommand) -> bool {
        // We can merge with other SetFilterCommand to avoid cluttering history
        // when user adjusts filter parameters
        other.description().starts_with("Set Filter")
    }

    fn merge_with(&mut self, other: Box<dyn UndoableCommand>) -> CommandResult<()> {
        // Downcast to SetFilterCommand
        let other_any = Box::into_raw(other) as *mut SetFilterCommand;

        unsafe {
            let other_cmd = Box::from_raw(other_any);
            // Update to the new value but keep the original old_params
            self.new_params = other_cmd.new_params;
        }

        Ok(())
    }
}

/// Command to set the voice mode (Synth or Sampler)
pub struct SetVoiceModeCommand {
    new_mode: VoiceMode,
    old_mode: Option<VoiceMode>,
}

impl SetVoiceModeCommand {
    pub fn new(mode: VoiceMode) -> Self {
        Self {
            new_mode: mode,
            old_mode: None,
        }
    }
}

impl UndoableCommand for SetVoiceModeCommand {
    fn execute(&mut self, state: &mut DawState) -> CommandResult<()> {
        self.old_mode = Some(state.voice_mode);
        state.voice_mode = self.new_mode;
        if !state.send_to_audio(Command::SetVoiceMode(self.new_mode)) {
            return Err(CommandError::ExecutionFailed(
                "Failed to send VoiceMode command to audio thread (ringbuffer full)".into(),
            ));
        }
        Ok(())
    }

    fn undo(&mut self, state: &mut DawState) -> CommandResult<()> {
        let old_mode = self
            .old_mode
            .ok_or_else(|| CommandError::UndoFailed("No previous voice mode stored".into()))?;
        state.voice_mode = old_mode;
        if !state.send_to_audio(Command::SetVoiceMode(old_mode)) {
            return Err(CommandError::UndoFailed(
                "Failed to send VoiceMode command to audio thread (ringbuffer full)".into(),
            ));
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("Set Voice Mode to {:?}", self.new_mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::channels::create_command_channel;
    use std::sync::{Arc, Mutex};

    fn create_test_state() -> DawState {
        let (tx, _rx) = create_command_channel(128);
        DawState::new(Arc::new(Mutex::new(tx)))
    }

    #[test]
    fn test_set_volume_command() {
        let mut state = create_test_state();
        let mut cmd = SetVolumeCommand::new(0.8);

        // Execute
        assert_eq!(state.volume, 0.5); // default
        cmd.execute(&mut state).unwrap();
        assert_eq!(state.volume, 0.8);

        // Undo
        cmd.undo(&mut state).unwrap();
        assert_eq!(state.volume, 0.5);
    }

    #[test]
    fn test_set_volume_clamps() {
        let cmd = SetVolumeCommand::new(1.5);
        assert_eq!(cmd.new_volume, 1.0);

        let cmd = SetVolumeCommand::new(-0.5);
        assert_eq!(cmd.new_volume, 0.0);
    }

    #[test]
    fn test_set_waveform_command() {
        let mut state = create_test_state();
        let mut cmd = SetWaveformCommand::new(WaveformType::Square);

        // Execute
        assert_eq!(state.waveform, WaveformType::Sine); // default
        cmd.execute(&mut state).unwrap();
        assert_eq!(state.waveform, WaveformType::Square);

        // Undo
        cmd.undo(&mut state).unwrap();
        assert_eq!(state.waveform, WaveformType::Sine);
    }

    #[test]
    fn test_volume_command_merge() {
        let mut cmd1 = SetVolumeCommand::new(0.5);
        let cmd2 = SetVolumeCommand::new(0.8);

        // Should be able to merge
        assert!(cmd1.can_merge_with(&cmd2));

        // Merge
        cmd1.merge_with(Box::new(cmd2)).unwrap();
        assert_eq!(cmd1.new_volume, 0.8);
    }

    #[test]
    fn test_volume_description() {
        let cmd = SetVolumeCommand::new(0.75);
        assert_eq!(cmd.description(), "Set Volume to 0.75");
    }

    #[test]
    fn test_waveform_description() {
        let cmd = SetWaveformCommand::new(WaveformType::Saw);
        assert_eq!(cmd.description(), "Set Waveform to Saw");
    }
}
