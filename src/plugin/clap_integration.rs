// CLAP Plugin Integration
//
// This module provides real integration with CLAP (CLever Audio Plug-in API) plugins.
// Uses libloading for dynamic loading and FFI for C API interop.

use crate::plugin::clap_ffi::*;
use crate::plugin::parameters::*;
use crate::plugin::trait_def::*;
use crate::plugin::{PluginError, PluginResult};
use libloading::{Library, Symbol};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::path::Path;
use std::ptr;
use std::sync::Arc;

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
            Library::new(&library_path).map_err(|e| {
                PluginError::LoadFailed(format!("Failed to load library: {}", e))
            })?
        };

        // Get the clap_entry symbol
        let entry_ptr: *const clap_plugin_entry = unsafe {
            let symbol: Symbol<*const clap_plugin_entry> = library
                .get(b"clap_entry\0")
                .map_err(|e| {
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
        let path_cstr = CString::new(path).map_err(|_| {
            PluginError::LoadFailed("Invalid path string".to_string())
        })?;

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
        let descriptor = unsafe { convert_clap_descriptor(clap_descriptor_ptr)? };

        println!("✅ Loaded CLAP plugin: {} ({})", descriptor.name, descriptor.id);

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
                .ok_or_else(|| {
                    PluginError::LoadFailed("Invalid bundle name".to_string())
                })?;

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
) -> PluginResult<PluginDescriptor> {
    let desc = &*clap_desc;

    let id = c_str_to_string(desc.id)
        .ok_or_else(|| PluginError::LoadFailed("Invalid plugin ID".to_string()))?;

    let name = c_str_to_string(desc.name)
        .ok_or_else(|| PluginError::LoadFailed("Invalid plugin name".to_string()))?;

    let vendor = c_str_to_string(desc.vendor).unwrap_or_else(|| "Unknown".to_string());
    let version = c_str_to_string(desc.version).unwrap_or_else(|| "1.0.0".to_string());
    let description = c_str_to_string(desc.description).unwrap_or_else(|| "".to_string());

    // Parse features to determine category
    let features = read_string_array(desc.features);
    let category = infer_category_from_features(&features);

    let mut descriptor = PluginDescriptor::new(&id, &name)
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

impl PluginFactory for ClapPluginFactory {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn create_instance(&self) -> Result<Box<dyn Plugin>, PluginError> {
        // TODO: Implement actual CLAP instance creation
        // For now, return a placeholder instance
        Ok(Box::new(ClapPluginInstance::new(self.descriptor.clone())))
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

/// CLAP plugin instance implementation (placeholder)
pub struct ClapPluginInstance {
    descriptor: PluginDescriptor,
    parameter_values: HashMap<String, f64>,
    is_active: bool,
}

impl ClapPluginInstance {
    /// Create a new CLAP plugin instance
    pub fn new(descriptor: PluginDescriptor) -> Self {
        let mut parameter_values = HashMap::new();

        // Initialize parameter values with defaults
        for param in &descriptor.parameters {
            parameter_values.insert(param.id.clone(), param.default_value);
        }

        Self {
            descriptor,
            parameter_values,
            is_active: false,
        }
    }
}

impl Plugin for ClapPluginInstance {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    fn initialize(&mut self, sample_rate: f64) -> Result<(), PluginError> {
        // TODO: Implement actual CLAP plugin initialization
        println!("Initializing CLAP plugin with sample rate: {}", sample_rate);
        self.is_active = true;
        Ok(())
    }

    fn process(
        &mut self,
        _inputs: &HashMap<String, &crate::audio::buffer::AudioBuffer>,
        _outputs: &mut HashMap<String, &mut crate::audio::buffer::AudioBuffer>,
        _sample_frames: usize,
    ) -> Result<(), PluginError> {
        if !self.is_active {
            return Err(PluginError::ProcessingFailed(
                "Plugin not active".to_string(),
            ));
        }

        // TODO: Implement actual CLAP audio processing
        // For now, just pass through (silence)
        Ok(())
    }

    fn set_parameter(&mut self, parameter_id: &str, value: f64) -> Result<(), PluginError> {
        if let Some(param) = self.descriptor.find_parameter(parameter_id) {
            let clamped_value = value.clamp(param.min_value, param.max_value);

            // Update our cached value
            self.parameter_values
                .insert(parameter_id.to_string(), clamped_value);

            // TODO: Set parameter in actual CLAP plugin

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
}

/// Simple host implementation for CLAP plugins (placeholder)
#[derive(Default)]
pub struct ClapHost;

// Include the simplified tests from the separate file
include!("clap_integration_tests.rs");
