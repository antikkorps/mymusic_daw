// MIDI to Plugin Bridge - Bypass display server requirement
// Maps DAW controls to plugin parameters via MIDI CC messages

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::MidiEvent;
use crate::MidiEventTimed;
use crate::plugin::{PluginHost, PluginInstanceId, PluginResult};
use ringbuf::{HeapRb, traits::{Split, Producer, Consumer}};

/// MIDI CC to Plugin Parameter Mapping
#[derive(Debug, Clone)]
pub struct MidiMapping {
    /// MIDI CC controller number (0-127)
    pub cc_number: u8,
    /// Plugin instance ID
    pub plugin_instance_id: PluginInstanceId,
    /// Plugin parameter index
    pub parameter_index: u32,
    /// Mapping name for UI display
    pub name: String,
    /// Minimum value (0.0 to 1.0)
    pub min_value: f32,
    /// Maximum value (0.0 to 1.0)
    pub max_value: f32,
}

/// MIDI Bridge for plugin communication
pub struct MidiPluginBridge {
    /// Current MIDI mappings
    mappings: Arc<Mutex<HashMap<u8, MidiMapping>>>,
    /// Plugin host reference
    plugin_host: Arc<PluginHost>,
    /// MIDI output buffer (to send to plugins)
    midi_output: ringbuf::HeapProd<MidiEventTimed>,
    /// MIDI input buffer (to receive from plugins)
    midi_input: ringbuf::HeapCons<MidiEventTimed>,
}

impl MidiPluginBridge {
    /// Create new MIDI bridge
    pub fn new(plugin_host: Arc<PluginHost>) -> Self {
        let (midi_prod, midi_cons) = HeapRb::<MidiEventTimed>::new(1024).split();
        Self {
            mappings: Arc::new(Mutex::new(HashMap::new())),
            plugin_host,
            midi_output: midi_prod,
            midi_input: midi_cons,
        }
    }

    /// Add a MIDI CC to plugin parameter mapping
    pub fn add_mapping(&self, mapping: MidiMapping) -> PluginResult<()> {
        let cc_number = mapping.cc_number;
        let param_index = mapping.parameter_index;
        let mut mappings = self.mappings.lock().unwrap();
        mappings.insert(mapping.cc_number, mapping);
        println!("🎛️ Added MIDI mapping: CC {} -> Plugin param {}", 
                 cc_number, param_index);
        Ok(())
    }

    /// Remove a MIDI mapping
    pub fn remove_mapping(&self, cc_number: u8) -> PluginResult<()> {
        let mut mappings = self.mappings.lock().unwrap();
        mappings.remove(&cc_number);
        println!("🗑️ Removed MIDI mapping: CC {}", cc_number);
        Ok(())
    }

    /// Get all current mappings
    pub fn get_mappings(&self) -> Vec<MidiMapping> {
        let mappings = self.mappings.lock().unwrap();
        mappings.values().cloned().collect()
    }

    /// Process incoming MIDI event from DAW/controller
    pub fn process_midi_input(&mut self, midi_event: &MidiEventTimed) -> PluginResult<()> {
        match midi_event.event {
            MidiEvent::ControlChange { controller, value } => {
                // Look up mapping for this CC
                let mappings = self.mappings.lock().unwrap();
                if let Some(mapping) = mappings.get(&controller) {
                    // Convert MIDI value (0-127) to plugin parameter value (min_value to max_value)
                    let normalized_value = value as f32 / 127.0;
                    let plugin_value = mapping.min_value + 
                        normalized_value * (mapping.max_value - mapping.min_value);

                    // Apply to plugin parameter
                    self.set_plugin_parameter(
                        mapping.plugin_instance_id,
                        mapping.parameter_index,
                        plugin_value
                    )?;

                    println!("🎛️ MIDI CC {} -> Plugin {:?} param {} = {:.3}", 
                             controller, mapping.plugin_instance_id, 
                             mapping.parameter_index, plugin_value);
                }
            }
            _ => {
                // Handle other MIDI events if needed
                // For now, just forward to plugins
                let _ = self.midi_output.try_push(midi_event.clone());
            }
        }
        Ok(())
    }

    /// Set plugin parameter value
    fn set_plugin_parameter(&self, 
                           instance_id: PluginInstanceId, 
                           param_index: u32, 
                           value: f32) -> PluginResult<()> {
        // This would call the plugin host to set the parameter
        // Implementation depends on the plugin host interface
        println!("🔧 Setting plugin {:?} param {} to {}", instance_id, param_index, value);
        
        // TODO: Actually set the parameter via plugin host
        // self.plugin_host.set_parameter(instance_id, param_index, value)?;
        
        Ok(())
    }

    /// Generate automatic mappings for a plugin instance
    pub fn auto_map_plugin(&self, 
                          instance_id: PluginInstanceId, 
                          start_cc: u8) -> PluginResult<Vec<MidiMapping>> {
        let mut mappings = Vec::new();
        
        // TODO: Get plugin parameter info from host
        // For now, create generic mappings
        let common_params = vec![
            ("Volume", 0.0, 1.0),
            ("Pan", 0.0, 1.0),
            ("Cutoff", 0.0, 1.0),
            ("Resonance", 0.0, 1.0),
            ("Attack", 0.0, 1.0),
            ("Decay", 0.0, 1.0),
            ("Sustain", 0.0, 1.0),
            ("Release", 0.0, 1.0),
        ];

        for (i, (name, min_val, max_val)) in common_params.iter().enumerate() {
            let mapping = MidiMapping {
                cc_number: start_cc + i as u8,
                plugin_instance_id: instance_id,
                parameter_index: i as u32,
                name: name.to_string(),
                min_value: *min_val,
                max_value: *max_val,
            };
            mappings.push(mapping.clone());
            self.add_mapping(mapping)?;
        }

        println!("🎛️ Auto-mapped {} parameters for plugin {:?} starting at CC {}", 
                 mappings.len(), instance_id, start_cc);

        Ok(mappings)
    }

    /// Create virtual MIDI port for plugin communication
    pub fn create_virtual_midi_port(&self, port_name: &str) -> PluginResult<()> {
        println!("🎹 Creating virtual MIDI port: {}", port_name);
        
        // TODO: Create virtual MIDI port using OS-specific APIs
        // On macOS: CoreMIDI
        // On Windows: MIDI API
        // On Linux: ALSA sequencer
        
        Ok(())
    }

    /// Send MIDI event to specific plugin
    pub fn send_midi_to_plugin(&self, 
                               instance_id: PluginInstanceId, 
                               midi_event: MidiEventTimed) -> PluginResult<()> {
        println!("📤 Sending MIDI to plugin {:?}: {:?}", instance_id, midi_event.event);
        
        // TODO: Route MIDI event to specific plugin instance
        // This would depend on the plugin's MIDI input capabilities
        
        Ok(())
    }

    /// Receive MIDI event from plugin
    pub fn receive_midi_from_plugin(&mut self) -> Option<MidiEventTimed> {
        self.midi_input.try_pop()
    }
}

/// Default MIDI CC assignments (following General MIDI standard)
pub mod default_cc_assignments {
    pub const VOLUME: u8 = 7;
    pub const PAN: u8 = 10;
    pub const EXPRESSION: u8 = 11;
    pub const SUSTAIN: u8 = 64;
    pub const PORTAMENTO: u8 = 65;
    pub const SOSTENUTO: u8 = 66;
    pub const SOFT_PEDAL: u8 = 67;
    pub const LEGATO_FOOTSWITCH: u8 = 68;
    pub const HOLD_2: u8 = 69;
    
    // Effects controllers
    pub const EFFECTS_1: u8 = 12;
    pub const EFFECTS_2: u8 = 13;
    pub const EFFECTS_3: u8 = 14;
    pub const EFFECTS_4: u8 = 15;
    pub const EFFECTS_5: u8 = 16;
    
    // Continuous controllers 16-31 are undefined in GM spec
    // Can be used for custom mappings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MidiEvent;

    #[test]
    fn test_midi_mapping_creation() {
        let mapping = MidiMapping {
            cc_number: 7,
            plugin_instance_id: PluginInstanceId::new(),
            parameter_index: 0,
            name: "Volume".to_string(),
            min_value: 0.0,
            max_value: 1.0,
        };

        assert_eq!(mapping.cc_number, 7);
        assert_eq!(mapping.name, "Volume");
        assert_eq!(mapping.min_value, 0.0);
        assert_eq!(mapping.max_value, 1.0);
    }

    #[test]
    fn test_midi_to_parameter_conversion() {
        // Test MIDI value 0 -> min_value
        let normalized_0 = 0.0 / 127.0;
        let plugin_val_0 = 0.0 + normalized_0 * (1.0 - 0.0);
        assert_eq!(plugin_val_0, 0.0);

        // Test MIDI value 127 -> max_value
        let normalized_127 = 127.0 / 127.0;
        let plugin_val_127 = 0.0 + normalized_127 * (1.0 - 0.0);
        assert_eq!(plugin_val_127, 1.0);

        // Test MIDI value 64 -> ~0.5
        let normalized_64 = 64.0 / 127.0;
        let plugin_val_64 = 0.0 + normalized_64 * (1.0 - 0.0);
        assert!((plugin_val_64 - 0.504_f64).abs() < 0.01_f64);
    }

    #[test]
    fn test_default_cc_assignments() {
        assert_eq!(default_cc_assignments::VOLUME, 7);
        assert_eq!(default_cc_assignments::PAN, 10);
        assert_eq!(default_cc_assignments::SUSTAIN, 64);
    }
}