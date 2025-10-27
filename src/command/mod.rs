// Command Pattern for Undo/Redo functionality
//
// This module implements the Command Pattern to enable undo/redo for all
// DAW operations. All state-changing operations should go through UndoableCommand.
//
// Architecture:
// - UndoableCommand trait: Defines execute(), undo(), description()
// - CommandManager: Manages undo/redo stacks
// - Concrete commands: SetVolumeCommand, SetWaveformCommand, etc.
//
// Integration with audio thread:
// - Commands execute on UI thread and update DawState
// - They send low-level Command messages via ringbuffer to audio thread
// - Store previous state for undo capability

pub mod commands;
pub mod manager;
pub mod state;
pub mod trait_def;

pub use manager::CommandManager;
pub use state::DawState;
pub use trait_def::UndoableCommand;
