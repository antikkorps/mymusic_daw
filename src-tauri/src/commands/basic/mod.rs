// Tests for Tauri bridge functionality

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use ringbuf::{HeapRb, traits::Producer};
    use mymusic_daw::messaging::command::Command;
    use atomic_float::AtomicF32;

    // Mock DAW state for testing
    fn create_mock_daw_state() -> DawState {
        let (tx, _) = HeapRb::<Command>::new(100).split();
        DawState {
            command_tx: Arc::new(Mutex::new(tx)),
            volume_atomic: AtomicF32::new(0.5),
        }
    }

    #[test]
    fn test_send_command_to_engine_success() {
        let state = create_mock_daw_state();
        let command = Command::NoteOn(60, 127);
        
        let result = send_command_to_engine(command, state.into());
        assert!(result.is_ok());
    }

    #[test]
    fn test_send_command_to_engine_buffer_full() {
        let state = create_mock_daw_state();
        
        // Fill the buffer to capacity
        for _ in 0..100 {
            let command = Command::NoteOn(60, 127);
            let _ = send_command_to_engine(command, state.into());
        }
        
        // Next command should fail
        let command = Command::NoteOn(61, 127);
        let result = send_command_to_engine(command, state.into());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("buffer full"));
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
    fn test_set_volume_clamping() {
        let state = create_mock_daw_state();
        
        // Test clamping
        assert!(set_volume(-0.5, state.into()).is_ok());
        assert_eq!(state.volume_atomic.get(), 0.0);
        
        assert!(set_volume(1.5, state.into()).is_ok());
        assert_eq!(state.volume_atomic.get(), 1.0);
    }

    #[test]
    fn test_get_volume() {
        let state = create_mock_daw_state();
        state.volume_atomic.set(0.75);
        
        let result = get_volume(state.into());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0.75);
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
    fn test_stop_note() {
        let state = create_mock_daw_state();
        
        // Test stop note (should always succeed)
        assert!(stop_note(60, state.into()).is_ok());
    }

    #[test]
    fn test_get_engine_status() {
        let result = get_engine_status();
        assert!(result.is_ok());
        
        let status = result.unwrap();
        assert_eq!(status["name"], "MyMusic DAW");
        assert_eq!(status["status"], "running");
        assert_eq!(status["audio_engine"], "CPAL");
        assert_eq!(status["sample_rate"], 44100);
        assert_eq!(status["buffer_size"], 512);
    }

    #[test]
    fn test_get_engine_info_alias() {
        let result = get_engine_info();
        assert!(result.is_ok());
        
        // Should return same as get_engine_status
        let info = result.unwrap();
        assert_eq!(info["name"], "MyMusic DAW");
    }

    #[test]
    fn test_play_test_beep() {
        let result = play_test_beep();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Test beep played successfully");
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

    #[test]
    fn test_set_adsr() {
        let state = create_mock_daw_state();
        
        // Test valid ADSR parameters
        assert!(set_adsr(0.1, 0.2, 0.7, 0.3, state.into()).is_ok());
        assert!(set_adsr(0.0, 0.0, 0.0, 0.0, state.into()).is_ok());
        assert!(set_adsr(10.0, 10.0, 1.0, 10.0, state.into()).is_ok());
    }

    #[test]
    fn test_set_lfo_valid() {
        let state = create_mock_daw_state();
        
        // Test valid LFO parameters
        assert!(set_lfo(
            "sine".to_string(), 
            1.0, 
            0.5, 
            "pitch".to_string(), 
            state.into()
        ).is_ok());
        
        assert!(set_lfo(
            "square".to_string(), 
            2.0, 
            0.8, 
            "volume".to_string(), 
            state.into()
        ).is_ok());
    }

    #[test]
    fn test_set_lfo_invalid_waveform() {
        let state = create_mock_daw_state();
        
        let result = set_lfo(
            "invalid".to_string(), 
            1.0, 
            0.5, 
            "pitch".to_string(), 
            state.into()
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid LFO waveform"));
    }

    #[test]
    fn test_set_lfo_invalid_destination() {
        let state = create_mock_daw_state();
        
        let result = set_lfo(
            "sine".to_string(), 
            1.0, 
            0.5, 
            "invalid".to_string(), 
            state.into()
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid LFO destination"));
    }

    #[test]
    fn test_set_filter_valid() {
        let state = create_mock_daw_state();
        
        // Test valid filter types
        assert!(set_filter("lowpass".to_string(), 1000.0, 0.5, state.into()).is_ok());
        assert!(set_filter("highpass".to_string(), 2000.0, 0.7, state.into()).is_ok());
        assert!(set_filter("bandpass".to_string(), 1500.0, 0.3, state.into()).is_ok());
        assert!(set_filter("notch".to_string(), 800.0, 0.9, state.into()).is_ok());
    }

    #[test]
    fn test_set_filter_invalid_type() {
        let state = create_mock_daw_state();
        
        let result = set_filter("invalid".to_string(), 1000.0, 0.5, state.into());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid filter type"));
    }

    #[test]
    fn test_set_poly_mode_valid() {
        let state = create_mock_daw_state();
        
        // Test valid polyphony modes
        assert!(set_poly_mode("poly".to_string(), state.into()).is_ok());
        assert!(set_poly_mode("mono".to_string(), state.into()).is_ok());
        assert!(set_poly_mode("legato".to_string(), state.into()).is_ok());
    }

    #[test]
    fn test_set_poly_mode_invalid() {
        let state = create_mock_daw_state();
        
        let result = set_poly_mode("invalid".to_string(), state.into());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid polyphony mode"));
    }

    #[test]
    fn test_set_portamento() {
        let state = create_mock_daw_state();
        
        // Test valid portamento times
        assert!(set_portamento(0.0, state.into()).is_ok());
        assert!(set_portamento(0.1, state.into()).is_ok());
        assert!(set_portamento(1.0, state.into()).is_ok());
        assert!(set_portamento(5.0, state.into()).is_ok());
    }

    #[test]
    fn test_set_voice_mode_valid() {
        let state = create_mock_daw_state();
        
        // Test valid voice modes
        assert!(set_voice_mode("synth".to_string(), state.into()).is_ok());
        assert!(set_voice_mode("sampler".to_string(), state.into()).is_ok());
    }

    #[test]
    fn test_set_voice_mode_invalid() {
        let state = create_mock_daw_state();
        
        let result = set_voice_mode("invalid".to_string(), state.into());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid voice mode"));
    }

    #[test]
    fn test_set_mod_routing_valid() {
        let state = create_mock_daw_state();
        
        // Test valid modulation routing
        assert!(set_mod_routing(
            0,
            "lfo".to_string(),
            "pitch".to_string(),
            0.5,
            state.into()
        ).is_ok());
        
        assert!(set_mod_routing(
            1,
            "velocity".to_string(),
            "amplitude".to_string(),
            0.8,
            state.into()
        ).is_ok());
    }

    #[test]
    fn test_set_mod_routing_invalid_source() {
        let state = create_mock_daw_state();
        
        let result = set_mod_routing(
            0,
            "invalid".to_string(),
            "pitch".to_string(),
            0.5,
            state.into()
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid modulation source"));
    }

    #[test]
    fn test_set_mod_routing_invalid_destination() {
        let state = create_mock_daw_state();
        
        let result = set_mod_routing(
            0,
            "lfo".to_string(),
            "invalid".to_string(),
            0.5,
            state.into()
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid modulation destination"));
    }

    #[test]
    fn test_clear_mod_routing() {
        let state = create_mock_daw_state();
        
        // Test clearing modulation routing
        assert!(clear_mod_routing(0, state.into()).is_ok());
        assert!(clear_mod_routing(5, state.into()).is_ok());
    }

    #[test]
    fn test_initialize_events() {
        let result = initialize_events();
        assert!(result.is_ok());
    }

    // Integration tests for command sequences
    #[test]
    fn test_note_sequence() {
        let state = create_mock_daw_state();
        
        // Test playing and stopping a note
        assert!(play_note(60, 100, state.clone().into()).is_ok());
        assert!(stop_note(60, state.into()).is_ok());
    }

    #[test]
    fn test_parameter_change_sequence() {
        let state = create_mock_daw_state();
        
        // Test changing multiple parameters in sequence
        assert!(set_volume(0.7, state.clone().into()).is_ok());
        assert!(set_waveform("saw".to_string(), state.clone().into()).is_ok());
        assert!(set_adsr(0.2, 0.3, 0.6, 0.4, state.clone().into()).is_ok());
        assert!(set_filter("lowpass".to_string(), 1200.0, 0.6, state.into()).is_ok());
    }

    // Edge case tests
    #[test]
    fn test_extreme_parameter_values() {
        let state = create_mock_daw_state();
        
        // Test extreme but valid values
        assert!(set_volume(f32::MIN, state.clone().into()).is_ok()); // Should clamp to 0.0
        assert!(set_volume(f32::MAX, state.clone().into()).is_ok()); // Should clamp to 1.0
        assert!(set_adsr(f32::MAX, f32::MAX, f32::MAX, f32::MAX, state.into()).is_ok());
    }

    #[test]
    fn test_boundary_midi_values() {
        let state = create_mock_daw_state();
        
        // Test MIDI boundary values
        assert!(play_note(0, 1, state.clone().into()).is_ok()); // Lowest note
        assert!(play_note(127, 127, state.into()).is_ok()); // Highest note, max velocity
    }
}