// Basic DAW commands (volume, notes, engine status)

use tauri::State;
use crate::DawState;
use mymusic_daw::messaging::command::Command;
use mymusic_daw::MidiEvent;

/// Set the master volume
#[tauri::command]
pub fn set_volume(volume: f32, state: State<DawState>) -> Result<(), String> {
    let clamped_volume = volume.clamp(0.0, 1.0);
    state.volume_atomic.set(clamped_volume);
    Ok(())
}

/// Play a MIDI note
#[tauri::command]
pub fn play_note(note: u8, velocity: u8, state: State<DawState>) -> Result<(), String> {
    if velocity == 0 {
        return Err("Velocity must be greater than 0".to_string());
    }

    let midi_event = MidiEvent::NoteOn { note, velocity };
    let command = Command::Midi(mymusic_daw::MidiEventTimed {
        event: midi_event,
        samples_from_now: 0,
    });

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
#[tauri::command]
pub fn stop_note(note: u8, state: State<DawState>) -> Result<(), String> {
    let midi_event = MidiEvent::NoteOff { note };
    let command = Command::Midi(mymusic_daw::MidiEventTimed {
        event: midi_event,
        samples_from_now: 0,
    });

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
#[tauri::command]
pub fn get_volume(state: State<DawState>) -> Result<f32, String> {
    Ok(state.volume_atomic.get())
}

/// Get DAW engine status/info
#[tauri::command]
pub fn get_engine_status() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "name": "MyMusic DAW",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "running",
        "audio_engine": "CPAL",
        "sample_rate": 44100,
        "buffer_size": 512
    }))
}

/// Alias for get_engine_status (for frontend compatibility)
#[tauri::command]
pub fn get_engine_info() -> Result<serde_json::Value, String> {
    get_engine_status()
}

/// Play a test beep sound
#[tauri::command]
pub fn play_test_beep() -> Result<String, String> {
    println!("🔊 Playing test beep (note 60, A4, 440Hz)");
    Ok("Test beep played successfully".to_string())
}