use crate::plugin::parameters::*;
use crate::plugin::trait_def::*;
use crate::plugin::{PluginError, PluginResult};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Wrapper for managing a plugin instance with additional metadata
pub struct PluginInstance {
    /// The actual plugin
    plugin: Box<dyn Plugin>,
    /// Instance ID
    id: PluginInstanceId,
    /// Instance name (for user identification)
    name: String,
    /// Whether the instance is currently active
    is_active: bool,
    /// Whether the instance is currently processing audio
    is_processing: bool,
    /// Current sample rate
    sample_rate: f64,
    /// Buffer size
    buffer_size: usize,
    /// Input audio buffers
    input_buffers: HashMap<String, crate::audio::buffer::AudioBuffer>,
    /// Output audio buffers
    output_buffers: HashMap<String, crate::audio::buffer::AudioBuffer>,
    /// Parameter change queue (for thread-safe parameter updates)
    parameter_queue: Arc<Mutex<Vec<ParameterChange>>>,
}

/// Parameter change event
#[derive(Debug, Clone)]
pub struct ParameterChange {
    pub parameter_id: String,
    pub value: f64,
    pub timestamp: u64,
}

impl PluginInstance {
    /// Create a new plugin instance
    pub fn new(plugin: Box<dyn Plugin>, id: PluginInstanceId, name: String) -> Self {
        Self {
            plugin,
            id,
            name,
            is_active: false,
            is_processing: false,
            sample_rate: 44100.0,
            buffer_size: 512,
            input_buffers: HashMap::new(),
            output_buffers: HashMap::new(),
            parameter_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get instance ID
    pub fn id(&self) -> PluginInstanceId {
        self.id
    }

    /// Get instance name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set instance name
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Get plugin descriptor
    pub fn descriptor(&self) -> &PluginDescriptor {
        self.plugin.descriptor()
    }

    /// Check if instance is active
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Check if instance is processing
    pub fn is_processing(&self) -> bool {
        self.is_processing
    }

    /// Get current sample rate
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    /// Get current buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    /// Initialize the plugin instance
    pub fn initialize(&mut self, sample_rate: f64, buffer_size: usize) -> PluginResult<()> {
        self.sample_rate = sample_rate;
        self.buffer_size = buffer_size;

        // Initialize audio buffers
        self.setup_audio_buffers()?;

        // Initialize the plugin
        self.plugin.initialize(sample_rate)?;

        self.is_active = true;
        Ok(())
    }

    /// Setup audio buffers based on plugin descriptor
    fn setup_audio_buffers(&mut self) -> PluginResult<()> {
        let descriptor = self.plugin.descriptor();

        // Setup input buffers
        for input_port in &descriptor.audio_inputs {
            let buffer = crate::audio::buffer::AudioBuffer::new(
                input_port.channel_count as usize * self.buffer_size,
            );
            self.input_buffers.insert(input_port.id.clone(), buffer);
        }

        // Setup output buffers
        for output_port in &descriptor.audio_outputs {
            let buffer = crate::audio::buffer::AudioBuffer::new(
                output_port.channel_count as usize * self.buffer_size,
            );
            self.output_buffers.insert(output_port.id.clone(), buffer);
        }

        Ok(())
    }

    /// Process audio through the plugin
    pub fn process(&mut self, _sample_frames: usize) -> PluginResult<()> {
        if !self.is_active {
            return Err(PluginError::ProcessingFailed(
                "Plugin not active".to_string(),
            ));
        }

        self.is_processing = true;

        // Apply queued parameter changes
        self.apply_parameter_changes()?;

        // Process audio
        // Convert buffers to references as expected by the trait
        let mut input_refs: HashMap<String, &crate::audio::buffer::AudioBuffer> = HashMap::new();
        for (key, buffer) in &self.input_buffers {
            input_refs.insert(key.clone(), buffer);
        }

        let mut output_refs: HashMap<String, &mut crate::audio::buffer::AudioBuffer> =
            HashMap::new();
        for (key, buffer) in &mut self.output_buffers {
            output_refs.insert(key.clone(), buffer);
        }

        let result = self
            .plugin
            .process(&input_refs, &mut output_refs, self.buffer_size);

        self.is_processing = false;
        result
    }

    /// Apply queued parameter changes
    fn apply_parameter_changes(&mut self) -> PluginResult<()> {
        let mut queue = self.parameter_queue.lock().unwrap();

        for change in queue.drain(..) {
            self.plugin
                .set_parameter(&change.parameter_id, change.value)?;
        }

        Ok(())
    }

    /// Queue a parameter change (thread-safe)
    pub fn set_parameter(&self, parameter_id: String, value: f64) {
        let change = ParameterChange {
            parameter_id,
            value,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };

        let mut queue = self.parameter_queue.lock().unwrap();
        queue.push(change);
    }

    /// Get parameter value immediately
    pub fn get_parameter(&self, parameter_id: &str) -> Option<f64> {
        self.plugin.get_parameter(parameter_id)
    }

    /// Get all parameter values
    pub fn get_all_parameters(&self) -> HashMap<String, f64> {
        self.plugin.get_all_parameters()
    }

    /// Save plugin state
    pub fn save_state(&self) -> PluginResult<PluginState> {
        self.plugin.save_state()
    }

    /// Load plugin state
    pub fn load_state(&mut self, state: &PluginState) -> PluginResult<()> {
        self.plugin.load_state(state)
    }

    /// Reset plugin to default state
    pub fn reset(&mut self) -> PluginResult<()> {
        self.plugin.reset()
    }

    /// Get plugin latency
    pub fn get_latency(&self) -> u32 {
        self.plugin.get_latency()
    }

    /// Get plugin tail length
    pub fn get_tail(&self) -> u32 {
        self.plugin.get_tail()
    }

    /// Get input buffer by port ID
    pub fn get_input_buffer(&self, port_id: &str) -> Option<&crate::audio::buffer::AudioBuffer> {
        self.input_buffers.get(port_id)
    }

    /// Get mutable input buffer by port ID
    pub fn get_input_buffer_mut(
        &mut self,
        port_id: &str,
    ) -> Option<&mut crate::audio::buffer::AudioBuffer> {
        self.input_buffers.get_mut(port_id)
    }

    /// Get output buffer by port ID
    pub fn get_output_buffer(&self, port_id: &str) -> Option<&crate::audio::buffer::AudioBuffer> {
        self.output_buffers.get(port_id)
    }

    /// Get mutable output buffer by port ID
    pub fn get_output_buffer_mut(
        &mut self,
        port_id: &str,
    ) -> Option<&mut crate::audio::buffer::AudioBuffer> {
        self.output_buffers.get_mut(port_id)
    }

    /// Deactivate the plugin instance
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.is_processing = false;

        // Clear buffers
        for buffer in self.input_buffers.values_mut() {
            buffer.clear();
        }

        for buffer in self.output_buffers.values_mut() {
            buffer.clear();
        }
    }

    /// Get GUI interface if supported
    pub fn as_gui(&mut self) -> Option<&mut dyn PluginGui> {
        // This requires downcasting, which is complex in Rust
        // For now, return None - GUI support will be added later
        None
    }

    /// Get instance information
    pub fn get_info(&self) -> PluginInstanceInfo {
        PluginInstanceInfo {
            id: self.id,
            name: self.name.clone(),
            plugin_id: self.plugin.descriptor().id.clone(),
            plugin_name: self.plugin.descriptor().name.clone(),
            is_active: self.is_active,
            is_processing: self.is_processing,
            sample_rate: self.sample_rate,
            buffer_size: self.buffer_size,
            latency: self.get_latency(),
            tail: self.get_tail(),
        }
    }
}

/// Plugin instance information
#[derive(Debug, Clone)]
pub struct PluginInstanceInfo {
    pub id: PluginInstanceId,
    pub name: String,
    pub plugin_id: String,
    pub plugin_name: String,
    pub is_active: bool,
    pub is_processing: bool,
    pub sample_rate: f64,
    pub buffer_size: usize,
    pub latency: u32,
    pub tail: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock plugin for testing
    struct MockPlugin {
        descriptor: PluginDescriptor,
        initialized: bool,
    }

    impl Plugin for MockPlugin {
        fn descriptor(&self) -> &PluginDescriptor {
            &self.descriptor
        }

        fn initialize(&mut self, _sample_rate: f64) -> Result<(), PluginError> {
            self.initialized = true;
            Ok(())
        }

        fn process(
            &mut self,
            _inputs: &HashMap<String, &crate::audio::buffer::AudioBuffer>,
            _outputs: &mut HashMap<String, &mut crate::audio::buffer::AudioBuffer>,
            _sample_frames: usize,
        ) -> Result<(), PluginError> {
            if !self.initialized {
                return Err(PluginError::ProcessingFailed("Not initialized".to_string()));
            }
            Ok(())
        }

        fn set_parameter(&mut self, _parameter_id: &str, _value: f64) -> Result<(), PluginError> {
            Ok(())
        }

        fn get_parameter(&self, _parameter_id: &str) -> Option<f64> {
            Some(0.0)
        }

        fn get_all_parameters(&self) -> HashMap<String, f64> {
            HashMap::new()
        }

        fn save_state(&self) -> Result<PluginState, PluginError> {
            Ok(PluginState::new())
        }

        fn load_state(&mut self, _state: &PluginState) -> Result<(), PluginError> {
            Ok(())
        }

        fn reset(&mut self) -> Result<(), PluginError> {
            Ok(())
        }
    }

    #[test]
    fn test_plugin_instance_creation() {
        let descriptor = PluginDescriptor::new(
            "test",
            "Test Plugin",
            std::path::PathBuf::from("/test/plugin.clap"),
        );
        let plugin = Box::new(MockPlugin {
            descriptor: descriptor.clone(),
            initialized: false,
        });

        let instance =
            PluginInstance::new(plugin, PluginInstanceId::new(), "Test Instance".to_string());

        assert_eq!(instance.name(), "Test Instance");
        assert!(!instance.is_active());
        assert!(!instance.is_processing());
    }

    #[test]
    fn test_initialization() {
        let descriptor = PluginDescriptor::new(
            "test",
            "Test Plugin",
            std::path::PathBuf::from("/test/plugin.clap"),
        );
        let plugin = Box::new(MockPlugin {
            descriptor: descriptor.clone(),
            initialized: false,
        });

        let mut instance =
            PluginInstance::new(plugin, PluginInstanceId::new(), "Test Instance".to_string());

        instance.initialize(44100.0, 512).unwrap();

        assert!(instance.is_active());
        assert_eq!(instance.sample_rate(), 44100.0);
        assert_eq!(instance.buffer_size(), 512);
    }

    #[test]
    fn test_parameter_queue() {
        let descriptor = PluginDescriptor::new(
            "test",
            "Test Plugin",
            std::path::PathBuf::from("/test/plugin.clap"),
        );
        let plugin = Box::new(MockPlugin {
            descriptor: descriptor.clone(),
            initialized: false,
        });

        let instance =
            PluginInstance::new(plugin, PluginInstanceId::new(), "Test Instance".to_string());

        instance.set_parameter("test_param".to_string(), 0.5);

        let queue = instance.parameter_queue.lock().unwrap();
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].parameter_id, "test_param");
        assert_eq!(queue[0].value, 0.5);
    }
}
