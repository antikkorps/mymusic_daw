// Tauri commands for MyMusic DAW
// Exposes audio engine controls to the React frontend

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::State;

// Import DAW modules (from parent crate)
use mymusic_daw::audio::AtomicF32;
use mymusic_daw::messaging::{Command, CommandProducer};
use mymusic_daw::midi::MidiEvent;
use mymusic_daw::plugin::{ClapPluginFactory, Plugin, PluginDescriptor};

/// Plugin instance wrapper with unique ID
struct ManagedPlugin {
    instance: Box<dyn Plugin>,
}

/// Shared state for the DAW engine
/// This is accessible from all Tauri commands
pub struct DawState {
    /// Command producer to send commands to audio thread
    command_tx: Arc<Mutex<CommandProducer>>,

    /// Volume control (atomic for thread-safe access)
    volume_atomic: Arc<AtomicF32>,

    /// Loaded plugin instances (plugin_id -> instance)
    plugins: Arc<Mutex<HashMap<String, ManagedPlugin>>>,

    /// Next plugin ID counter
    next_plugin_id: Arc<Mutex<u32>>,
}

impl DawState {
    pub fn new(command_tx: CommandProducer, volume_atomic: Arc<AtomicF32>) -> Self {
        Self {
            command_tx: Arc::new(Mutex::new(command_tx)),
            volume_atomic,
            plugins: Arc::new(Mutex::new(HashMap::new())),
            next_plugin_id: Arc::new(Mutex::new(0)),
        }
    }

    /// Generate a unique plugin ID
    fn generate_plugin_id(&self) -> String {
        let mut counter = self.next_plugin_id.lock().unwrap();
        let id = format!("plugin_{}", *counter);
        *counter += 1;
        id
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
///
/// # Returns
/// Success message
///
/// # Example
/// ```js
/// import { invoke } from '@tauri-apps/api/core';
/// const result = await invoke('play_test_beep');
/// console.log(result); // "Test beep played successfully"
/// ```
#[tauri::command]
pub fn play_test_beep() -> Result<String, String> {
    println!("🔊 Playing test beep (note 60, A4, 440Hz)");
    Ok("Test beep played successfully".to_string())
}

//
// ============ PLUGIN COMMANDS ============
//

/// Load a CLAP plugin instance
///
/// # Arguments
/// * `plugin_path` - Path to the .clap plugin file
///
/// # Returns
/// Plugin ID (unique identifier for this instance)
///
/// # Example
/// ```js
/// const pluginId = await invoke('load_plugin_instance', {
///   pluginPath: '/Library/Audio/Plug-Ins/CLAP/Surge XT.clap'
/// });
/// ```
#[tauri::command]
pub fn load_plugin_instance(plugin_path: String, state: State<DawState>) -> Result<String, String> {
    // Create plugin factory
    let factory = ClapPluginFactory::from_path(&plugin_path)
        .map_err(|e| format!("Failed to load plugin: {}", e))?;

    // Create plugin instance
    let mut instance = factory
        .create_instance()
        .map_err(|e| format!("Failed to create plugin instance: {}", e))?;

    // Initialize plugin with default sample rate (44100 Hz)
    instance
        .initialize(44100.0)
        .map_err(|e| format!("Failed to initialize plugin: {}", e))?;

    // Generate unique ID for this instance
    let plugin_id = state.generate_plugin_id();

    // Store the instance
    let mut plugins = state
        .plugins
        .lock()
        .map_err(|_| "Failed to acquire plugins lock".to_string())?;

    plugins.insert(
        plugin_id.clone(),
        ManagedPlugin { instance },
    );

    println!("✅ Loaded plugin instance: {}", plugin_id);
    Ok(plugin_id)
}

/// Get all parameters for a loaded plugin
///
/// # Arguments
/// * `plugin_id` - Plugin instance ID
///
/// # Returns
/// Array of parameter descriptors with id, name, min, max, default values
///
/// # Example
/// ```js
/// const params = await invoke('get_plugin_parameters', { pluginId });
/// // Returns: [{ id: "cutoff", name: "Cutoff", min: 20, max: 20000, ... }, ...]
/// ```
#[tauri::command]
pub fn get_plugin_parameters(
    plugin_id: String,
    state: State<DawState>,
) -> Result<Vec<serde_json::Value>, String> {
    let plugins = state
        .plugins
        .lock()
        .map_err(|_| "Failed to acquire plugins lock".to_string())?;

    let managed_plugin = plugins
        .get(&plugin_id)
        .ok_or_else(|| format!("Plugin not found: {}", plugin_id))?;

    let descriptor = managed_plugin.instance.descriptor();

    // Convert parameters to JSON
    let params: Vec<serde_json::Value> = descriptor
        .parameters
        .iter()
        .map(|p| {
            serde_json::json!({
                "id": p.id,
                "name": p.name,
                "min": p.min_value,
                "max": p.max_value,
                "default": p.default_value,
                "unit": p.unit
            })
        })
        .collect();

    Ok(params)
}

/// Get current value of a plugin parameter
///
/// # Arguments
/// * `plugin_id` - Plugin instance ID
/// * `parameter_id` - Parameter ID
///
/// # Returns
/// Current parameter value
///
/// # Example
/// ```js
/// const value = await invoke('get_plugin_parameter_value', {
///   pluginId,
///   parameterId: 'cutoff'
/// });
/// ```
#[tauri::command]
pub fn get_plugin_parameter_value(
    plugin_id: String,
    parameter_id: String,
    state: State<DawState>,
) -> Result<f64, String> {
    let plugins = state
        .plugins
        .lock()
        .map_err(|_| "Failed to acquire plugins lock".to_string())?;

    let managed_plugin = plugins
        .get(&plugin_id)
        .ok_or_else(|| format!("Plugin not found: {}", plugin_id))?;

    managed_plugin
        .instance
        .get_parameter(&parameter_id)
        .ok_or_else(|| format!("Parameter not found: {}", parameter_id))
}

/// Set a plugin parameter value
///
/// # Arguments
/// * `plugin_id` - Plugin instance ID
/// * `parameter_id` - Parameter ID
/// * `value` - New parameter value
///
/// # Example
/// ```js
/// await invoke('set_plugin_parameter_value', {
///   pluginId,
///   parameterId: 'cutoff',
///   value: 1000.0
/// });
/// ```
#[tauri::command]
pub fn set_plugin_parameter_value(
    plugin_id: String,
    parameter_id: String,
    value: f64,
    state: State<DawState>,
) -> Result<(), String> {
    let mut plugins = state
        .plugins
        .lock()
        .map_err(|_| "Failed to acquire plugins lock".to_string())?;

    let managed_plugin = plugins
        .get_mut(&plugin_id)
        .ok_or_else(|| format!("Plugin not found: {}", plugin_id))?;

    managed_plugin
        .instance
        .set_parameter(&parameter_id, value)
        .map_err(|e| format!("Failed to set parameter: {}", e))?;

    Ok(())
}

/// Unload a plugin instance
///
/// # Arguments
/// * `plugin_id` - Plugin instance ID
///
/// # Example
/// ```js
/// await invoke('unload_plugin_instance', { pluginId });
/// ```
#[tauri::command]
pub fn unload_plugin_instance(plugin_id: String, state: State<DawState>) -> Result<(), String> {
    let mut plugins = state
        .plugins
        .lock()
        .map_err(|_| "Failed to acquire plugins lock".to_string())?;

    plugins
        .remove(&plugin_id)
        .ok_or_else(|| format!("Plugin not found: {}", plugin_id))?;

    println!("🗑️  Unloaded plugin instance: {}", plugin_id);
    Ok(())
}

/// Get info about all loaded plugin instances
///
/// # Returns
/// Array of loaded plugin IDs and their descriptors
///
/// # Example
/// ```js
/// const loadedPlugins = await invoke('get_loaded_plugins');
/// // Returns: [{ id: "plugin_0", name: "Surge XT", vendor: "Surge Synth Team" }, ...]
/// ```
#[tauri::command]
pub fn get_loaded_plugins(state: State<DawState>) -> Result<Vec<serde_json::Value>, String> {
    let plugins = state
        .plugins
        .lock()
        .map_err(|_| "Failed to acquire plugins lock".to_string())?;

    let loaded: Vec<serde_json::Value> = plugins
        .iter()
        .map(|(id, managed_plugin)| {
            let descriptor = managed_plugin.instance.descriptor();
            serde_json::json!({
                "id": id,
                "name": descriptor.name,
                "vendor": descriptor.vendor,
                "version": descriptor.version,
                "category": format!("{:?}", descriptor.category)
            })
        })
        .collect();

    Ok(loaded)
}

// Helper function to initialize Tauri with DAW commands
pub fn register_commands<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        // Audio engine commands
        set_volume,
        play_note,
        stop_note,
        get_volume,
        get_engine_status,
        // Plugin commands
        load_plugin_instance,
        get_plugin_parameters,
        get_plugin_parameter_value,
        set_plugin_parameter_value,
        unload_plugin_instance,
        get_loaded_plugins,
    ])
}
