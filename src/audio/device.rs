// Gestion des devices audio CPAL

use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, Host};

#[derive(Clone, Debug)]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

pub struct AudioDeviceManager {
    host: Host,
}

impl AudioDeviceManager {
    pub fn new() -> Self {
        Self {
            host: cpal::default_host(),
        }
    }

    /// Liste tous les périphériques de sortie audio disponibles
    pub fn list_output_devices(&self) -> Vec<AudioDeviceInfo> {
        let mut devices = Vec::new();

        // Récupérer le périphérique par défaut
        let default_device = self.host.default_output_device();
        let default_name = default_device
            .as_ref()
            .and_then(|d| d.name().ok())
            .unwrap_or_default();

        // Énumérer tous les périphériques de sortie
        if let Ok(output_devices) = self.host.output_devices() {
            for (index, device) in output_devices.enumerate() {
                if let Ok(name) = device.name() {
                    let is_default = name == default_name;
                    devices.push(AudioDeviceInfo {
                        id: format!("audio_out_{}", index),
                        name: name.clone(),
                        is_default,
                    });
                }
            }
        }

        devices
    }

    /// Récupère le périphérique de sortie par défaut
    pub fn get_default_output_device(&self) -> Option<Device> {
        self.host.default_output_device()
    }

    /// Récupère un périphérique par son nom
    pub fn get_output_device_by_name(&self, device_name: &str) -> Option<Device> {
        if let Ok(devices) = self.host.output_devices() {
            for device in devices {
                if let Ok(name) = device.name()
                    && name == device_name
                {
                    return Some(device);
                }
            }
        }
        None
    }
}

impl Default for AudioDeviceManager {
    fn default() -> Self {
        Self::new()
    }
}
