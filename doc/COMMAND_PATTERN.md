# Command Pattern - Undo/Redo System

## Overview

The Command Pattern has been implemented in Phase 2 to enable undo/redo functionality for all DAW operations. This architectural decision was made early to avoid the complexity of retrofitting undo/redo later.

## Architecture

### Key Components

1. **`UndoableCommand` trait** (`src/command/trait_def.rs`)
   - Defines the interface for all undoable commands
   - Methods: `execute()`, `undo()`, `description()`
   - Optional: `can_merge_with()` and `merge_with()` for command coalescing

2. **`CommandManager`** (`src/command/manager.rs`)
   - Manages undo/redo stacks
   - Maintains history with configurable limit (default: 100 commands)
   - Automatically clears redo stack when new commands are executed

3. **`DawState`** (`src/command/state.rs`)
   - Central mutable state of the DAW
   - Contains all parameters that can be modified by commands
   - Holds shared reference to command sender for audio thread communication

4. **Concrete Commands** (`src/command/commands.rs`)
   - `SetVolumeCommand`: Change volume with undo capability
   - `SetWaveformCommand`: Change waveform type with undo capability
   - More commands will be added in future phases (ADSR, LFO, etc.)

## Data Flow

```
User Action (UI)
    ↓
Create UndoableCommand
    ↓
CommandManager.execute()
    ├─→ Command.execute(DawState)
    │       ├─→ Store old value for undo
    │       ├─→ Update DawState
    │       └─→ Send message to audio thread via ringbuffer
    └─→ Push to undo stack

Undo (Ctrl+Z)
    ↓
CommandManager.undo()
    ├─→ Pop from undo stack
    ├─→ Command.undo(DawState)
    │       ├─→ Restore old value
    │       └─→ Send message to audio thread
    └─→ Push to redo stack

Redo (Ctrl+Y or Ctrl+Shift+Z)
    ↓
CommandManager.redo()
    ├─→ Pop from redo stack
    ├─→ Command.execute(DawState)
    └─→ Push to undo stack
```

## Thread Safety

- **UI Thread**: Executes commands and manages undo/redo stacks
- **Audio Thread**: Receives low-level messages via lock-free ringbuffer
- **Communication**: `Arc<Mutex<CommandProducer>>` allows sharing the command sender between `DawApp` and `DawState`

The mutex is only locked briefly to push messages to the ringbuffer, so there's no risk of blocking the UI thread.

## Usage

### Creating a New Command

```rust
use crate::command::{UndoableCommand, DawState};
use crate::command::trait_def::{CommandResult, CommandError};
use crate::messaging::command::Command;

pub struct SetParameterCommand {
    new_value: f32,
    old_value: Option<f32>,
}

impl SetParameterCommand {
    pub fn new(value: f32) -> Self {
        Self {
            new_value: value,
            old_value: None,
        }
    }
}

impl UndoableCommand for SetParameterCommand {
    fn execute(&mut self, state: &mut DawState) -> CommandResult<()> {
        // Store old value
        self.old_value = Some(state.parameter);

        // Update state
        state.parameter = self.new_value;

        // Send to audio thread
        if !state.send_to_audio(Command::SetParameter(self.new_value)) {
            return Err(CommandError::ExecutionFailed(
                "Ringbuffer full".into()
            ));
        }

        Ok(())
    }

    fn undo(&mut self, state: &mut DawState) -> CommandResult<()> {
        let old_value = self.old_value
            .ok_or_else(|| CommandError::UndoFailed("No old value".into()))?;

        state.parameter = old_value;
        state.send_to_audio(Command::SetParameter(old_value));

        Ok(())
    }

    fn description(&self) -> String {
        format!("Set Parameter to {:.2}", self.new_value)
    }
}
```

### Executing a Command in the UI

```rust
// In DawApp or any UI component with access to CommandManager
let cmd = Box::new(SetParameterCommand::new(0.75));
if let Err(e) = self.command_manager.execute(cmd, &mut self.daw_state) {
    eprintln!("Command failed: {}", e);
}
```

### Keyboard Shortcuts

- **Ctrl+Z** (Cmd+Z on macOS): Undo last command
- **Ctrl+Y** or **Ctrl+Shift+Z**: Redo last undone command

## Implementation Status

### ✅ Completed (Phase 2 - Initial Implementation)

- [x] `UndoableCommand` trait definition
- [x] `CommandManager` with undo/redo stacks
- [x] `DawState` with command sender integration
- [x] `SetVolumeCommand` implementation
- [x] `SetWaveformCommand` implementation
- [x] Integration into `DawApp`
- [x] Ctrl+Z / Ctrl+Y keyboard shortcuts
- [x] 13 unit tests for command pattern

### 🔜 Future Commands (Phase 2+)

- [ ] ADSR parameter commands (Attack, Decay, Sustain, Release)
- [ ] LFO parameter commands (Rate, Depth, Waveform)
- [ ] Filter parameter commands (Cutoff, Resonance)
- [ ] Note editing commands (Add, Delete, Move, Resize)
- [ ] Track/mixer commands (Pan, Mute, Solo)
- [ ] Plugin parameter commands

## Testing

Run all command pattern tests:
```bash
cargo test --lib command
```

Current test coverage:
- 13 tests for command pattern
- 68 total tests (including integration with existing code)

## Performance Considerations

1. **Memory**: Command history is limited to 100 commands by default to prevent unbounded growth
2. **CPU**: Commands are executed synchronously on the UI thread (fast operations only)
3. **Audio Thread**: Never blocked - receives messages via lock-free ringbuffer
4. **Command Merging**: Currently disabled (TODO), but infrastructure exists for slider operations

## Future Improvements

1. **Command Merging**: Implement downcasting to merge consecutive slider commands
2. **Macro Commands**: Combine multiple commands into one (e.g., "Set ADSR Envelope")
3. **Undo Groups**: Group related commands (e.g., all changes during plugin preset load)
4. **Persistence**: Save/load command history with projects
5. **Undo Visualization**: Show undo/redo history in UI

## References

- Design Pattern: Gang of Four "Command Pattern"
- Implementation inspired by: Ableton Live, FL Studio undo systems
- Thread-safe approach: Lock-free ringbuffers + minimal mutex usage
