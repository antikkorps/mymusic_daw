// Project persistence system for MyMusic DAW
// Implements ZIP container format for saving/loading projects

pub mod manager;

use crate::sequencer::pattern::PatternId;

pub mod migration;
pub mod serialization;
pub mod types;

pub use manager::{ProjectError, ProjectLoadOptions, ProjectManager};
pub use types::{
    PatternSerializable, Project, ProjectMetadata, ProjectVersion, SynthParams, Track,
};

/// Helper function to generate unique IDs
pub fn generate_project_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// Helper function to generate unique pattern IDs
pub fn generate_pattern_id() -> PatternId {
    crate::sequencer::pattern::generate_note_id() // Reuse the existing pattern ID generator
}

/// Helper function to validate project structure
pub fn validate_project_structure(project: &Project) -> Result<(), ProjectError> {
    // Check project metadata
    if project.metadata.name.trim().is_empty() {
        return Err(ProjectError::InvalidStructure(
            "Project name cannot be empty".to_string(),
        ));
    }

    if project.metadata.name.len() > 255 {
        return Err(ProjectError::InvalidStructure(
            "Project name cannot exceed 255 characters".to_string(),
        ));
    }

    if project.metadata.version.major < 1 {
        return Err(ProjectError::InvalidStructure(
            "Invalid project version".to_string(),
        ));
    }

    // Check tempo range (more strict bounds)
    if project.metadata.tempo < 40.0 || project.metadata.tempo > 300.0 {
        return Err(ProjectError::InvalidStructure(
            "Tempo must be between 40 and 300 BPM".to_string(),
        ));
    }

    // Validate sample rate
    if project.metadata.sample_rate < 22050.0 || project.metadata.sample_rate > 192000.0 {
        return Err(ProjectError::InvalidStructure(
            "Sample rate must be between 22050 and 192000 Hz".to_string(),
        ));
    }

    // Validate time signature (more strict)
    if project.metadata.time_signature.numerator == 0
        || project.metadata.time_signature.numerator > 32
    {
        return Err(ProjectError::InvalidStructure(
            "Time signature numerator must be between 1 and 32".to_string(),
        ));
    }

    if !project
        .metadata
        .time_signature
        .denominator
        .is_power_of_two()
        || project.metadata.time_signature.denominator > 32
    {
        return Err(ProjectError::InvalidStructure(
            "Time signature denominator must be a power of 2 and ≤ 32".to_string(),
        ));
    }

    // Check that at least one track exists
    if project.tracks.is_empty() {
        return Err(ProjectError::InvalidStructure(
            "Project must have at least one track".to_string(),
        ));
    }

    // Check for duplicate pattern IDs
    let mut pattern_ids = std::collections::HashSet::new();
    for pattern_id in project.patterns.keys() {
        if pattern_ids.contains(pattern_id) {
            return Err(ProjectError::InvalidStructure(format!(
                "Duplicate pattern ID: {}",
                pattern_id
            )));
        }
        pattern_ids.insert(*pattern_id);
    }

    // Validate patterns
    for (pattern_id, pattern) in &project.patterns {
        // Check pattern name
        if pattern.name.trim().is_empty() {
            return Err(ProjectError::InvalidStructure(format!(
                "Pattern {} name cannot be empty",
                pattern_id
            )));
        }

        if pattern.name.len() > 255 {
            return Err(ProjectError::InvalidStructure(format!(
                "Pattern {} name cannot exceed 255 characters",
                pattern_id
            )));
        }

        // Check pattern length
        if pattern.length_bars == 0 || pattern.length_bars > 999 {
            return Err(ProjectError::InvalidStructure(format!(
                "Pattern {} length must be between 1 and 999 bars",
                pattern_id
            )));
        }

        // Check for duplicate note IDs within pattern
        let mut note_ids = std::collections::HashSet::new();
        for note in &pattern.notes {
            if note_ids.contains(&note.id) {
                return Err(ProjectError::InvalidStructure(format!(
                    "Duplicate note ID {} in pattern {}",
                    note.id, pattern_id
                )));
            }
            note_ids.insert(note.id);

            // Validate note properties
            if note.pitch > 127 {
                return Err(ProjectError::InvalidStructure(format!(
                    "Note pitch {} exceeds MIDI range (0-127) in pattern {}",
                    note.pitch, pattern_id
                )));
            }

            if note.velocity > 127 {
                return Err(ProjectError::InvalidStructure(format!(
                    "Note velocity {} exceeds MIDI range (0-127) in pattern {}",
                    note.velocity, pattern_id
                )));
            }

            if note.duration_samples == 0 {
                return Err(ProjectError::InvalidStructure(format!(
                    "Note duration cannot be 0 in pattern {}",
                    pattern_id
                )));
            }
        }
    }

    // Validate tracks
    for (track_id, track) in &project.tracks {
        // Check track name
        if track.name.trim().is_empty() {
            return Err(ProjectError::InvalidStructure(format!(
                "Track {} name cannot be empty",
                track_id
            )));
        }

        if track.name.len() > 255 {
            return Err(ProjectError::InvalidStructure(format!(
                "Track {} name cannot exceed 255 characters",
                track_id
            )));
        }

        // Check track volume
        if track.volume < 0.0 || track.volume > 2.0 {
            return Err(ProjectError::InvalidStructure(format!(
                "Track {} volume must be between 0.0 and 2.0",
                track_id
            )));
        }

        // Check track pan
        if track.pan < -1.0 || track.pan > 1.0 {
            return Err(ProjectError::InvalidStructure(format!(
                "Track {} pan must be between -1.0 and 1.0",
                track_id
            )));
        }

        // Check pattern validity
        if let Some(pattern_id) = track.pattern_id
            && !project.patterns.contains_key(&pattern_id)
        {
            return Err(ProjectError::InvalidStructure(format!(
                "Track {} references missing pattern {}",
                track_id, pattern_id
            )));
        }
    }

    // Validate synth parameters
    if project.synth_params.volume < 0.0 || project.synth_params.volume > 2.0 {
        return Err(ProjectError::InvalidStructure(
            "Synth volume must be between 0.0 and 2.0".to_string(),
        ));
    }

    if project.synth_params.pan < -1.0 || project.synth_params.pan > 1.0 {
        return Err(ProjectError::InvalidStructure(
            "Synth pan must be between -1.0 and 1.0".to_string(),
        ));
    }

    if project.synth_params.pan_spread < 0.0 || project.synth_params.pan_spread > 1.0 {
        return Err(ProjectError::InvalidStructure(
            "Synth pan spread must be between 0.0 and 1.0".to_string(),
        ));
    }

    // Validate ADSR parameters
    if project.synth_params.adsr.attack < 0.0 || project.synth_params.adsr.attack > 10.0 {
        return Err(ProjectError::InvalidStructure(
            "ADSR attack must be between 0.0 and 10.0 seconds".to_string(),
        ));
    }

    if project.synth_params.adsr.decay < 0.0 || project.synth_params.adsr.decay > 10.0 {
        return Err(ProjectError::InvalidStructure(
            "ADSR decay must be between 0.0 and 10.0 seconds".to_string(),
        ));
    }

    if project.synth_params.adsr.sustain < 0.0 || project.synth_params.adsr.sustain > 1.0 {
        return Err(ProjectError::InvalidStructure(
            "ADSR sustain must be between 0.0 and 1.0".to_string(),
        ));
    }

    if project.synth_params.adsr.release < 0.0 || project.synth_params.adsr.release > 10.0 {
        return Err(ProjectError::InvalidStructure(
            "ADSR release must be between 0.0 and 10.0 seconds".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_project_id() {
        let id1 = generate_project_id();
        let id2 = generate_project_id();
        assert_ne!(id1, id2);
        assert!(id1 < id2);
    }

    #[test]
    fn test_generate_pattern_id() {
        let id1 = generate_pattern_id();
        let id2 = generate_pattern_id();
        assert_ne!(id1, id2);
        assert!(id1 < id2);
    }

    #[test]
    fn test_validate_project_structure_valid() {
        let manager = crate::project::manager::ProjectManager::new(48000.0);
        let project = manager.create_new_project("Valid Test".to_string());
        let result = validate_project_structure(&project);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_project_structure_invalid_name() {
        let mut project = Project::default();
        project.metadata.name = "".to_string();
        let result = validate_project_structure(&project);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("name cannot be empty")
        );
    }

    #[test]
    fn test_validate_project_structure_invalid_tempo() {
        let mut project = Project::default();
        project.metadata.tempo = 10.0; // Too low
        let result = validate_project_structure(&project);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Tempo must be between")
        );
    }

    #[test]
    fn test_validate_project_structure_invalid_time_signature() {
        let manager = crate::project::manager::ProjectManager::new(48000.0);
        let mut project = manager.create_new_project("Invalid Time Sig Test".to_string());
        // Create invalid time signature directly (bypassing constructor validation)
        project.metadata.time_signature = crate::sequencer::timeline::TimeSignature {
            numerator: 0,
            denominator: 4,
        };
        let result = validate_project_structure(&project);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Time signature numerator must be between 1 and 32")
        );
    }

    #[test]
    fn test_validate_project_structure_no_tracks() {
        let mut project = Project::default();
        project.tracks.clear(); // Remove all tracks
        let result = validate_project_structure(&project);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must have at least one track")
        );
    }

    #[test]
    fn test_validate_project_structure_missing_pattern() {
        let manager = crate::project::manager::ProjectManager::new(48000.0);
        let mut project = manager.create_new_project("Missing Pattern Test".to_string());

        // Create a track that references a non-existent pattern
        let fake_pattern_id = generate_pattern_id();
        if let Some(track) = project.tracks.get_mut(&0) {
            track.pattern_id = Some(fake_pattern_id);
        }

        let result = validate_project_structure(&project);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("references missing pattern")
        );
    }
}
