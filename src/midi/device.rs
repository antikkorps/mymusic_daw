// Gestion des devices MIDI

use midir::{MidiInput as MidirInput, MidiInputPort};

#[derive(Clone, Debug)]
pub struct MidiDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

pub struct MidiDeviceManager;

impl MidiDeviceManager {
    pub fn new() -> Self {
        Self
    }

    /// Liste tous les ports MIDI disponibles
    pub fn list_input_ports(&self) -> Vec<MidiDeviceInfo> {
        let mut devices = Vec::new();

        // Créer une instance temporaire pour lister les ports
        if let Ok(midi_in) = MidirInput::new("MyMusic DAW MIDI Scanner") {
            let ports = midi_in.ports();

            for (index, port) in ports.iter().enumerate() {
                if let Ok(name) = midi_in.port_name(port) {
                    devices.push(MidiDeviceInfo {
                        id: format!("midi_in_{}", index),
                        name: name.clone(),
                        is_default: index == 0, // Le premier port est considéré comme défaut
                    });
                }
            }
        }

        devices
    }

    /// Récupère le premier port MIDI disponible (port par défaut)
    pub fn get_default_input_port(&self) -> Option<(MidirInput, MidiInputPort)> {
        let midi_in = MidirInput::new("MyMusic DAW MIDI Input").ok()?;
        let ports = midi_in.ports();

        if ports.is_empty() {
            return None;
        }

        let port = ports.into_iter().next()?;
        Some((midi_in, port))
    }

    /// Récupère un port MIDI par son nom
    pub fn get_input_port_by_name(&self, device_name: &str) -> Option<(MidirInput, MidiInputPort)> {
        let midi_in = MidirInput::new("MyMusic DAW MIDI Input").ok()?;
        let ports = midi_in.ports();

        for port in ports {
            if let Ok(name) = midi_in.port_name(&port) {
                if name == device_name {
                    return Some((midi_in, port));
                }
            }
        }

        None
    }
}

impl Default for MidiDeviceManager {
    fn default() -> Self {
        Self::new()
    }
}
