// Tauri commands for MyMusic DAW
// Exposes audio engine controls to the React frontend

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Import DAW modules (from parent crate)
use mymusic_daw::audio::parameters::AtomicF32;
use mymusic_daw::messaging::channels::CommandProducer;
use mymusic_daw::plugin::{Plugin, PluginHost, PluginInstanceId};

// Import modular command modules
mod commands;
use commands::basic::*;
use commands::plugin::*;

// Event system
pub mod events;

// Window utilities
pub mod window_utils;

/// Plugin instance wrapper with unique ID
pub struct ManagedPlugin {
    pub host: PluginHost,
    pub instance_id: PluginInstanceId,
    pub gui_info: Option<PluginGuiInfo>,
}

/// Plugin GUI information
#[derive(Clone)]
pub struct PluginGuiInfo {
    pub is_visible: bool,
    pub width: u32,
    pub height: u32,
    pub can_resize: bool,
    pub api: String,
}

/// Shared state for the DAW engine
/// This is accessible from all Tauri commands
#[derive(Clone)]
pub struct DawState {
    /// Command producer to send commands to audio thread
    pub command_tx: Arc<Mutex<CommandProducer>>,

    /// Volume control (atomic for thread-safe access)
    pub volume_atomic: Arc<AtomicF32>,

    /// Loaded plugin instances (plugin_id -> instance)
    pub plugins: Arc<Mutex<HashMap<String, ManagedPlugin>>>,

    /// Next plugin ID counter
    pub next_plugin_id: Arc<Mutex<u32>>,
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
    pub fn generate_plugin_id(&self) -> String {
        let mut counter = self.next_plugin_id.lock().unwrap();
        let id = format!("plugin_{}", *counter);
        *counter += 1;
        id
    }
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
        get_engine_info,
        play_test_beep,
        // Synthesizer parameters
        set_waveform,
        set_adsr,
        set_lfo,
        set_filter,
        set_poly_mode,
        set_portamento,
        set_voice_mode,
        set_mod_routing,
        clear_mod_routing,
        // Event system
        initialize_events,
        // Plugin commands
        load_plugin_instance,
        get_plugin_parameters,
        get_plugin_parameter_value,
        set_plugin_parameter_value,
        unload_plugin_instance,
        get_loaded_plugins,
        scan_for_plugins,
        get_plugin_search_paths,
        scan_plugin_directory,
        // Plugin GUI commands
        show_plugin_gui,
        hide_plugin_gui,
        attach_plugin_gui,
        get_plugin_gui_size,
        set_plugin_gui_size,
        is_plugin_gui_visible,
        get_window_handle_for_plugin,
        // MIDI Bridge commands (bypass display server)
        add_midi_mapping,
        remove_midi_mapping,
        get_midi_mappings,
        auto_map_plugin,
        send_midi_cc,
        create_virtual_midi_port,
        test_midi_communication,
        get_default_midi_assignments,
    ])
}
