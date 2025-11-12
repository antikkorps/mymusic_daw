// Tauri commands for MyMusic DAW
// These commands are callable from the React frontend

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};

// Import DAW modules (from parent crate)
use mymusic_daw::audio::{AtomicF32, AudioDeviceManager, AudioEngine, CpuMonitor};
use mymusic_daw::messaging::{create_channels, Command, CommandProducer};
use mymusic_daw::midi::{MidiConnectionManager, MidiEvent};
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

//
// ============ AUDIO ENGINE COMMANDS ============
//

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
    let clamped_volume = volume.max(0.0).min(1.0);

    // Update atomic volume
    state.volume_atomic.store(clamped_volume);

    println!("🔊 Volume set to: {:.2}", clamped_volume);
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
/// await invoke('play_note', { note: 60, velocity: 100 });
/// ```
#[tauri::command]
pub fn play_note(note: u8, velocity: u8, state: State<DawState>) -> Result<(), String> {
    let mut command_tx = state
        .command_tx
        .lock()
        .map_err(|_| "Failed to acquire command producer lock".to_string())?;

    let midi_event = MidiEvent {
        timestamp: 0,
        channel: 0,
        data: vec![0x90, note, velocity], // Note On
    };

    command_tx
        .push(Command::MidiInput(midi_event))
        .map_err(|_| "Failed to send MIDI command".to_string())?;

    println!("🎹 Note ON: {} (vel: {})", note, velocity);
    Ok(())
}

/// Stop a MIDI note
///
/// # Arguments
/// * `note` - MIDI note number (0-127)
///
/// # Example
/// ```js
/// await invoke('stop_note', { note: 60 });
/// ```
#[tauri::command]
pub fn stop_note(note: u8, state: State<DawState>) -> Result<(), String> {
    let mut command_tx = state
        .command_tx
        .lock()
        .map_err(|_| "Failed to acquire command producer lock".to_string())?;

    let midi_event = MidiEvent {
        timestamp: 0,
        channel: 0,
        data: vec![0x80, note, 0], // Note Off
    };

    command_tx
        .push(Command::MidiInput(midi_event))
        .map_err(|_| "Failed to send MIDI command".to_string())?;

    println!("🎹 Note OFF: {}", note);
    Ok(())
}

/// Get the current master volume
///
/// # Returns
/// Current volume level (0.0 to 1.0)
///
/// # Example
/// ```js
/// const volume = await invoke('get_volume');
/// console.log('Current volume:', volume);
/// ```
#[tauri::command]
pub fn get_volume(state: State<DawState>) -> Result<f32, String> {
    let volume = state.volume_atomic.load();
    Ok(volume)
}

/// Get engine status information
///
/// # Returns
/// Engine information including name, version, and status
///
/// # Example
/// ```js
/// const status = await invoke('get_engine_status');
/// console.log('Engine:', status.name, status.version);
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
                "parameterCount": descriptor.parameters.len()
            })
        })
        .collect();

    Ok(loaded)
}

//
// ============ TAURI APP INITIALIZATION ============
//

/// Initialize and run the Tauri application with DAW engine
/// This function is called from main.rs
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize the audio engine
    println!("🎵 Initializing MyMusic DAW...");

    // Create communication channels
    let (command_tx, command_rx) = create_channels();

    // Create volume atomic
    let volume_atomic = Arc::new(AtomicF32::new(0.5)); // Default 50% volume

    // Create MIDI connection manager
    let _midi_manager = MidiConnectionManager::new(command_tx.clone());

    // Create CPU monitor
    let cpu_monitor = CpuMonitor::new();

    // Create notification channel (unused in Tauri for now, but required by AudioEngine)
    let (notification_tx, _notification_rx) = ringbuf::HeapRb::new(256).split();

    // Initialize audio device manager
    let audio_device_manager = AudioDeviceManager::new();
    let available_devices = audio_device_manager.list_output_devices();

    println!("📢 Available audio devices:");
    for device in &available_devices {
        println!(
            "  {} {}",
            if device.is_default { "✓" } else { " " },
            device.name
        );
    }

    // Create audio engine
    let audio_engine = AudioEngine::new(
        command_rx,
        volume_atomic.clone(),
        cpu_monitor.clone(),
        notification_tx,
    );

    // Start audio stream
    match audio_engine.start() {
        Ok(_stream) => {
            println!("✅ Audio engine started successfully");

            // Store stream to keep it alive (Tauri will manage its lifetime)
            // In a real app, you'd want to store this in managed state
            std::mem::forget(_stream);
        }
        Err(e) => {
            eprintln!("❌ Failed to start audio engine: {}", e);
            std::process::exit(1);
        }
    }

    // Create DAW state for Tauri
    let daw_state = DawState::new(command_tx, volume_atomic);

    // Build and run Tauri application
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            println!("🚀 Tauri app initialized");
            println!("🎹 DAW is ready!");

            // Log window info
            if let Some(window) = app.get_webview_window("main") {
                println!("📱 Main window created: {:?}", window.label());
            }

            Ok(())
        })
        .manage(daw_state)
        .invoke_handler(tauri::generate_handler![
            // Audio engine commands
            set_volume,
            play_note,
            stop_note,
            get_volume,
            get_engine_status,
            // Plugin management commands
            load_plugin_instance,
            get_plugin_parameters,
            get_plugin_parameter_value,
            set_plugin_parameter_value,
            unload_plugin_instance,
            get_loaded_plugins,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
