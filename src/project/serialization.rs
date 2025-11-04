// Serialization utilities for project persistence

use crate::project::types::*;
use ron::{from_str as ron_from_str, to_string as ron_to_string};
use std::path::{Path, PathBuf};

/// Serialize project state to RON format
pub fn serialize_to_ron(project: &Project) -> Result<String, crate::project::ProjectError> {
    ron_to_string(project).map_err(|e| {
        crate::project::ProjectError::SerializationError(format!(
            "Failed to serialize to RON: {}",
            e
        ))
    })
}

/// Deserialize project state from RON format
pub fn deserialize_from_ron(ron_data: &str) -> Result<Project, crate::project::ProjectError> {
    ron_from_str(ron_data).map_err(|e| {
        crate::project::ProjectError::SerializationError(format!(
            "Failed to deserialize from RON: {}",
            e
        ))
    })
}

/// Serialize project metadata to JSON format
pub fn serialize_metadata_to_json(
    metadata: &ProjectMetadata,
) -> Result<String, crate::project::ProjectError> {
    serde_json::to_string_pretty(metadata).map_err(|e| {
        crate::project::ProjectError::SerializationError(format!(
            "Failed to serialize metadata to JSON: {}",
            e
        ))
    })
}

/// Deserialize project metadata from JSON format
pub fn deserialize_metadata_from_json(
    json_data: &str,
) -> Result<ProjectMetadata, crate::project::ProjectError> {
    serde_json::from_str(json_data).map_err(|e| {
        crate::project::ProjectError::SerializationError(format!(
            "Failed to deserialize metadata from JSON: {}",
            e
        ))
    })
}

/// Convert existing Pattern to serializable PatternSerializable
pub fn pattern_to_serializable(
    pattern: &crate::sequencer::pattern::Pattern,
) -> PatternSerializable {
    PatternSerializable {
        id: pattern.id,
        name: pattern.name.clone(),
        length_bars: pattern.length_bars,
        notes: pattern
            .notes()
            .iter()
            .map(|note| SerializableNote {
                id: note.id,
                pitch: note.pitch,
                start_samples: note.start.samples,
                duration_samples: note.duration_samples,
                velocity: note.velocity,
            })
            .collect(),
    }
}

/// Convert serializable PatternSerializable back to Pattern
pub fn pattern_from_serializable(
    serializable: &PatternSerializable,
    sample_rate: f64,
) -> crate::sequencer::pattern::Pattern {
    let mut pattern = crate::sequencer::pattern::Pattern::new(
        serializable.id,
        serializable.name.clone(),
        serializable.length_bars,
    );

    // Recreate notes from serializable data
    for serializable_note in &serializable.notes {
        let position = crate::sequencer::timeline::Position::from_samples(
            serializable_note.start_samples,
            sample_rate,
            &crate::sequencer::timeline::Tempo::default(),
            &crate::sequencer::timeline::TimeSignature::default(),
        );

        let note = crate::sequencer::note::Note::new(
            serializable_note.id,
            serializable_note.pitch,
            position,
            serializable_note.duration_samples,
            serializable_note.velocity,
        );

        pattern.add_note(note);
    }

    pattern
}

/// Export samples referenced by a project to audio directory
pub fn export_samples_to_directory(
    project: &Project,
    source_dir: &Path,
    target_dir: &Path,
) -> Result<std::collections::HashMap<String, PathBuf>, crate::project::ProjectError> {
    use std::fs;

    if let Some(sample_bank) = &project.sample_bank {
        let mut exported_samples = std::collections::HashMap::new();

        // Create audio/samples directory
        let samples_dir = target_dir.join("audio").join("samples");
        fs::create_dir_all(&samples_dir).map_err(|e| {
            crate::project::ProjectError::FileSystemError(format!(
                "Failed to create samples directory: {}",
                e
            ))
        })?;

        for mapping in &sample_bank.samples {
            let source_path = source_dir.join(&mapping.sample_path);

            if source_path.exists() {
                let sample_name = mapping
                    .sample_path
                    .file_name()
                    .ok_or_else(|| {
                        crate::project::ProjectError::InvalidStructure(
                            "Invalid sample path".to_string(),
                        )
                    })?
                    .to_string_lossy()
                    .to_string();

                let target_path = samples_dir.join(&sample_name);

                // Copy file
                fs::copy(&source_path, &target_path).map_err(|e| {
                    crate::project::ProjectError::FileSystemError(format!(
                        "Failed to copy sample {}: {}",
                        sample_name, e
                    ))
                })?;

                // Update mapping with new relative path
                let new_relative_path = PathBuf::from("samples").join(&sample_name);
                exported_samples.insert(sample_name, new_relative_path);
            }
        }

        Ok(exported_samples)
    } else {
        Ok(std::collections::HashMap::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_ron_serialization() {
        let project = crate::project::Project::default();
        let ron_data = serialize_to_ron(&project).unwrap();

        // Should contain recognizable strings
        assert!(ron_data.contains("Project"));
        assert!(ron_data.contains("Untitled Project"));

        // Round trip test
        let deserialized: crate::project::Project = deserialize_from_ron(&ron_data).unwrap();
        assert_eq!(deserialized.metadata.name, "Untitled Project");
    }

    #[test]
    fn test_json_metadata_serialization() {
        let metadata = crate::project::types::ProjectMetadata {
            name: "Test Project".to_string(),
            version: crate::project::types::ProjectVersion::new(1, 0, 0),
            created: "2023-01-01T00:00:00Z".to_string(),
            modified: "2023-01-01T00:00:00Z".to_string(),
            tempo: 120.0,
            time_signature: crate::sequencer::timeline::TimeSignature::four_four(),
            sample_rate: 48000.0,
            author: Some("Test Author".to_string()),
            description: Some("Test Description".to_string()),
            metronome_enabled: Some(true),
            metronome_volume: Some(0.5),
            loop_enabled: Some(false),
            loop_start_bars: Some(1),
            loop_end_bars: Some(8),
        };

        let json = serialize_metadata_to_json(&metadata).unwrap();
        let deserialized: crate::project::types::ProjectMetadata =
            deserialize_metadata_from_json(&json).unwrap();

        assert_eq!(deserialized.name, "Test Project");
        assert_eq!(deserialized.author, Some("Test Author".to_string()));
        assert_eq!(deserialized.tempo, 120.0);
    }

    #[test]
    fn test_pattern_conversion() {
        // Create a test pattern
        let mut pattern =
            crate::sequencer::pattern::Pattern::new_default(42, "Test Pattern".to_string());

        let note = crate::sequencer::note::Note::new(
            1,
            60,
            crate::sequencer::timeline::Position::zero(),
            48000,
            100,
        );
        pattern.add_note(note);

        // Convert to serializable
        let serializable = pattern_to_serializable(&pattern);
        assert_eq!(serializable.id, 42);
        assert_eq!(serializable.name, "Test Pattern");
        assert_eq!(serializable.notes.len(), 1);
        assert_eq!(serializable.notes[0].pitch, 60);

        // Convert back
        let recovered_pattern = pattern_from_serializable(&serializable, 48000.0);
        assert_eq!(recovered_pattern.id, 42);
        assert_eq!(recovered_pattern.name, "Test Pattern");
        assert_eq!(recovered_pattern.note_count(), 1);
    }

    #[test]
    fn test_export_samples_empty_bank() {
        let project = crate::project::Project::default();
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path();
        let target_dir = temp_dir.path().join("export");

        let result = export_samples_to_directory(&project, source_dir, &target_dir).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_export_samples_with_bank() {
        use crate::sampler::loader::LoopMode;

        let mut project = crate::project::Project::default();

        // Create a mock sample bank
        let mut sample_bank = crate::sampler::bank::SampleBank::new("Test Bank".to_string());
        let sample_path = "test_samples/kick.wav";

        let mapping = crate::sampler::bank::SampleMapping {
            note: 36,
            sample_path: std::path::PathBuf::from(sample_path),
            name: "Kick".to_string(),
            volume: 1.0,
            pan: 0.0,
            loop_mode: LoopMode::Off,
            loop_start: 0,
            loop_end: 44100,
            reverse: false,
            pitch_offset: 0,
        };

        sample_bank.add_mapping(mapping);
        project.sample_bank = Some(sample_bank);

        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path();
        let target_dir = temp_dir.path().join("export");

        // Create source directory structure
        let source_samples_dir = source_dir.join("test_samples");
        fs::create_dir_all(&source_samples_dir).unwrap();
        let source_file = source_samples_dir.join("kick.wav");
        fs::write(&source_file, "fake audio data").unwrap();

        let result = export_samples_to_directory(&project, source_dir, &target_dir).unwrap();

        assert_eq!(result.len(), 1);
        assert!(result.contains_key("kick.wav"));
        assert_eq!(
            result.get("kick.wav"),
            Some(&std::path::PathBuf::from("samples/kick.wav"))
        );

        // Verify the file was copied
        let copied_file = target_dir.join("audio").join("samples").join("kick.wav");
        assert!(copied_file.exists());
    }
}
