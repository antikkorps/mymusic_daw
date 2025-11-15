use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Plugin category for classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginCategory {
    Instrument,
    Effect,
    Analyzer,
    Generator,
    Drum,
    Modulator,
    Spatial,
    Spacializer,
    Utility,
    Other,
}

/// Audio port information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioPortInfo {
    pub id: String,
    pub name: String,
    pub channel_count: u32,
    pub is_main: bool,
}

/// Plugin parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginParameter {
    pub id: String,
    pub name: String,
    pub value: f64,
    pub default_value: f64,
    pub min_value: f64,
    pub max_value: f64,
    pub is_automatable: bool,
    pub parameter_type: ParameterType,
}

/// Parameter type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    Linear,
    Logarithmic,
    Enum,
}

/// Plugin descriptor
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
    pub file_path: PathBuf, // Added for loading plugins
}

impl PluginDescriptor {
    pub fn new(id: impl Into<String>, name: impl Into<String>, file_path: PathBuf) -> Self {
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
            supports_state: false,
            file_path,
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

    pub fn with_file_path(mut self, file_path: PathBuf) -> Self {
        self.file_path = file_path;
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

    pub fn with_parameter(mut self, parameter: PluginParameter) -> Self {
        self.parameters.push(parameter);
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
}

impl Default for PluginInstanceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PluginInstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for PluginInstanceId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}
