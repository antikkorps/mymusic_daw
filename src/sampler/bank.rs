use crate::sampler::loader::{LoopMode, Sample};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Serializable sample bank configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleBank {
    pub name: String,
    pub version: String,
    pub samples: Vec<SampleMapping>,
}

/// Mapping from MIDI note to sample configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleMapping {
    /// MIDI note number (0-127)
    pub note: u8,
    /// Relative path to sample file (from bank file location)
    pub sample_path: PathBuf,
    /// Sample display name
    pub name: String,
    /// Volume multiplier
    pub volume: f32,
    /// Pan position (-1.0 left, 0.0 center, 1.0 right)
    pub pan: f32,
    /// Loop mode
    pub loop_mode: LoopMode,
    /// Loop start point in samples
    pub loop_start: usize,
    /// Loop end point in samples
    pub loop_end: usize,
    /// Reverse playback
    pub reverse: bool,
    /// Pitch offset in semitones (-12 to +12)
    pub pitch_offset: i8,
}

impl SampleBank {
    /// Create a new empty sample bank
    pub fn new(name: String) -> Self {
        Self {
            name,
            version: "1.0".to_string(),
            samples: Vec::new(),
        }
    }

    /// Add a sample mapping to the bank
    pub fn add_mapping(&mut self, mapping: SampleMapping) {
        // Remove any existing mapping for this note
        self.samples.retain(|m| m.note != mapping.note);
        self.samples.push(mapping);
    }

    /// Get mapping for a specific note
    pub fn get_mapping(&self, note: u8) -> Option<&SampleMapping> {
        self.samples.iter().find(|m| m.note == note)
    }

    /// Remove mapping for a specific note
    pub fn remove_mapping(&mut self, note: u8) -> bool {
        let initial_len = self.samples.len();
        self.samples.retain(|m| m.note != note);
        self.samples.len() < initial_len
    }

    /// Get all mappings sorted by note
    pub fn get_sorted_mappings(&self) -> Vec<&SampleMapping> {
        let mut mappings: Vec<&SampleMapping> = self.samples.iter().collect();
        mappings.sort_by_key(|m| m.note);
        mappings
    }

    /// Save bank to JSON file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let json_str = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize bank: {}", e))?;

        std::fs::write(path, json_str).map_err(|e| format!("Failed to write bank file: {}", e))?;

        Ok(())
    }

    /// Load bank from JSON file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let json_str = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read bank file: {}", e))?;

        serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse bank file: {}", e))
    }

    /// Convert from loaded samples and note mappings
    pub fn from_samples_and_mappings(
        name: String,
        samples: &[Sample],
        note_mappings: &[Option<String>],
        bank_base_path: &Path,
    ) -> Self {
        let mut bank = Self::new(name);

        for (note, maybe_path) in note_mappings.iter().enumerate() {
            if let Some(sample_path) = maybe_path {
                // Find the corresponding sample
                if let Some(sample) = samples.iter().find(|s| s.name == *sample_path) {
                    // Convert absolute path to relative path
                    let relative_path = if let Ok(abs_path) = std::fs::canonicalize(sample_path) {
                        if let Ok(base_abs) = std::fs::canonicalize(bank_base_path) {
                            abs_path
                                .strip_prefix(&base_abs)
                                .map(|p| p.to_path_buf())
                                .unwrap_or_else(|_| PathBuf::from(sample_path))
                        } else {
                            PathBuf::from(sample_path)
                        }
                    } else {
                        PathBuf::from(sample_path)
                    };

                    let mapping = SampleMapping {
                        note: note as u8,
                        sample_path: relative_path,
                        name: sample.name.clone(),
                        volume: sample.volume,
                        pan: sample.pan,
                        loop_mode: sample.loop_mode,
                        loop_start: sample.loop_start,
                        loop_end: sample.loop_end,
                        reverse: sample.reverse,
                        pitch_offset: sample.pitch_offset,
                    };

                    bank.add_mapping(mapping);
                }
            }
        }

        bank
    }
}

impl Default for SampleBank {
    fn default() -> Self {
        Self::new("Untitled Bank".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_sample_bank_serialization() {
        let mut bank = SampleBank::new("Test Bank".to_string());

        let mapping = SampleMapping {
            note: 60,
            sample_path: PathBuf::from("kick.wav"),
            name: "Kick Drum".to_string(),
            volume: 1.5,
            pan: 0.0,
            loop_mode: LoopMode::Off,
            loop_start: 0,
            loop_end: 44100,
            reverse: false,
            pitch_offset: 0,
        };

        bank.add_mapping(mapping);

        // Test serialization
        let json = serde_json::to_string(&bank).unwrap();
        let loaded: SampleBank = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.name, "Test Bank");
        assert_eq!(loaded.samples.len(), 1);
        assert_eq!(loaded.samples[0].note, 60);
        assert_eq!(loaded.samples[0].name, "Kick Drum");
    }

    #[test]
    fn test_save_load_bank() {
        let dir = tempdir().unwrap();
        let bank_path = dir.path().join("test_bank.json");

        let mut bank = SampleBank::new("Test Bank".to_string());

        let mapping = SampleMapping {
            note: 62,
            sample_path: PathBuf::from("snare.wav"),
            name: "Snare".to_string(),
            volume: 1.2,
            pan: -0.2,
            loop_mode: LoopMode::Forward,
            loop_start: 1000,
            loop_end: 20000,
            reverse: false,
            pitch_offset: 2,
        };

        bank.add_mapping(mapping);

        // Save and reload
        bank.save_to_file(&bank_path).unwrap();
        let loaded = SampleBank::load_from_file(&bank_path).unwrap();

        assert_eq!(loaded.name, "Test Bank");
        assert_eq!(loaded.samples.len(), 1);
        assert_eq!(loaded.samples[0].note, 62);
        assert_eq!(loaded.samples[0].volume, 1.2);
        assert_eq!(loaded.samples[0].pan, -0.2);
        assert_eq!(loaded.samples[0].loop_mode, LoopMode::Forward);
        assert_eq!(loaded.samples[0].loop_start, 1000);
        assert_eq!(loaded.samples[0].loop_end, 20000);
        assert!(!loaded.samples[0].reverse);
        assert_eq!(loaded.samples[0].pitch_offset, 2);
    }

    #[test]
    fn test_bank_operations() {
        let mut bank = SampleBank::new("Test".to_string());

        // Add mappings
        let mapping1 = SampleMapping {
            note: 60,
            sample_path: PathBuf::from("kick.wav"),
            name: "Kick".to_string(),
            volume: 1.0,
            pan: 0.0,
            loop_mode: LoopMode::Off,
            loop_start: 0,
            loop_end: 1000,
            reverse: false,
            pitch_offset: 0,
        };

        let mapping2 = SampleMapping {
            note: 62,
            sample_path: PathBuf::from("snare.wav"),
            name: "Snare".to_string(),
            volume: 1.0,
            pan: 0.0,
            loop_mode: LoopMode::Off,
            loop_start: 0,
            loop_end: 1000,
            reverse: false,
            pitch_offset: 0,
        };

        bank.add_mapping(mapping1);
        bank.add_mapping(mapping2);

        assert_eq!(bank.samples.len(), 2);

        // Test get_mapping
        assert!(bank.get_mapping(60).is_some());
        assert!(bank.get_mapping(61).is_none());
        assert!(bank.get_mapping(62).is_some());

        // Test remove_mapping
        assert!(bank.remove_mapping(60));
        assert_eq!(bank.samples.len(), 1);
        assert!(!bank.remove_mapping(60)); // Already removed
        assert_eq!(bank.samples.len(), 1);

        // Test sorted mappings
        let sorted = bank.get_sorted_mappings();
        assert_eq!(sorted.len(), 1);
        assert_eq!(sorted[0].note, 62);
    }
}
