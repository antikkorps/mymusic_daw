use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Plugin category for classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginCategory {
    Instrument,
    Effect,
    Analyzer,
    Generator,
    Utility,
    Other,
}

/// Plugin parameter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginParameter {
    pub id: String,
    pub name: String,
    pub module: String,
    pub min_value: f64,
    pub max_value: f64,
    pub default_value: f64,
    pub current_value: f64,
    pub is_automatable: bool,
    pub is_periodic: bool,
    pub cookie: u32,
}

impl PluginParameter {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        module: impl Into<String>,
        min_value: f64,
        max_value: f64,
        default_value: f64,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            module: module.into(),
            min_value,
            max_value,
            default_value,
            current_value: default_value,
            is_automatable: true,
            is_periodic: false,
            cookie: 0,
        }
    }

    pub fn with_automatable(mut self, automatable: bool) -> Self {
        self.is_automatable = automatable;
        self
    }

    pub fn with_periodic(mut self, periodic: bool) -> Self {
        self.is_periodic = periodic;
        self
    }

    pub fn with_cookie(mut self, cookie: u32) -> Self {
        self.cookie = cookie;
        self
    }

    /// Normalize value to [0.0, 1.0] range
    pub fn normalize(&self, value: f64) -> f64 {
        if self.max_value <= self.min_value {
            return 0.0;
        }
        ((value - self.min_value) / (self.max_value - self.min_value)).clamp(0.0, 1.0)
    }

    /// Denormalize value from [0.0, 1.0] range to parameter range
    pub fn denormalize(&self, normalized: f64) -> f64 {
        self.min_value + normalized * (self.max_value - self.min_value)
    }
}

/// Audio port configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioPortInfo {
    pub id: String,
    pub name: String,
    pub channel_count: u32,
    pub port_type: PortType,
    pub is_main: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortType {
    Input,
    Output,
}

/// Plugin metadata and descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDescriptor {
    pub id: String,
    pub name: String,
    pub version: String,
    pub vendor: String,
    pub description: String,
    pub url: String,
    pub category: PluginCategory,
    pub audio_inputs: Vec<AudioPortInfo>,
    pub audio_outputs: Vec<AudioPortInfo>,
    pub parameters: Vec<PluginParameter>,
    pub supports_dsp: bool,
    pub supports_gui: bool,
    pub supports_state: bool,
}

impl PluginDescriptor {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: "1.0.0".to_string(),
            vendor: "Unknown".to_string(),
            description: String::new(),
            url: String::new(),
            category: PluginCategory::Other,
            audio_inputs: Vec::new(),
            audio_outputs: Vec::new(),
            parameters: Vec::new(),
            supports_dsp: true,
            supports_gui: false,
            supports_state: true,
        }
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    pub fn with_vendor(mut self, vendor: impl Into<String>) -> Self {
        self.vendor = vendor.into();
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    pub fn with_category(mut self, category: PluginCategory) -> Self {
        self.category = category;
        self
    }

    pub fn with_audio_input(mut self, port: AudioPortInfo) -> Self {
        self.audio_inputs.push(port);
        self
    }

    pub fn with_audio_output(mut self, port: AudioPortInfo) -> Self {
        self.audio_outputs.push(port);
        self
    }

    pub fn with_parameter(mut self, param: PluginParameter) -> Self {
        self.parameters.push(param);
        self
    }

    pub fn with_dsp_support(mut self, supports: bool) -> Self {
        self.supports_dsp = supports;
        self
    }

    pub fn with_gui_support(mut self, supports: bool) -> Self {
        self.supports_gui = supports;
        self
    }

    pub fn with_state_support(mut self, supports: bool) -> Self {
        self.supports_state = supports;
        self
    }

    /// Find parameter by ID
    pub fn find_parameter(&self, id: &str) -> Option<&PluginParameter> {
        self.parameters.iter().find(|p| p.id == id)
    }

    /// Find parameter by ID (mutable)
    pub fn find_parameter_mut(&mut self, id: &str) -> Option<&mut PluginParameter> {
        self.parameters.iter_mut().find(|p| p.id == id)
    }
}

/// Plugin state for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginState {
    pub parameters: HashMap<String, f64>,
    pub custom_data: HashMap<String, String>,
}

impl Default for PluginState {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginState {
    pub fn new() -> Self {
        Self {
            parameters: HashMap::new(),
            custom_data: HashMap::new(),
        }
    }

    pub fn with_parameter(mut self, id: impl Into<String>, value: f64) -> Self {
        self.parameters.insert(id.into(), value);
        self
    }

    pub fn with_custom_data(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_data.insert(key.into(), value.into());
        self
    }
}

/// Unique identifier for a plugin instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginInstanceId(Uuid);

impl PluginInstanceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for PluginInstanceId {
    fn default() -> Self {
        Self::new()
    }
}
