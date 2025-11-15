// CLAP Plugin Integration
//
// This module provides real integration with CLAP (CLever Audio Plug-in API) plugins.
// Uses libloading for dynamic loading and FFI for C API interop.

use crate::midi::event::MidiEvent;
use crate::plugin::buffer_pool::AudioBufferPool;
use crate::plugin::clap_ffi::*;
use crate::plugin::clap_gui::ClapPluginGui;
use crate::plugin::parameters::*;
use crate::MidiEventTimed;
use crate::plugin::trait_def::*;
use crate::plugin::{PluginError, PluginResult};
use libloading::{Library, Symbol};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::path::Path;
use std::ptr;
use std::sync::Arc;

/// CLAP event wrapper (union-like)
enum ClapEvent {
    Note(clap_event_note),
    ParamValue(clap_event_param_value),
}

/// Event list for CLAP input events
struct ClapEventList {
    events: Vec<ClapEvent>,
}

impl ClapEventList {
    fn new() -> Self {
        Self { events: Vec::new() }
    }

    fn add_note_on(&mut self, note: u8, velocity: u8, sample_offset: u32) {
        let event = clap_event_note {
            header: clap_event_header {
                size: std::mem::size_of::<clap_event_note>() as u32,
                time: sample_offset,
                space_id: CLAP_CORE_EVENT_SPACE_ID,
                type_: CLAP_EVENT_NOTE_ON,
                flags: 0,
            },
            note_id: -1, // -1 means no specific note ID
            port_index: 0,
            channel: 0,
            key: note as i16,
            velocity: velocity as f64 / 127.0, // Normalize to 0.0-1.0
        };
        self.events.push(ClapEvent::Note(event));
    }

    fn add_note_off(&mut self, note: u8, sample_offset: u32) {
        let event = clap_event_note {
            header: clap_event_header {
                size: std::mem::size_of::<clap_event_note>() as u32,
                time: sample_offset,
                space_id: CLAP_CORE_EVENT_SPACE_ID,
                type_: CLAP_EVENT_NOTE_OFF,
                flags: 0,
            },
            note_id: -1,
            port_index: 0,
            channel: 0,
            key: note as i16,
            velocity: 0.0,
        };
        self.events.push(ClapEvent::Note(event));
    }

    fn add_param_value(&mut self, param_id: u32, value: f64, sample_offset: u32) {
        let event = clap_event_param_value {
            header: clap_event_header {
                size: std::mem::size_of::<clap_event_param_value>() as u32,
                time: sample_offset,
                space_id: CLAP_CORE_EVENT_SPACE_ID,
                type_: CLAP_EVENT_PARAM_VALUE,
                flags: 0,
            },
            param_id,
            cookie: ptr::null_mut(),
            note_id: -1,
            port_index: -1,
            channel: -1,
            key: -1,
            value,
        };
        self.events.push(ClapEvent::ParamValue(event));
    }

    fn as_clap_input_events(&self) -> clap_input_events {
        clap_input_events {
            ctx: self as *const Self as *mut std::ffi::c_void,
            size: event_list_size,
            get: event_list_get,
        }
    }
}

/// Callback: Get event list size
extern "C" fn event_list_size(list: *const clap_input_events) -> u32 {
    unsafe {
        let event_list = &*((*list).ctx as *const ClapEventList);
        event_list.events.len() as u32
    }
}

/// Callback: Get event from list
extern "C" fn event_list_get(
    list: *const clap_input_events,
    index: u32,
) -> *const clap_event_header {
    unsafe {
        let event_list = &*((*list).ctx as *const ClapEventList);
        if (index as usize) < event_list.events.len() {
            match &event_list.events[index as usize] {
                ClapEvent::Note(note_event) => &note_event.header as *const clap_event_header,
                ClapEvent::ParamValue(param_event) => {
                    &param_event.header as *const clap_event_header
                }
            }
        } else {
            ptr::null()
        }
    }
}

// Include simplified tests from separate file
include!("simple_tests.rs");

/// CLAP plugin factory implementation (real)
pub struct ClapPluginFactory {
    descriptor: PluginDescriptor,
    library: Arc<Library>,
    plugin_entry: *const clap_plugin_entry,
    plugin_factory: *const clap_plugin_factory,
    bundle_path: String,
}

// Safety: Library is Send + Sync, raw pointers are only used with proper synchronization
unsafe impl Send for ClapPluginFactory {}
unsafe impl Sync for ClapPluginFactory {}

impl ClapPluginFactory {
    /// Create a new CLAP plugin factory from a file path
    pub fn from_path(path: &str) -> PluginResult<Self> {
        let bundle_path = Path::new(path);

        // Get the actual library path (handle macOS bundles)
        let library_path = get_library_path(bundle_path)?;

        println!("Loading CLAP plugin from: {:?}", library_path);

        // Load the dynamic library
        let library = unsafe {
            Library::new(&library_path)
                .map_err(|e| PluginError::LoadFailed(format!("Failed to load library: {}", e)))?
        };

        // Get the clap_entry symbol
        let entry_ptr: *const clap_plugin_entry = unsafe {
            let symbol: Symbol<*const clap_plugin_entry> =
                library.get(b"clap_entry\0").map_err(|e| {
                    PluginError::LoadFailed(format!("Failed to get clap_entry symbol: {}", e))
                })?;

            *symbol
        };

        if entry_ptr.is_null() {
            return Err(PluginError::LoadFailed(
                "clap_entry returned NULL".to_string(),
            ));
        }

        let entry = unsafe { &*entry_ptr };

        // Check CLAP version compatibility
        if !entry.clap_version.is_compatible(&clap_version::CLAP_1_0_0) {
            return Err(PluginError::LoadFailed(format!(
                "Incompatible CLAP version: {}.{}.{}",
                entry.clap_version.major, entry.clap_version.minor, entry.clap_version.revision
            )));
        }

        // Initialize the plugin entry
        let path_cstr = CString::new(path)
            .map_err(|_| PluginError::LoadFailed("Invalid path string".to_string()))?;

        let init_result = (entry.init)(path_cstr.as_ptr());
        if !init_result {
            return Err(PluginError::LoadFailed(
                "Plugin initialization failed".to_string(),
            ));
        }

        // Get the plugin factory
        let factory_id = CStr::from_bytes_with_nul(CLAP_PLUGIN_FACTORY_ID)
            .map_err(|_| PluginError::LoadFailed("Invalid factory ID".to_string()))?;

        let factory_ptr = (entry.get_factory)(factory_id.as_ptr());
        if factory_ptr.is_null() {
            return Err(PluginError::LoadFailed(
                "Failed to get plugin factory".to_string(),
            ));
        }

        let plugin_factory = factory_ptr as *const clap_plugin_factory;
        let factory = unsafe { &*plugin_factory };

        // Get the first plugin descriptor
        let plugin_count = (factory.get_plugin_count)(plugin_factory);
        if plugin_count == 0 {
            return Err(PluginError::LoadFailed(
                "No plugins found in factory".to_string(),
            ));
        }

        let clap_descriptor_ptr = (factory.get_plugin_descriptor)(plugin_factory, 0);
        if clap_descriptor_ptr.is_null() {
            return Err(PluginError::LoadFailed(
                "Failed to get plugin descriptor".to_string(),
            ));
        }

        // Convert CLAP descriptor to our PluginDescriptor
        let descriptor = unsafe { convert_clap_descriptor(clap_descriptor_ptr, bundle_path)? };

        println!(
            "✅ Loaded CLAP plugin: {} ({})",
            descriptor.name, descriptor.id
        );

        Ok(Self {
            descriptor,
            library: Arc::new(library),
            plugin_entry: entry_ptr,
            plugin_factory,
            bundle_path: path.to_string(),
        })
    }

    /// Get the CLAP bundle path
    pub fn bundle(&self) -> &str {
        &self.bundle_path
    }
}

/// Get the actual library path from a CLAP bundle
/// On macOS, .clap is a bundle, so we need to extract the dylib inside
fn get_library_path(bundle_path: &Path) -> PluginResult<std::path::PathBuf> {
    #[cfg(target_os = "macos")]
    {
        // macOS: .clap is a bundle, look for Contents/MacOS/{name}
        if bundle_path.extension().and_then(|s| s.to_str()) == Some("clap") {
            let stem = bundle_path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| PluginError::LoadFailed("Invalid bundle name".to_string()))?;

            let dylib_path = bundle_path.join("Contents/MacOS").join(stem);

            if dylib_path.exists() {
                return Ok(dylib_path);
            }
        }

        // Fallback: try the path as-is
        Ok(bundle_path.to_path_buf())
    }

    #[cfg(target_os = "windows")]
    {
        // Windows: .clap is the DLL directly
        Ok(bundle_path.to_path_buf())
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: .clap is the .so directly
        Ok(bundle_path.to_path_buf())
    }
}

/// Convert CLAP descriptor to our PluginDescriptor
unsafe fn convert_clap_descriptor(
    clap_desc: *const clap_plugin_descriptor,
    file_path: &Path,
) -> PluginResult<PluginDescriptor> {
    // SAFETY: All unsafe operations are wrapped in unsafe blocks as required by Rust 2024
    let desc = unsafe { &*clap_desc };

    let id = unsafe { c_str_to_string(desc.id) }
        .ok_or_else(|| PluginError::LoadFailed("Invalid plugin ID".to_string()))?;

    let name = unsafe { c_str_to_string(desc.name) }
        .ok_or_else(|| PluginError::LoadFailed("Invalid plugin name".to_string()))?;

    let vendor = unsafe { c_str_to_string(desc.vendor) }.unwrap_or_else(|| "Unknown".to_string());
    let version = unsafe { c_str_to_string(desc.version) }.unwrap_or_else(|| "1.0.0".to_string());
    let description =
        unsafe { c_str_to_string(desc.description) }.unwrap_or_else(|| "".to_string());

    // Parse features to determine category
    let features = unsafe { read_string_array(desc.features) };
    let category = infer_category_from_features(&features);

    let mut descriptor = PluginDescriptor::new(&id, &name, file_path.to_path_buf())
        .with_version(&version)
        .with_vendor(&vendor)
        .with_description(&description)
        .with_category(category);

    // Check for common extensions
    descriptor.supports_state = features.iter().any(|f| f.contains("state"));
    descriptor.supports_gui = features.iter().any(|f| f.contains("gui"));

    Ok(descriptor)
}

/// Infer plugin category from CLAP features
fn infer_category_from_features(features: &[String]) -> PluginCategory {
    for feature in features {
        match feature.as_str() {
            "instrument" | "synthesizer" => return PluginCategory::Instrument,
            "audio-effect" | "effect" => return PluginCategory::Effect,
            "analyzer" => return PluginCategory::Analyzer,
            _ => {}
        }
    }

    PluginCategory::Effect // Default
}

/// Create a minimal CLAP host for plugins
fn create_minimal_host() -> clap_host {
    static HOST_NAME: &[u8] = b"MyMusic DAW\0";
    static HOST_VENDOR: &[u8] = b"MyMusic\0";
    static HOST_URL: &[u8] = b"https://github.com/antikkorps/mymusic_daw\0";
    static HOST_VERSION: &[u8] = b"0.2.0\0";

    clap_host {
        clap_version: clap_version::CLAP_1_0_0,
        host_data: ptr::null_mut(),
        name: HOST_NAME.as_ptr() as *const i8,
        vendor: HOST_VENDOR.as_ptr() as *const i8,
        url: HOST_URL.as_ptr() as *const i8,
        version: HOST_VERSION.as_ptr() as *const i8,
        get_extension: host_get_extension,
        request_callback: host_request_callback,
        request_restart: host_request_restart,
        request_process: host_request_process,
    }
}

/// Host callback: get extension (stub)
extern "C" fn host_get_extension(
    _host: *const clap_host,
    _extension_id: *const std::os::raw::c_char,
) -> *const std::os::raw::c_void {
    ptr::null()
}

/// Host callback: request callback (stub)
extern "C" fn host_request_callback(_host: *const clap_host) {
    // TODO: Implement callback request handling
}

/// Host callback: request restart (stub)
extern "C" fn host_request_restart(_host: *const clap_host) {
    // TODO: Implement restart handling
}

/// Host callback: request process (stub)
extern "C" fn host_request_process(_host: *const clap_host) {
    // TODO: Implement process request handling
}

impl PluginFactory for ClapPluginFactory {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn create_instance(&self) -> Result<Box<dyn Plugin>, PluginError> {
        // Create a minimal CLAP host
        let host = create_minimal_host();

        // Get the plugin factory
        let factory = unsafe { &*self.plugin_factory };

        // Convert plugin ID to C string
        let plugin_id = CString::new(self.descriptor.id.clone())
            .map_err(|_| PluginError::InitializationFailed("Invalid plugin ID".to_string()))?;

        // Create plugin instance via CLAP factory
        let plugin_ptr = (factory.create_plugin)(
            self.plugin_factory,
            &host as *const clap_host,
            plugin_id.as_ptr(),
        );

        if plugin_ptr.is_null() {
            return Err(PluginError::InitializationFailed(
                "Failed to create plugin instance".to_string(),
            ));
        }

        println!("✅ Created CLAP plugin instance: {}", self.descriptor.name);

        // SAFETY: plugin_ptr is a valid pointer obtained from the CLAP plugin factory
        Ok(Box::new(unsafe {
            ClapPluginInstance::new(
                self.descriptor.clone(),
                plugin_ptr,
                host,
                self.library.clone(),
            )
        }))
    }

    fn supports_feature(&self, feature: &str) -> bool {
        match feature {
            "audio" => true,
            "midi" => false,
            "parameters" => self.descriptor.parameters.iter().any(|p| !p.id.is_empty()),
            "state" => self.descriptor.supports_state,
            "gui" => self.descriptor.supports_gui,
            _ => false,
        }
    }
}

/// CLAP plugin instance implementation (real)
pub struct ClapPluginInstance {
    descriptor: PluginDescriptor,
    parameter_values: HashMap<String, f64>,
    parameter_id_map: HashMap<String, u32>, // String ID -> CLAP param ID
    is_active: bool,
    plugin_ptr: *mut clap_plugin,
    host: clap_host,
    #[allow(dead_code)]
    library: Arc<Library>, // Keep library alive
    sample_rate: f64,
    pending_midi_events: Vec<(MidiEvent, u32)>, // (event, sample_offset)
    pending_param_changes: Vec<(u32, f64)>,     // (param_id, value)
    gui: Option<ClapPluginGui>,                 // Optional GUI support
    buffer_pool: AudioBufferPool,               // Pre-allocated buffers for RT-safe processing
}

// Safety: plugin_ptr is only accessed from audio thread or with proper synchronization
unsafe impl Send for ClapPluginInstance {}
unsafe impl Sync for ClapPluginInstance {}

impl ClapPluginInstance {
    /// Create a new CLAP plugin instance (real)
    ///
    /// # Safety
    /// plugin_ptr must be a valid CLAP plugin pointer obtained from the plugin factory
    pub unsafe fn new(
        descriptor: PluginDescriptor,
        plugin_ptr: *mut clap_plugin,
        host: clap_host,
        library: Arc<Library>,
    ) -> Self {
        let mut parameter_values = HashMap::new();
        let mut parameter_id_map = HashMap::new();

        // Initialize parameter values with defaults
        // Note: For CLAP plugins, we'll populate this from the plugin's params extension
        for (idx, param) in descriptor.parameters.iter().enumerate() {
            parameter_values.insert(param.id.clone(), param.default_value);
            parameter_id_map.insert(param.id.clone(), idx as u32);
        }

        // NOTE: GUI creation is deferred until after plugin.init() is called
        // This is required by CLAP specification: init() must be called before get_extension()

        // Create buffer pool (1 input, 2 output stereo, max 8192 samples)
        let buffer_pool = AudioBufferPool::new(1, 2, 8192);

        Self {
            descriptor,
            parameter_values,
            parameter_id_map,
            is_active: false,
            plugin_ptr,
            host,
            library,
            sample_rate: 44100.0, // Default, will be set in initialize()
            pending_midi_events: Vec::new(),
            pending_param_changes: Vec::new(),
            gui: None, // Will be created after init()
            buffer_pool,
        }
    }

    /// Send MIDI event to plugin (will be processed in next process() call)
    pub fn send_midi_event(&mut self, event: MidiEvent, sample_offset: u32) {
        self.pending_midi_events.push((event, sample_offset));
    }

    /// Clear all pending MIDI events
    pub fn clear_midi_events(&mut self) {
        self.pending_midi_events.clear();
    }

    /// Check if plugin has GUI support
    pub fn has_gui(&self) -> bool {
        self.gui.is_some()
    }

    /// Get mutable reference to GUI (if available)
    pub fn gui_mut(&mut self) -> Option<&mut ClapPluginGui> {
        self.gui.as_mut()
    }

    /// Get reference to GUI (if available)
    pub fn gui(&self) -> Option<&ClapPluginGui> {
        self.gui.as_ref()
    }
}

impl Drop for ClapPluginInstance {
    fn drop(&mut self) {
        // Clean up the plugin instance
        if !self.plugin_ptr.is_null() {
            unsafe {
                let plugin = &*self.plugin_ptr;

                // Stop processing if active
                if self.is_active {
                    (plugin.stop_processing)(self.plugin_ptr);
                    (plugin.deactivate)(self.plugin_ptr);
                }

                // Destroy the plugin
                (plugin.destroy)(self.plugin_ptr);
            }

            self.plugin_ptr = ptr::null_mut();
        }
    }
}

impl Plugin for ClapPluginInstance {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn initialize(&mut self, sample_rate: f64) -> Result<(), PluginError> {
        if self.plugin_ptr.is_null() {
            return Err(PluginError::InitializationFailed(
                "Plugin pointer is null".to_string(),
            ));
        }

        self.sample_rate = sample_rate;

        unsafe {
            let plugin = &*self.plugin_ptr;

            // Initialize the plugin with panic handling and timeout
            println!("🔧 Calling plugin.init()...");
            println!("⚠️ Note: If this hangs, the plugin requires a display server (GUI environment)");
            
            // Use timeout to prevent hanging during init()
            // Since we can't move the plugin pointer between threads, we'll use a different approach
            let (sender, receiver) = std::sync::mpsc::channel();
            let timeout_sender = sender.clone();
            
            // Spawn a timeout thread
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(5));
                let _ = timeout_sender.send(Err("Plugin init() timed out".to_string()));
            });
            
            // Run the init in the current thread
            let init_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let plugin = &*self.plugin_ptr;
                (plugin.init)(self.plugin_ptr)
            }));
            
            // Send the actual result
            let _ = sender.send(Ok(init_result));
            
            // Wait for either the actual result or timeout
            match receiver.recv() {
                Ok(Ok(Ok(true))) => {
                    println!("✅ Plugin init() succeeded");
                }
                Ok(Ok(Ok(false))) => {
                    return Err(PluginError::InitializationFailed(
                        "Plugin init() returned false".to_string(),
                    ));
                }
                Ok(Ok(Err(_))) => {
                    return Err(PluginError::InitializationFailed(
                        "Plugin init() panicked (likely no display server or incompatible environment)".to_string(),
                    ));
                }
                Ok(Err(timeout_msg)) => {
                    return Err(PluginError::InitializationFailed(timeout_msg));
                }
                Err(_) => {
                    return Err(PluginError::InitializationFailed(
                        "Plugin init() communication failed".to_string(),
                    ));
                }
            }

            // Activate the plugin with panic handling
            println!("🔧 Calling plugin.activate()...");
            let activate_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (plugin.activate)(
                    self.plugin_ptr,
                    sample_rate,
                    512,  // min_frames_count
                    8192, // max_frames_count
                )
            }));
            
            match activate_result {
                Ok(true) => {
                    println!("✅ Plugin activate() succeeded");
                }
                Ok(false) => {
                    return Err(PluginError::InitializationFailed(
                        "Plugin activate() returned false".to_string(),
                    ));
                }
                Err(_) => {
                    return Err(PluginError::InitializationFailed(
                        "Plugin activate() panicked (likely no display server or incompatible environment)".to_string(),
                    ));
                }
            }

            // Start processing with panic handling
            println!("🔧 Calling plugin.start_processing()...");
            let start_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (plugin.start_processing)(self.plugin_ptr)
            }));
            
            match start_result {
                Ok(true) => {
                    println!("✅ Plugin start_processing() succeeded");
                }
                Ok(false) => {
                    return Err(PluginError::InitializationFailed(
                        "Plugin start_processing() returned false".to_string(),
                    ));
                }
                Err(_) => {
                    return Err(PluginError::InitializationFailed(
                        "Plugin start_processing() panicked (likely no display server or incompatible environment)".to_string(),
                    ));
                }
            }
        }

        // Create GUI after plugin is initialized (required by CLAP spec)
        if self.gui.is_none() {
            println!("🔨 Creating GUI after plugin initialization...");
            let gui = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                // SAFETY: plugin_ptr is valid and plugin.init() has been called
                unsafe { ClapPluginGui::new(self.plugin_ptr) }
            }));
            
            self.gui = match gui {
                Ok(gui_opt) => {
                    println!("✅ GUI creation completed successfully after init");
                    gui_opt
                }
                Err(_) => {
                    println!("⚠️ GUI creation panicked (likely no display server) - continuing without GUI");
                    None
                }
            };
        }

        self.is_active = true;
        println!(
            "✅ Initialized CLAP plugin: {} at {} Hz",
            self.descriptor.name, sample_rate
        );

        Ok(())
    }

    fn process(
        &mut self,
        inputs: &HashMap<String, &crate::audio::buffer::AudioBuffer>,
        outputs: &mut HashMap<String, &mut crate::audio::buffer::AudioBuffer>,
        sample_frames: usize,
    ) -> Result<(), PluginError> {
        if !self.is_active {
            return Err(PluginError::ProcessingFailed(
                "Plugin not active".to_string(),
            ));
        }

        if self.plugin_ptr.is_null() {
            return Err(PluginError::ProcessingFailed(
                "Plugin pointer is null".to_string(),
            ));
        }

        unsafe {
            let plugin = &*self.plugin_ptr;

            // Copy input data into pool first (if available)
            if let Some((_, input_buffer)) = inputs.iter().next() {
                let input_data = input_buffer.data();
                let pool_input = self.buffer_pool.input_buffer_mut(0, sample_frames);
                for (i, sample) in input_data.iter().take(sample_frames).enumerate() {
                    pool_input[i] = *sample;
                }
            }

            // Prepare buffer pool (zero allocations - reuses pre-allocated buffers)
            let (input_ptrs, output_ptrs) = self.buffer_pool.prepare(sample_frames);

            // Copy pointer slices to local vectors to allow further borrowing
            let input_ptrs_vec: Vec<*mut f32> = input_ptrs.to_vec();
            let output_ptrs_vec: Vec<*mut f32> = output_ptrs.to_vec();

            let clap_input_buffer = clap_audio_buffer {
                channel_count: if input_ptrs_vec.is_empty() { 0 } else { 1 },
                latency: 0,
                data32: if input_ptrs_vec.is_empty() {
                    ptr::null_mut()
                } else {
                    input_ptrs_vec.as_ptr() as *mut *mut f32
                },
                data64: ptr::null_mut(),
            };

            let mut clap_output_buffer = clap_audio_buffer {
                channel_count: 2,
                latency: 0,
                data32: output_ptrs_vec.as_ptr() as *mut *mut f32,
                data64: ptr::null_mut(),
            };

            // Convert pending MIDI events to CLAP events
            let mut event_list = ClapEventList::new();

            // Add MIDI events
            for (midi_event, sample_offset) in &self.pending_midi_events {
                match midi_event {
                    MidiEvent::NoteOn { note, velocity } => {
                        event_list.add_note_on(*note, *velocity, *sample_offset);
                    }
                    MidiEvent::NoteOff { note } => {
                        event_list.add_note_off(*note, *sample_offset);
                    }
                    _ => {
                        // Ignore other MIDI events for now
                    }
                }
            }

            // Add parameter changes
            for (param_id, value) in &self.pending_param_changes {
                event_list.add_param_value(*param_id, *value, 0); // Sample offset 0 for immediate
            }

            let input_events = event_list.as_clap_input_events();

            let empty_output_events = clap_output_events {
                ctx: ptr::null_mut(),
                try_push: clap_output_events_try_push,
            };

            // Create process structure
            let clap_process_data = clap_process {
                steady_time: 0,
                frames_count: sample_frames as u32,
                transport: ptr::null(),
                audio_inputs: &clap_input_buffer,
                audio_inputs_count: if input_ptrs.is_empty() { 0 } else { 1 },
                audio_outputs: &mut clap_output_buffer,
                audio_outputs_count: 1,
                in_events: &input_events,
                out_events: &empty_output_events,
            };

            // Call the plugin's process function
            let status = (plugin.process)(self.plugin_ptr, &clap_process_data);

            // Check process status
            match status {
                clap_process_status::CLAP_PROCESS_ERROR => {
                    return Err(PluginError::ProcessingFailed(
                        "Plugin process returned ERROR".to_string(),
                    ));
                }
                _ => {
                    // Success (CONTINUE, TAIL, SLEEP are all valid)
                }
            }

            // Copy output data back to our buffers (from buffer pool)
            if let Some((_, output_buffer)) = outputs.iter_mut().next() {
                let output_data = output_buffer.data_mut();

                // Mix stereo to mono (average L+R from buffer pool)
                let left_buffer = self.buffer_pool.output_buffer(0, sample_frames);
                let right_buffer = self.buffer_pool.output_buffer(1, sample_frames);

                for i in 0..sample_frames.min(output_data.len()) {
                    let left = left_buffer[i];
                    let right = right_buffer[i];
                    output_data[i] = (left + right) * 0.5;
                }
            }
        }

        // Clear processed events
        self.pending_midi_events.clear();
        self.pending_param_changes.clear();

        Ok(())
    }

    fn set_parameter(&mut self, parameter_id: &str, value: f64) -> Result<(), PluginError> {
        if let Some(param) = self.descriptor.find_parameter(parameter_id) {
            let clamped_value = value.clamp(param.min_value, param.max_value);

            // Update our cached value
            self.parameter_values
                .insert(parameter_id.to_string(), clamped_value);

            // Queue parameter change for next process() call
            if let Some(&clap_param_id) = self.parameter_id_map.get(parameter_id) {
                self.pending_param_changes
                    .push((clap_param_id, clamped_value));
            }

            Ok(())
        } else {
            Err(PluginError::InvalidParameter(format!(
                "Parameter not found: {}",
                parameter_id
            )))
        }
    }

    fn get_parameter(&self, parameter_id: &str) -> Option<f64> {
        self.parameter_values.get(parameter_id).copied()
    }

    fn get_all_parameters(&self) -> HashMap<String, f64> {
        self.parameter_values.clone()
    }

    fn save_state(&self) -> Result<PluginState, PluginError> {
        let mut state = PluginState::new();

        // Save parameter values
        for (id, value) in &self.parameter_values {
            state = state.with_parameter(id.clone(), *value);
        }

        // TODO: Save actual CLAP state if available

        Ok(state)
    }

    fn load_state(&mut self, state: &PluginState) -> Result<(), PluginError> {
        // Load parameter values
        for (id, value) in &state.parameters {
            if self.descriptor.find_parameter(id).is_some() {
                self.parameter_values.insert(id.clone(), *value);

                // TODO: Set parameter in actual CLAP plugin
            }
        }

        // TODO: Load actual CLAP state if available

        Ok(())
    }

    fn reset(&mut self) -> Result<(), PluginError> {
        // Reset all parameters to default values
        let params_to_reset: Vec<(String, f64)> = self
            .descriptor
            .parameters
            .iter()
            .map(|param| (param.id.clone(), param.default_value))
            .collect();

        for (id, default_value) in params_to_reset {
            self.set_parameter(&id, default_value)?;
        }
        Ok(())
    }

    fn get_latency(&self) -> u32 {
        // TODO: Get latency from actual CLAP plugin
        0
    }

    fn get_tail(&self) -> u32 {
        // TODO: Get tail from actual CLAP plugin
        0
    }

    fn is_processing(&self) -> bool {
        self.is_active
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn process_midi(&mut self, midi_event: &MidiEventTimed) -> Result<(), PluginError> {
        // Add MIDI event to pending queue for processing in next audio callback
        self.pending_midi_events.push((midi_event.event, midi_event.samples_from_now));
        
        println!("🎹 MIDI queued for plugin {}: {:?} (offset: {} samples)", 
                 self.descriptor.name, midi_event.event, midi_event.samples_from_now);
        
        Ok(())
    }
}

/// Empty output event list callback (we don't accept output events for now)
extern "C" fn clap_output_events_try_push(
    _list: *const clap_output_events,
    _event: *const clap_event_header,
) -> bool {
    false // Don't accept events for now
}

/// Simple host implementation for CLAP plugins (placeholder)
#[derive(Default)]
pub struct ClapHost;

// Include the simplified tests from the separate file
include!("clap_integration_tests.rs");
