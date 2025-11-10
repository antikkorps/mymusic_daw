// Tauri commands for MyMusic DAW
// Exposes audio engine controls to the React frontend

use std::sync::{Arc, Mutex};
use tauri::State;

// Import DAW modules (from parent crate)
use mymusic_daw::audio::AtomicF32;
use mymusic_daw::messaging::{Command, CommandProducer};
use mymusic_daw::midi::MidiEvent;

/// Shared state for the DAW engine
/// This is accessible from all Tauri commands
pub struct DawState {
    /// Command producer to send commands to audio thread
    command_tx: Arc<Mutex<CommandProducer>>,

    /// Volume control (atomic for thread-safe access)
    volume_atomic: Arc<AtomicF32>,
}

impl DawState {
    pub fn new(command_tx: CommandProducer, volume_atomic: Arc<AtomicF32>) -> Self {
        Self {
            command_tx: Arc::new(Mutex::new(command_tx)),
            volume_atomic,
        }
    }
}

/// Set the master volume
///
/// # Arguments
/// * `volume` - Volume level (0.0 to 1.0)
///
/// # Example
/// ```js
/// import { invoke } from '@tauri-apps/api/core';
/// await invoke('set_volume', { volume: 0.5 });
/// ```
#[tauri::command]
pub fn set_volume(volume: f32, state: State<DawState>) -> Result<(), String> {
    // Clamp volume to valid range
    let clamped_volume = volume.clamp(0.0, 1.0);

    // Update atomic volume (used by audio thread)
    state.volume_atomic.set(clamped_volume);

    Ok(())
}

/// Play a MIDI note
///
/// # Arguments
/// * `note` - MIDI note number (0-127)
/// * `velocity` - Note velocity (0-127)
///
/// # Example
/// ```js
/// import { invoke } from '@tauri-apps/api/core';
/// // Play middle C (note 60) with velocity 100
/// await invoke('play_note', { note: 60, velocity: 100 });
/// ```
#[tauri::command]
pub fn play_note(note: u8, velocity: u8, state: State<DawState>) -> Result<(), String> {
    // Validate note and velocity
    if velocity == 0 {
        return Err("Velocity must be greater than 0".to_string());
    }

    // Create MIDI NoteOn event
    let midi_event = MidiEvent::NoteOn { note, velocity };

    // Create command with immediate timing (samples_from_now = 0)
    let command = Command::Midi(mymusic_daw::midi::MidiEventTimed {
        event: midi_event,
        samples_from_now: 0,
    });

    // Send command to audio thread
    if let Ok(mut tx) = state.command_tx.lock() {
        use ringbuf::traits::Producer;
        tx.try_push(command)
            .map_err(|_| "Failed to send note command (buffer full)".to_string())?;
        Ok(())
    } else {
        Err("Failed to acquire command producer lock".to_string())
    }
}

/// Stop a MIDI note
///
/// # Arguments
/// * `note` - MIDI note number (0-127)
///
/// # Example
/// ```js
/// import { invoke } from '@tauri-apps/api/core';
/// // Stop middle C (note 60)
/// await invoke('stop_note', { note: 60 });
/// ```
#[tauri::command]
pub fn stop_note(note: u8, state: State<DawState>) -> Result<(), String> {
    // Create MIDI NoteOff event
    let midi_event = MidiEvent::NoteOff { note };

    // Create command with immediate timing
    let command = Command::Midi(mymusic_daw::midi::MidiEventTimed {
        event: midi_event,
        samples_from_now: 0,
    });

    // Send command to audio thread
    if let Ok(mut tx) = state.command_tx.lock() {
        use ringbuf::traits::Producer;
        tx.try_push(command)
            .map_err(|_| "Failed to send note command (buffer full)".to_string())?;
        Ok(())
    } else {
        Err("Failed to acquire command producer lock".to_string())
    }
}

/// Get current master volume
///
/// # Returns
/// Current volume level (0.0 to 1.0)
///
/// # Example
/// ```js
/// import { invoke } from '@tauri-apps/api/core';
/// const volume = await invoke('get_volume');
/// console.log('Current volume:', volume);
/// ```
#[tauri::command]
pub fn get_volume(state: State<DawState>) -> Result<f32, String> {
    Ok(state.volume_atomic.get())
}

/// Get DAW engine status/info
///
/// # Returns
/// JSON object with engine status information
///
/// # Example
/// ```js
/// import { invoke } from '@tauri-apps/api/core';
/// const status = await invoke('get_engine_status');
/// console.log('Engine status:', status);
/// ```
#[tauri::command]
pub fn get_engine_status() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "name": "MyMusic DAW",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "running"
    }))
}

// Helper function to initialize Tauri with DAW commands
pub fn register_commands<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        set_volume,
        play_note,
        stop_note,
        get_volume,
        get_engine_status,
    ])
}
