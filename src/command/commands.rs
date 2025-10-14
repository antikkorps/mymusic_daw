// Concrete command implementations

use crate::command::state::DawState;
use crate::command::trait_def::{UndoableCommand, CommandResult, CommandError};
use crate::messaging::command::Command;
use crate::synth::envelope::AdsrParams;
use crate::synth::oscillator::WaveformType;

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
                "Failed to send volume command to audio thread (ringbuffer full)".into()
            ));
        }

        Ok(())
    }

    fn undo(&mut self, state: &mut DawState) -> CommandResult<()> {
        let old_volume = self.old_volume
            .ok_or_else(|| CommandError::UndoFailed("No previous volume stored".into()))?;

        // Restore old value
        state.volume = old_volume;

        // Send to audio thread
        if !state.send_to_audio(Command::SetVolume(old_volume)) {
            return Err(CommandError::UndoFailed(
                "Failed to send volume command to audio thread (ringbuffer full)".into()
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
        let other_any = Box::into_raw(other) as *mut dyn UndoableCommand as *mut SetVolumeCommand;

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
                "Failed to send waveform command to audio thread (ringbuffer full)".into()
            ));
        }

        Ok(())
    }

    fn undo(&mut self, state: &mut DawState) -> CommandResult<()> {
        let old_waveform = self.old_waveform
            .ok_or_else(|| CommandError::UndoFailed("No previous waveform stored".into()))?;

        // Restore old value
        state.waveform = old_waveform;

        // Send to audio thread
        if !state.send_to_audio(Command::SetWaveform(old_waveform)) {
            return Err(CommandError::UndoFailed(
                "Failed to send waveform command to audio thread (ringbuffer full)".into()
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
                "Failed to send ADSR command to audio thread (ringbuffer full)".into()
            ));
        }

        Ok(())
    }

    fn undo(&mut self, state: &mut DawState) -> CommandResult<()> {
        let old_params = self.old_params
            .ok_or_else(|| CommandError::UndoFailed("No previous ADSR parameters stored".into()))?;

        // Restore old value
        state.adsr = old_params;

        // Send to audio thread
        if !state.send_to_audio(Command::SetAdsr(old_params)) {
            return Err(CommandError::UndoFailed(
                "Failed to send ADSR command to audio thread (ringbuffer full)".into()
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
