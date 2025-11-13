// Event system for streaming data from audio engine to React UI
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use tauri::{AppHandle, Emitter};

// Event types that can be streamed from audio engine to UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AudioEvent {
    /// MIDI note events (for visual feedback)
    MidiNote {
        note: u8,
        velocity: u8,
        on: bool, // true for NoteOn, false for NoteOff
        timestamp: u64, // samples since start
    },
    /// Active voices count (for performance monitoring)
    ActiveVoices {
        count: u32,
        timestamp: u64,
    },
    /// CPU usage percentage (for performance monitoring)
    CpuUsage {
        percentage: f32,
        timestamp: u64,
    },
    /// Audio level meter (for VU meters)
    AudioLevel {
        left: f32,
        right: f32,
        peak_left: f32,
        peak_right: f32,
        timestamp: u64,
    },
    /// Parameter changes (for UI synchronization)
    ParameterChanged {
        parameter: String,
        value: serde_json::Value,
        timestamp: u64,
    },
    /// Transport position updates
    TransportPosition {
        samples: u64,
        musical_time: String, // "bars:beats:ticks"
        is_playing: bool,
        tempo: f32,
        timestamp: u64,
    },
    /// Metronome events
    MetronomeTick {
        beat: u32,
        is_accent: bool,
        timestamp: u64,
    },
    /// Error notifications
    Error {
        message: String,
        severity: String, // "warning", "error", "info"
        timestamp: u64,
    },
}

/// Event emitter for sending events from audio engine to UI
pub struct AudioEventEmitter {
    app_handle: Option<AppHandle>,
    // Buffer for events when app_handle is not available (during startup)
    pending_events: Arc<Mutex<Vec<AudioEvent>>>,
}

impl AudioEventEmitter {
    pub fn new() -> Self {
        Self {
            app_handle: None,
            pending_events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Set the app handle (called after Tauri app is initialized)
    pub fn set_app_handle(&mut self, app_handle: AppHandle) {
        self.app_handle = Some(app_handle);
        
        // Emit any pending events
        if let Ok(mut pending) = self.pending_events.lock() {
            for event in pending.drain(..) {
                self.emit_event_internal(event);
            }
        }
    }

    /// Emit an event to the UI
    pub fn emit(&self, event: AudioEvent) {
        if let Some(ref app_handle) = self.app_handle {
            self.emit_event_internal(event);
        } else {
            // Store for later emission when app_handle is available
            if let Ok(mut pending) = self.pending_events.lock() {
                pending.push(event);
            }
        }
    }

    fn emit_event_internal(&self, event: AudioEvent) {
        if let Some(app_handle) = &self.app_handle {
            let event_name = match event {
                AudioEvent::MidiNote { .. } => "audio:midi-note",
                AudioEvent::ActiveVoices { .. } => "audio:active-voices",
                AudioEvent::CpuUsage { .. } => "audio:cpu-usage",
                AudioEvent::AudioLevel { .. } => "audio:level",
                AudioEvent::ParameterChanged { .. } => "audio:parameter-changed",
                AudioEvent::TransportPosition { .. } => "audio:transport-position",
                AudioEvent::MetronomeTick { .. } => "audio:metronome-tick",
                AudioEvent::Error { .. } => "audio:error",
            };

            if let Err(e) = app_handle.emit(event_name, &event) {
                eprintln!("Failed to emit audio event '{}': {}", event_name, e);
            }
        }
    }

    /// Convenience methods for specific event types
    pub fn midi_note(&self, note: u8, velocity: u8, on: bool, timestamp: u64) {
        self.emit(AudioEvent::MidiNote {
            note,
            velocity,
            on,
            timestamp,
        });
    }

    pub fn active_voices(&self, count: u32, timestamp: u64) {
        self.emit(AudioEvent::ActiveVoices {
            count,
            timestamp,
        });
    }

    pub fn cpu_usage(&self, percentage: f32, timestamp: u64) {
        self.emit(AudioEvent::CpuUsage {
            percentage,
            timestamp,
        });
    }

    pub fn audio_level(&self, left: f32, right: f32, peak_left: f32, peak_right: f32, timestamp: u64) {
        self.emit(AudioEvent::AudioLevel {
            left,
            right,
            peak_left,
            peak_right,
            timestamp,
        });
    }

    pub fn parameter_changed(&self, parameter: String, value: serde_json::Value, timestamp: u64) {
        self.emit(AudioEvent::ParameterChanged {
            parameter,
            value,
            timestamp,
        });
    }

    pub fn transport_position(&self, samples: u64, musical_time: String, is_playing: bool, tempo: f32, timestamp: u64) {
        self.emit(AudioEvent::TransportPosition {
            samples,
            musical_time,
            is_playing,
            tempo,
            timestamp,
        });
    }

    pub fn metronome_tick(&self, beat: u32, is_accent: bool, timestamp: u64) {
        self.emit(AudioEvent::MetronomeTick {
            beat,
            is_accent,
            timestamp,
        });
    }

    pub fn error(&self, message: String, severity: String, timestamp: u64) {
        self.emit(AudioEvent::Error {
            message,
            severity,
            timestamp,
        });
    }
}

impl Default for AudioEventEmitter {
    fn default() -> Self {
        Self::new()
    }
}

// Global event emitter instance
lazy_static::lazy_static! {
    pub static ref AUDIO_EVENT_EMITTER: Arc<Mutex<AudioEventEmitter>> = 
        Arc::new(Mutex::new(AudioEventEmitter::new()));
}

/// Helper function to emit events from anywhere in the codebase
pub fn emit_audio_event(event: AudioEvent) {
    if let Ok(emitter) = AUDIO_EVENT_EMITTER.lock() {
        emitter.emit(event);
    }
}

/// Convenience functions for common events
pub fn emit_midi_note(note: u8, velocity: u8, on: bool) {
    emit_audio_event(AudioEvent::MidiNote {
        note,
        velocity,
        on,
        timestamp: get_timestamp(),
    });
}

pub fn emit_active_voices(count: u32) {
    emit_audio_event(AudioEvent::ActiveVoices {
        count,
        timestamp: get_timestamp(),
    });
}

pub fn emit_cpu_usage(percentage: f32) {
    emit_audio_event(AudioEvent::CpuUsage {
        percentage,
        timestamp: get_timestamp(),
    });
}

pub fn emit_audio_level(left: f32, right: f32, peak_left: f32, peak_right: f32) {
    emit_audio_event(AudioEvent::AudioLevel {
        left,
        right,
        peak_left,
        peak_right,
        timestamp: get_timestamp(),
    });
}

pub fn emit_parameter_changed(parameter: String, value: serde_json::Value) {
    emit_audio_event(AudioEvent::ParameterChanged {
        parameter,
        value,
        timestamp: get_timestamp(),
    });
}

pub fn emit_transport_position(samples: u64, musical_time: String, is_playing: bool, tempo: f32) {
    emit_audio_event(AudioEvent::TransportPosition {
        samples,
        musical_time,
        is_playing,
        tempo,
        timestamp: get_timestamp(),
    });
}

pub fn emit_metronome_tick(beat: u32, is_accent: bool) {
    emit_audio_event(AudioEvent::MetronomeTick {
        beat,
        is_accent,
        timestamp: get_timestamp(),
    });
}

pub fn emit_error(message: String, severity: &str) {
    emit_audio_event(AudioEvent::Error {
        message,
        severity: severity.to_string(),
        timestamp: get_timestamp(),
    });
}

/// Get current timestamp in samples (approximate)
fn get_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}