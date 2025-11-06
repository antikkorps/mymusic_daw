// CLAP FFI - C API bindings for CLAP plugins
// Based on the official CLAP specification: https://github.com/free-audio/clap

#![allow(non_camel_case_types)]
#![allow(dead_code)]

use std::os::raw::{c_char, c_void};

/// CLAP version structure
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct clap_version {
    pub major: u32,
    pub minor: u32,
    pub revision: u32,
}

impl clap_version {
    pub const fn new(major: u32, minor: u32, revision: u32) -> Self {
        Self {
            major,
            minor,
            revision,
        }
    }

    /// CLAP 1.0.0 (current stable)
    pub const CLAP_1_0_0: Self = Self::new(1, 0, 0);

    /// Check if this version is compatible with another version
    pub fn is_compatible(&self, other: &Self) -> bool {
        self.major == other.major
    }
}

/// CLAP plugin descriptor
#[repr(C)]
pub struct clap_plugin_descriptor {
    pub clap_version: clap_version,
    pub id: *const c_char,
    pub name: *const c_char,
    pub vendor: *const c_char,
    pub url: *const c_char,
    pub manual_url: *const c_char,
    pub support_url: *const c_char,
    pub version: *const c_char,
    pub description: *const c_char,
    pub features: *const *const c_char, // NULL-terminated array
}

/// CLAP plugin factory
#[repr(C)]
pub struct clap_plugin_factory {
    /// Get the number of plugins available
    pub get_plugin_count: extern "C" fn(factory: *const clap_plugin_factory) -> u32,

    /// Get plugin descriptor by index
    pub get_plugin_descriptor:
        extern "C" fn(factory: *const clap_plugin_factory, index: u32) -> *const clap_plugin_descriptor,

    /// Create a plugin instance
    pub create_plugin: extern "C" fn(
        factory: *const clap_plugin_factory,
        host: *const clap_host,
        plugin_id: *const c_char,
    ) -> *mut clap_plugin,
}

/// CLAP host interface
#[repr(C)]
pub struct clap_host {
    pub clap_version: clap_version,
    pub host_data: *mut c_void,
    pub name: *const c_char,
    pub vendor: *const c_char,
    pub url: *const c_char,
    pub version: *const c_char,

    /// Get extension from host
    pub get_extension: extern "C" fn(host: *const clap_host, extension_id: *const c_char) -> *const c_void,

    /// Request callback
    pub request_callback: extern "C" fn(host: *const clap_host),

    /// Request restart
    pub request_restart: extern "C" fn(host: *const clap_host),

    /// Request process
    pub request_process: extern "C" fn(host: *const clap_host),
}

/// CLAP plugin interface
#[repr(C)]
pub struct clap_plugin {
    pub desc: *const clap_plugin_descriptor,
    pub plugin_data: *mut c_void,

    /// Initialize the plugin
    pub init: extern "C" fn(plugin: *const clap_plugin) -> bool,

    /// Destroy the plugin instance
    pub destroy: extern "C" fn(plugin: *const clap_plugin),

    /// Activate the plugin (prepare for processing)
    pub activate: extern "C" fn(
        plugin: *const clap_plugin,
        sample_rate: f64,
        min_frames_count: u32,
        max_frames_count: u32,
    ) -> bool,

    /// Deactivate the plugin
    pub deactivate: extern "C" fn(plugin: *const clap_plugin),

    /// Start processing
    pub start_processing: extern "C" fn(plugin: *const clap_plugin) -> bool,

    /// Stop processing
    pub stop_processing: extern "C" fn(plugin: *const clap_plugin),

    /// Reset the plugin (stop all voices, clear buffers)
    pub reset: extern "C" fn(plugin: *const clap_plugin),

    /// Process audio
    pub process: extern "C" fn(plugin: *const clap_plugin, process: *const clap_process) -> clap_process_status,

    /// Get extension from plugin
    pub get_extension: extern "C" fn(plugin: *const clap_plugin, extension_id: *const c_char) -> *const c_void,

    /// Called by host on main thread
    pub on_main_thread: extern "C" fn(plugin: *const clap_plugin),
}

/// CLAP process status
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum clap_process_status {
    /// Processing succeeded
    CLAP_PROCESS_CONTINUE = 0,
    /// Processing succeeded and plugin needs more processing (tail)
    CLAP_PROCESS_CONTINUE_IF_NOT_QUIET = 1,
    /// Processing succeeded but plugin is in tail mode
    CLAP_PROCESS_TAIL = 2,
    /// Processing succeeded and plugin has finished (sleep mode)
    CLAP_PROCESS_SLEEP = 3,
    /// Processing failed
    CLAP_PROCESS_ERROR = 4,
}

/// CLAP audio buffer
#[repr(C)]
pub struct clap_audio_buffer {
    /// Number of channels (samples is interleaved if data32/data64 is not NULL)
    pub channel_count: u32,
    /// Latency from/to the audio interface in samples
    pub latency: u32,
    /// Either data32 or data64 must be set
    pub data32: *mut *mut f32,
    pub data64: *mut *mut f64,
}

/// CLAP event header
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct clap_event_header {
    pub size: u32,
    pub time: u32,
    pub space_id: u16,
    pub type_: u16,
    pub flags: u32,
}

/// CLAP event types
pub const CLAP_EVENT_NOTE_ON: u16 = 0;
pub const CLAP_EVENT_NOTE_OFF: u16 = 1;
pub const CLAP_EVENT_NOTE_CHOKE: u16 = 2;
pub const CLAP_EVENT_NOTE_END: u16 = 3;
pub const CLAP_EVENT_NOTE_EXPRESSION: u16 = 4;
pub const CLAP_EVENT_PARAM_VALUE: u16 = 5;
pub const CLAP_EVENT_PARAM_MOD: u16 = 6;
pub const CLAP_EVENT_PARAM_GESTURE_BEGIN: u16 = 7;
pub const CLAP_EVENT_PARAM_GESTURE_END: u16 = 8;
pub const CLAP_EVENT_TRANSPORT: u16 = 9;
pub const CLAP_EVENT_MIDI: u16 = 10;
pub const CLAP_EVENT_MIDI_SYSEX: u16 = 11;
pub const CLAP_EVENT_MIDI2: u16 = 12;

/// CLAP core event space
pub const CLAP_CORE_EVENT_SPACE_ID: u16 = 0;

/// CLAP note event
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct clap_event_note {
    pub header: clap_event_header,
    pub note_id: i32,
    pub port_index: i16,
    pub channel: i16,
    pub key: i16,
    pub velocity: f64,
}

/// CLAP MIDI event (raw MIDI bytes)
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct clap_event_midi {
    pub header: clap_event_header,
    pub port_index: u16,
    pub data: [u8; 3],
}

/// CLAP parameter value event
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct clap_event_param_value {
    pub header: clap_event_header,
    pub param_id: u32,
    pub cookie: *mut std::ffi::c_void,
    pub note_id: i32,
    pub port_index: i16,
    pub channel: i16,
    pub key: i16,
    pub value: f64,
}

/// CLAP input events
#[repr(C)]
pub struct clap_input_events {
    pub ctx: *mut c_void,
    pub size: extern "C" fn(list: *const clap_input_events) -> u32,
    pub get: extern "C" fn(list: *const clap_input_events, index: u32) -> *const clap_event_header,
}

/// CLAP output events
#[repr(C)]
pub struct clap_output_events {
    pub ctx: *mut c_void,
    pub try_push: extern "C" fn(list: *const clap_output_events, event: *const clap_event_header) -> bool,
}

/// CLAP process structure
#[repr(C)]
pub struct clap_process {
    /// Steady sample time at the start of the process call
    pub steady_time: i64,
    /// Number of frames to process
    pub frames_count: u32,
    /// Transport info (optional)
    pub transport: *const c_void,
    /// Audio inputs
    pub audio_inputs: *const clap_audio_buffer,
    pub audio_inputs_count: u32,
    /// Audio outputs
    pub audio_outputs: *mut clap_audio_buffer,
    pub audio_outputs_count: u32,
    /// Input events (MIDI, parameters, etc.)
    pub in_events: *const clap_input_events,
    /// Output events
    pub out_events: *const clap_output_events,
}

/// CLAP plugin entry point
#[repr(C)]
pub struct clap_plugin_entry {
    pub clap_version: clap_version,

    /// Initialize the plugin entry
    pub init: extern "C" fn(plugin_path: *const c_char) -> bool,

    /// Deinitialize the plugin entry
    pub deinit: extern "C" fn(),

    /// Get factory by ID
    pub get_factory: extern "C" fn(factory_id: *const c_char) -> *const c_void,
}

/// CLAP extension: parameters
pub const CLAP_EXT_PARAMS: &[u8] = b"clap.params\0";

/// CLAP extension: GUI
pub const CLAP_EXT_GUI: &[u8] = b"clap.gui\0";

/// CLAP extension: state
pub const CLAP_EXT_STATE: &[u8] = b"clap.state\0";

/// CLAP factory ID
pub const CLAP_PLUGIN_FACTORY_ID: &[u8] = b"clap.plugin-factory\0";

/// CLAP parameter info
#[repr(C)]
pub struct clap_param_info {
    pub id: u32,
    pub flags: u32,
    pub cookie: *mut std::ffi::c_void,
    pub name: [u8; 256],
    pub module: [u8; 1024],
    pub min_value: f64,
    pub max_value: f64,
    pub default_value: f64,
}

/// CLAP plugin params extension
#[repr(C)]
pub struct clap_plugin_params {
    /// Get parameter count
    pub count: extern "C" fn(plugin: *const clap_plugin) -> u32,

    /// Get parameter info by index
    pub get_info: extern "C" fn(
        plugin: *const clap_plugin,
        index: u32,
        info: *mut clap_param_info,
    ) -> bool,

    /// Get parameter value by ID
    pub get_value: extern "C" fn(
        plugin: *const clap_plugin,
        param_id: u32,
        value: *mut f64,
    ) -> bool,

    /// Convert value to text
    pub value_to_text: extern "C" fn(
        plugin: *const clap_plugin,
        param_id: u32,
        value: f64,
        display: *mut u8,
        size: u32,
    ) -> bool,

    /// Convert text to value
    pub text_to_value: extern "C" fn(
        plugin: *const clap_plugin,
        param_id: u32,
        display: *const u8,
        value: *mut f64,
    ) -> bool,

    /// Flush parameter changes (main thread)
    pub flush: extern "C" fn(
        plugin: *const clap_plugin,
        in_events: *const clap_input_events,
        out_events: *const clap_output_events,
    ),
}

/// Helper function to convert C string to Rust String
pub unsafe fn c_str_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }

    let c_str = std::ffi::CStr::from_ptr(ptr);
    c_str.to_str().ok().map(|s| s.to_string())
}

/// Helper function to read NULL-terminated string array
pub unsafe fn read_string_array(mut ptr: *const *const c_char) -> Vec<String> {
    let mut result = Vec::new();

    if ptr.is_null() {
        return result;
    }

    while !(*ptr).is_null() {
        if let Some(s) = c_str_to_string(*ptr) {
            result.push(s);
        }
        ptr = ptr.add(1);
    }

    result
}
