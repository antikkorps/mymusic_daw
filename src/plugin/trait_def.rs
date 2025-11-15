use crate::audio::buffer::AudioBuffer;
use crate::plugin::PluginError;
use crate::plugin::parameters::*;
use crate::MidiEventTimed;
use std::collections::HashMap;

/// Core plugin trait that all plugins must implement
pub trait Plugin: Send + Sync {
    /// Get plugin descriptor
    fn descriptor(&self) -> &PluginDescriptor;

    /// Initialize the plugin with given sample rate
    fn initialize(&mut self, sample_rate: f64) -> Result<(), PluginError>;

    /// Process audio buffer
    ///
    /// # Arguments
    /// * `inputs` - Input audio buffers indexed by port ID
    /// * `outputs` - Output audio buffers indexed by port ID  
    /// * `sample_frames` - Number of samples to process
    fn process(
        &mut self,
        inputs: &HashMap<String, &AudioBuffer>,
        outputs: &mut HashMap<String, &mut AudioBuffer>,
        sample_frames: usize,
    ) -> Result<(), PluginError>;

    /// Set parameter value
    fn set_parameter(&mut self, parameter_id: &str, value: f64) -> Result<(), PluginError>;

    /// Get parameter value
    fn get_parameter(&self, parameter_id: &str) -> Option<f64>;

    /// Get all current parameter values
    fn get_all_parameters(&self) -> HashMap<String, f64>;

    /// Save plugin state
    fn save_state(&self) -> Result<PluginState, PluginError>;

    /// Load plugin state
    fn load_state(&mut self, state: &PluginState) -> Result<(), PluginError>;

    /// Reset plugin to default state
    fn reset(&mut self) -> Result<(), PluginError>;

    /// Get plugin latency in samples
    fn get_latency(&self) -> u32 {
        0
    }

    /// Get tail length in samples (for reverb, delay, etc.)
    fn get_tail(&self) -> u32 {
        0
    }

    /// Check if plugin is currently processing audio
    fn is_processing(&self) -> bool {
        false
    }

    /// Get plugin as Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;
    
    /// Get plugin as Any for downcasting (mutable)
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;

    /// Process MIDI event sent to plugin
    fn process_midi(&mut self, _midi_event: &MidiEventTimed) -> Result<(), PluginError> {
        // Default implementation does nothing
        Ok(())
    }
}

/// GUI-related plugin capabilities
pub trait PluginGui: Send {
    /// Check if plugin has a GUI
    fn has_gui(&self) -> bool;

    /// Show GUI (if supported)
    fn show_gui(&mut self) -> Result<(), PluginError>;

    /// Hide GUI (if supported)
    fn hide_gui(&mut self) -> Result<(), PluginError>;

    /// Check if GUI is currently visible
    fn is_gui_visible(&self) -> bool;

    /// Get preferred GUI size
    fn get_gui_size(&self) -> (u32, u32);

    /// Set GUI size (if resizable)
    fn set_gui_size(&mut self, width: u32, height: u32) -> Result<(), PluginError>;
}

/// Plugin factory for creating instances
pub trait PluginFactory: Send + Sync {
    /// Get plugin descriptor
    fn descriptor(&self) -> &PluginDescriptor;

    /// Create a new plugin instance
    fn create_instance(&self) -> Result<Box<dyn Plugin>, PluginError>;

    /// Check if this plugin supports the given feature
    fn supports_feature(&self, _feature: &str) -> bool {
        false
    }
}

/// Default implementation for plugins without GUI
pub struct NoGui;

impl PluginGui for NoGui {
    fn has_gui(&self) -> bool {
        false
    }

    fn show_gui(&mut self) -> Result<(), PluginError> {
        Err(PluginError::GuiFailed(
            "Plugin does not support GUI".to_string(),
        ))
    }

    fn hide_gui(&mut self) -> Result<(), PluginError> {
        Err(PluginError::GuiFailed(
            "Plugin does not support GUI".to_string(),
        ))
    }

    fn is_gui_visible(&self) -> bool {
        false
    }

    fn get_gui_size(&self) -> (u32, u32) {
        (0, 0)
    }

    fn set_gui_size(&mut self, _width: u32, _height: u32) -> Result<(), PluginError> {
        Err(PluginError::GuiFailed(
            "Plugin does not support GUI".to_string(),
        ))
    }
}
