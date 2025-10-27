// CommandManager - Manages undo/redo stacks

use crate::command::state::DawState;
use crate::command::trait_def::{CommandError, CommandResult, UndoableCommand};
use std::collections::VecDeque;

/// Default maximum number of commands to keep in history
const DEFAULT_MAX_HISTORY: usize = 100;

/// Manages command execution and undo/redo functionality
///
/// The CommandManager maintains two stacks:
/// - Undo stack: Commands that have been executed and can be undone
/// - Redo stack: Commands that have been undone and can be redone
///
/// When a new command is executed:
/// 1. Execute the command
/// 2. Push it onto the undo stack
/// 3. Clear the redo stack (since we're on a new timeline)
///
/// # Memory Management
/// The manager limits the number of commands in the undo stack to prevent
/// unbounded memory growth. When the limit is reached, the oldest command
/// is removed.
pub struct CommandManager {
    /// Stack of commands that can be undone (most recent at the back)
    undo_stack: VecDeque<Box<dyn UndoableCommand>>,

    /// Stack of commands that can be redone (most recent at the back)
    redo_stack: VecDeque<Box<dyn UndoableCommand>>,

    /// Maximum number of commands to keep in history
    max_history: usize,
}

impl CommandManager {
    /// Create a new CommandManager with default settings
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_MAX_HISTORY)
    }

    /// Create a new CommandManager with a custom history limit
    pub fn with_capacity(max_history: usize) -> Self {
        Self {
            undo_stack: VecDeque::with_capacity(max_history),
            redo_stack: VecDeque::with_capacity(max_history),
            max_history,
        }
    }

    /// Execute a command and add it to the undo stack
    ///
    /// This will:
    /// 1. Execute the command
    /// 2. Add it to the undo stack (if successful)
    /// 3. Clear the redo stack (new timeline)
    /// 4. Trim history if needed
    ///
    /// # Errors
    /// Returns an error if the command execution fails.
    pub fn execute(
        &mut self,
        mut command: Box<dyn UndoableCommand>,
        state: &mut DawState,
    ) -> CommandResult<()> {
        // Execute the command
        command.execute(state)?;

        // TODO: Implement command merging for slider operations
        // This requires downcasting or a different approach with type IDs

        // Add to undo stack
        self.undo_stack.push_back(command);

        // Clear redo stack (we're on a new timeline now)
        self.redo_stack.clear();

        // Trim history if needed
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.pop_front();
        }

        Ok(())
    }

    /// Undo the last command
    ///
    /// Pops the last command from the undo stack, undoes it, and pushes it to the redo stack.
    ///
    /// # Errors
    /// Returns an error if:
    /// - There are no commands to undo
    /// - The undo operation fails
    pub fn undo(&mut self, state: &mut DawState) -> CommandResult<String> {
        let mut command = self
            .undo_stack
            .pop_back()
            .ok_or_else(|| CommandError::UndoFailed("Nothing to undo".into()))?;

        let description = command.description();

        // Undo the command
        command.undo(state)?;

        // Move to redo stack
        self.redo_stack.push_back(command);

        Ok(description)
    }

    /// Redo the last undone command
    ///
    /// Pops the last command from the redo stack, executes it again, and pushes it to the undo stack.
    ///
    /// # Errors
    /// Returns an error if:
    /// - There are no commands to redo
    /// - The execution fails
    pub fn redo(&mut self, state: &mut DawState) -> CommandResult<String> {
        let mut command = self
            .redo_stack
            .pop_back()
            .ok_or_else(|| CommandError::ExecutionFailed("Nothing to redo".into()))?;

        let description = command.description();

        // Re-execute the command
        command.execute(state)?;

        // Move to undo stack
        self.undo_stack.push_back(command);

        Ok(description)
    }

    /// Check if there are commands that can be undone
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if there are commands that can be redone
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get a description of the command that would be undone
    pub fn undo_description(&self) -> Option<String> {
        self.undo_stack.back().map(|cmd| cmd.description())
    }

    /// Get a description of the command that would be redone
    pub fn redo_description(&self) -> Option<String> {
        self.redo_stack.back().map(|cmd| cmd.description())
    }

    /// Clear all command history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Get the number of commands in the undo stack
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get the number of commands in the redo stack
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}

impl Default for CommandManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::channels::create_command_channel;
    use std::sync::{Arc, Mutex};

    // Mock command for testing
    struct MockCommand {
        value: i32,
        old_value: Option<i32>,
        executed: bool,
    }

    impl MockCommand {
        fn new(value: i32) -> Self {
            Self {
                value,
                old_value: None,
                executed: false,
            }
        }
    }

    impl UndoableCommand for MockCommand {
        fn execute(&mut self, _state: &mut DawState) -> CommandResult<()> {
            self.old_value = Some(0); // Mock: store old value
            self.executed = true;
            Ok(())
        }

        fn undo(&mut self, _state: &mut DawState) -> CommandResult<()> {
            if self.old_value.is_none() {
                return Err(CommandError::UndoFailed("Not executed".into()));
            }
            self.executed = false;
            Ok(())
        }

        fn description(&self) -> String {
            format!("Set value to {}", self.value)
        }
    }

    fn create_test_state() -> DawState {
        let (tx, _rx) = create_command_channel(128);
        DawState::new(Arc::new(Mutex::new(tx)))
    }

    #[test]
    fn test_execute_command() {
        let mut manager = CommandManager::new();
        let mut state = create_test_state();

        let cmd = Box::new(MockCommand::new(42));
        manager.execute(cmd, &mut state).unwrap();

        assert_eq!(manager.undo_count(), 1);
        assert_eq!(manager.redo_count(), 0);
        assert!(manager.can_undo());
        assert!(!manager.can_redo());
    }

    #[test]
    fn test_undo() {
        let mut manager = CommandManager::new();
        let mut state = create_test_state();

        let cmd = Box::new(MockCommand::new(42));
        manager.execute(cmd, &mut state).unwrap();

        let description = manager.undo(&mut state).unwrap();
        assert_eq!(description, "Set value to 42");
        assert_eq!(manager.undo_count(), 0);
        assert_eq!(manager.redo_count(), 1);
    }

    #[test]
    fn test_redo() {
        let mut manager = CommandManager::new();
        let mut state = create_test_state();

        let cmd = Box::new(MockCommand::new(42));
        manager.execute(cmd, &mut state).unwrap();
        manager.undo(&mut state).unwrap();

        let description = manager.redo(&mut state).unwrap();
        assert_eq!(description, "Set value to 42");
        assert_eq!(manager.undo_count(), 1);
        assert_eq!(manager.redo_count(), 0);
    }

    #[test]
    fn test_redo_stack_cleared_on_new_command() {
        let mut manager = CommandManager::new();
        let mut state = create_test_state();

        // Execute, undo, then execute a new command
        manager
            .execute(Box::new(MockCommand::new(1)), &mut state)
            .unwrap();
        manager.undo(&mut state).unwrap();
        manager
            .execute(Box::new(MockCommand::new(2)), &mut state)
            .unwrap();

        // Redo stack should be cleared
        assert!(!manager.can_redo());
        assert_eq!(manager.redo_count(), 0);
    }

    #[test]
    fn test_history_limit() {
        let mut manager = CommandManager::with_capacity(3);
        let mut state = create_test_state();

        // Execute 5 commands (more than limit)
        for i in 0..5 {
            manager
                .execute(Box::new(MockCommand::new(i)), &mut state)
                .unwrap();
        }

        // Should only keep the last 3
        assert_eq!(manager.undo_count(), 3);
    }

    #[test]
    fn test_undo_with_empty_stack() {
        let mut manager = CommandManager::new();
        let mut state = create_test_state();

        let result = manager.undo(&mut state);
        assert!(result.is_err());
    }

    #[test]
    fn test_redo_with_empty_stack() {
        let mut manager = CommandManager::new();
        let mut state = create_test_state();

        let result = manager.redo(&mut state);
        assert!(result.is_err());
    }
}
