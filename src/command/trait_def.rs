// UndoableCommand trait definition

use crate::command::state::DawState;
use std::fmt;

/// Result type for command operations
pub type CommandResult<T> = Result<T, CommandError>;

/// Errors that can occur during command execution
#[derive(Debug, Clone)]
pub enum CommandError {
    /// Command execution failed
    ExecutionFailed(String),
    /// Undo operation failed
    UndoFailed(String),
    /// Invalid state for this operation
    InvalidState(String),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            CommandError::UndoFailed(msg) => write!(f, "Undo failed: {}", msg),
            CommandError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
        }
    }
}

impl std::error::Error for CommandError {}

/// Trait for commands that support undo/redo
///
/// All state-changing operations in the DAW should implement this trait.
/// Commands are executed on the UI thread and may send messages to the audio thread.
///
/// # Thread Safety
/// Commands must be Send as they may be moved between threads.
///
/// # Example
/// ```no_run
/// use mymusic_daw::command::trait_def::{UndoableCommand, CommandResult, CommandError};
/// use mymusic_daw::command::state::DawState;
///
/// struct SetVolumeCommand {
///     new_volume: f32,
///     old_volume: Option<f32>,
/// }
///
/// impl UndoableCommand for SetVolumeCommand {
///     fn execute(&mut self, state: &mut DawState) -> CommandResult<()> {
///         self.old_volume = Some(state.volume);
///         state.volume = self.new_volume;
///         Ok(())
///     }
///
///     fn undo(&mut self, state: &mut DawState) -> CommandResult<()> {
///         if let Some(old) = self.old_volume {
///             state.volume = old;
///             Ok(())
///         } else {
///             Err(CommandError::UndoFailed("No old state stored".into()))
///         }
///     }
///
///     fn description(&self) -> String {
///         format!("Set Volume to {:.2}", self.new_volume)
///     }
/// }
/// ```
pub trait UndoableCommand: Send {
    /// Execute the command
    ///
    /// Should store the previous state internally for undo capability.
    /// May send messages to the audio thread via DawState's command channels.
    fn execute(&mut self, state: &mut DawState) -> CommandResult<()>;

    /// Undo the command
    ///
    /// Restores the state to what it was before execute() was called.
    /// Should use the stored previous state.
    fn undo(&mut self, state: &mut DawState) -> CommandResult<()>;

    /// Get a human-readable description of the command
    ///
    /// Used for UI display (e.g., "Undo: Set Volume to 0.5")
    fn description(&self) -> String;

    /// Optional: Check if this command can be merged with another
    ///
    /// Useful for combining multiple similar commands (e.g., dragging a slider)
    /// Default implementation returns false.
    fn can_merge_with(&self, _other: &dyn UndoableCommand) -> bool {
        false
    }

    /// Optional: Merge this command with another
    ///
    /// Only called if can_merge_with() returned true.
    /// Default implementation does nothing.
    fn merge_with(&mut self, _other: Box<dyn UndoableCommand>) -> CommandResult<()> {
        Ok(())
    }
}
