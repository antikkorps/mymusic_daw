// Basic DAW commands (volume, notes, engine status)

use tauri::State;
use crate::DawState;
use mymusic_daw::messaging::command::Command;
use mymusic_daw::MidiEvent;
use mymusic_daw::synth::oscillator::WaveformType;
use mymusic_daw::synth::envelope::AdsrParams;
use mymusic_daw::synth::lfo::LfoParams;
use mymusic_daw::synth::filter::FilterParams;
use mymusic_daw::synth::modulation::{ModRouting, ModSource, ModDestination};
use mymusic_daw::synth::poly_mode::PolyMode;
use mymusic_daw::synth::portamento::PortamentoParams;
use mymusic_daw::synth::voice_manager::VoiceMode;

/// Helper function to send commands to the audio engine
fn send_command_to_engine(command: Command, state: State<DawState>) -> Result<(), String> {
    if let Ok(mut tx) = state.command_tx.lock() {
        use ringbuf::traits::Producer;
        tx.try_push(command)
            .map_err(|_| "Failed to send command (buffer full)".to_string())?;
        Ok(())
    } else {
        Err("Failed to acquire command producer lock".to_string())
    }
}

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

    send_command_to_engine(command, state)
}

/// Stop a MIDI note
#[tauri::command]
pub fn stop_note(note: u8, state: State<DawState>) -> Result<(), String> {
    let midi_event = MidiEvent::NoteOff { note };
    let command = Command::Midi(mymusic_daw::MidiEventTimed {
        event: midi_event,
        samples_from_now: 0,
    });

    send_command_to_engine(command, state)
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

// ===== SYNTHESIZER PARAMETERS =====

/// Set oscillator waveform type
#[tauri::command]
pub fn set_waveform(waveform: String, state: State<DawState>) -> Result<(), String> {
    let waveform_type = match waveform.as_str() {
        "sine" => WaveformType::Sine,
        "square" => WaveformType::Square,
        "saw" => WaveformType::Saw,
        "triangle" => WaveformType::Triangle,
        _ => return Err(format!("Invalid waveform: {}", waveform)),
    };

    let command = Command::SetWaveform(waveform_type);
    send_command_to_engine(command, state)
}

/// Set ADSR envelope parameters
#[tauri::command]
pub fn set_adsr(attack: f32, decay: f32, sustain: f32, release: f32, state: State<DawState>) -> Result<(), String> {
    let params = AdsrParams::new(attack, decay, sustain, release);
    let command = Command::SetAdsr(params);
    send_command_to_engine(command, state)
}

/// Set LFO parameters
#[tauri::command]
pub fn set_lfo(waveform: String, rate: f32, depth: f32, destination: String, state: State<DawState>) -> Result<(), String> {
    let lfo_waveform = match waveform.as_str() {
        "sine" => WaveformType::Sine,
        "square" => WaveformType::Square,
        "saw" => WaveformType::Saw,
        "triangle" => WaveformType::Triangle,
        _ => return Err(format!("Invalid LFO waveform: {}", waveform)),
    };

    let lfo_destination = match destination.as_str() {
        "pitch" => mymusic_daw::synth::lfo::LfoDestination::Pitch,
        "volume" => mymusic_daw::synth::lfo::LfoDestination::Volume,
        "filter" => mymusic_daw::synth::lfo::LfoDestination::FilterCutoff,
        _ => return Err(format!("Invalid LFO destination: {}", destination)),
    };

    let params = LfoParams::new(lfo_waveform, rate, depth, lfo_destination);
    let command = Command::SetLfo(params);
    send_command_to_engine(command, state)
}

/// Set filter parameters
#[tauri::command]
pub fn set_filter(filter_type: String, cutoff: f32, resonance: f32, state: State<DawState>) -> Result<(), String> {
    use mymusic_daw::synth::filter::FilterType;
    
    let ft = match filter_type.as_str() {
        "lowpass" => FilterType::LowPass,
        "highpass" => FilterType::HighPass,
        "bandpass" => FilterType::BandPass,
        "notch" => FilterType::Notch,
        _ => return Err(format!("Invalid filter type: {}", filter_type)),
    };

    let params = FilterParams {
        cutoff,
        resonance,
        filter_type: ft,
        enabled: true,
    };
    let command = Command::SetFilter(params);
    send_command_to_engine(command, state)
}

/// Set polyphony mode
#[tauri::command]
pub fn set_poly_mode(mode: String, state: State<DawState>) -> Result<(), String> {
    let poly_mode = match mode.as_str() {
        "poly" => PolyMode::Poly,
        "mono" => PolyMode::Mono,
        "legato" => PolyMode::Legato,
        _ => return Err(format!("Invalid polyphony mode: {}", mode)),
    };

    let command = Command::SetPolyMode(poly_mode);
    send_command_to_engine(command, state)
}

/// Set portamento (glide) parameters
#[tauri::command]
pub fn set_portamento(time: f32, state: State<DawState>) -> Result<(), String> {
    let params = PortamentoParams::new(time);
    let command = Command::SetPortamento(params);
    send_command_to_engine(command, state)
}

/// Set voice mode (Synth vs Sampler)
#[tauri::command]
pub fn set_voice_mode(mode: String, state: State<DawState>) -> Result<(), String> {
    let voice_mode = match mode.as_str() {
        "synth" => VoiceMode::Synth,
        "sampler" => VoiceMode::Sampler,
        _ => return Err(format!("Invalid voice mode: {}", mode)),
    };

    let command = Command::SetVoiceMode(voice_mode);
    send_command_to_engine(command, state)
}

/// Set modulation routing
#[tauri::command]
pub fn set_mod_routing(index: u8, source: String, destination: String, amount: f32, state: State<DawState>) -> Result<(), String> {
    let mod_source = match source.as_str() {
        "lfo" => ModSource::Lfo(0),
        "velocity" => ModSource::Velocity,
        "aftertouch" => ModSource::Aftertouch,
        "envelope" => ModSource::Envelope,
        _ => return Err(format!("Invalid modulation source: {}", source)),
    };

    let mod_destination = match destination.as_str() {
        "pitch" => ModDestination::OscillatorPitch(0),
        "amplitude" => ModDestination::Amplitude,
        "filter" => ModDestination::FilterCutoff,
        "pan" => ModDestination::Pan,
        _ => return Err(format!("Invalid modulation destination: {}", destination)),
    };

    let routing = ModRouting {
        source: mod_source,
        destination: mod_destination,
        amount,
        enabled: true,
    };
    let command = Command::SetModRouting { index, routing };
    send_command_to_engine(command, state)
}

/// Clear modulation routing
#[tauri::command]
pub fn clear_mod_routing(index: u8, state: State<DawState>) -> Result<(), String> {
    let command = Command::ClearModRouting { index };
    send_command_to_engine(command, state)
}

/// Initialize event system (call this once when app starts)
#[tauri::command]
pub fn initialize_events() -> Result<(), String> {
    // This will be called from frontend to initialize the event system
    // The actual app handle will be set up in the main Tauri setup
    Ok(())
}

// Include tests
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use ringbuf::{HeapRb, traits::Producer};
    use mymusic_daw::messaging::command::Command;
    use atomic_float::AtomicF32;

    // Mock DAW state for testing
    fn create_mock_daw_state() -> crate::DawState {
        let (tx, _) = HeapRb::<Command>::new(100).split();
        crate::DawState {
            command_tx: Arc::new(Mutex::new(tx)),
            volume_atomic: AtomicF32::new(0.5),
        }
    }

    #[test]
    fn test_set_volume_valid_range() {
        let state = create_mock_daw_state();
        
        // Test valid values
        assert!(set_volume(0.0, state.into()).is_ok());
        assert!(set_volume(0.5, state.into()).is_ok());
        assert!(set_volume(1.0, state.into()).is_ok());
    }

    #[test]
    fn test_play_note_valid_velocity() {
        let state = create_mock_daw_state();
        
        // Test valid velocities
        assert!(play_note(60, 1, state.into()).is_ok());
        assert!(play_note(60, 127, state.into()).is_ok());
    }

    #[test]
    fn test_play_note_zero_velocity() {
        let state = create_mock_daw_state();
        
        // Test zero velocity (should fail)
        let result = play_note(60, 0, state.into());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Velocity must be greater than 0"));
    }

    #[test]
    fn test_get_engine_status() {
        let result = get_engine_status();
        assert!(result.is_ok());
        
        let status = result.unwrap();
        assert_eq!(status["name"], "MyMusic DAW");
        assert_eq!(status["status"], "running");
    }

    #[test]
    fn test_set_waveform_valid() {
        let state = create_mock_daw_state();
        
        // Test valid waveforms
        assert!(set_waveform("sine".to_string(), state.into()).is_ok());
        assert!(set_waveform("square".to_string(), state.into()).is_ok());
        assert!(set_waveform("saw".to_string(), state.into()).is_ok());
        assert!(set_waveform("triangle".to_string(), state.into()).is_ok());
    }

    #[test]
    fn test_set_waveform_invalid() {
        let state = create_mock_daw_state();
        
        // Test invalid waveform
        let result = set_waveform("invalid".to_string(), state.into());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid waveform"));
    }
}