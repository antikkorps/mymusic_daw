// Project manager for loading and saving projects

use crate::project::migration::{MigrationResult, ProjectMigrator};
use crate::project::serialization::*;
use crate::project::types::*;
use std::fs::File;
use std::path::Path;
use zip::{ZipArchive, ZipWriter};

/// Project error types
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("File system error: {0}")]
    FileSystemError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Invalid project structure: {0}")]
    InvalidStructure(String),

    #[error("Invalid project format version")]
    InvalidVersion,

    #[error("Missing required files in project")]
    MissingFiles,

    #[error("Invalid sample file: {0}")]
    InvalidSample(String),

    #[error("Project validation failed: {0}")]
    ValidationFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("RON error: {0}")]
    Ron(#[from] ron::Error),

    #[error("Migration error: {0}")]
    MigrationError(String),
}

/// Options for loading a project
#[derive(Debug, Clone)]
pub struct ProjectLoadOptions {
    /// Whether to validate the project structure
    pub validate: bool,
    /// Whether to load samples into memory
    pub load_samples: bool,
    /// Sample rate to use if different from project default
    pub sample_rate_override: Option<f64>,
}

impl Default for ProjectLoadOptions {
    fn default() -> Self {
        Self {
            validate: true,
            load_samples: true,
            sample_rate_override: None,
        }
    }
}

/// Project manager - handles saving/loading projects
pub struct ProjectManager {
    /// Default sample rate for projects
    default_sample_rate: f64,
}

impl ProjectManager {
    /// Create a new project manager
    pub fn new(default_sample_rate: f64) -> Self {
        Self {
            default_sample_rate,
        }
    }

    /// Create a new empty project
    pub fn create_new_project(&self, name: String) -> Project {
        let mut project = Project::default();
        project.metadata.name = name;
        project.metadata.sample_rate = self.default_sample_rate;

        // Create default track
        let default_track_id = 0;
        let default_pattern_id = crate::project::generate_pattern_id();

        // Create a default empty pattern
        let default_pattern = crate::project::types::PatternSerializable {
            id: default_pattern_id,
            name: "Default Pattern".to_string(),
            length_bars: 4,
            notes: Vec::new(),
        };
        project.patterns.insert(default_pattern_id, default_pattern);

        project.tracks.insert(
            default_track_id,
            Track {
                id: default_track_id,
                name: "Track 1".to_string(),
                pattern_id: Some(default_pattern_id),
                color: Some([100, 150, 200]), // Blue-ish
                volume: 0.8,
                pan: 0.0,
                muted: false,
                soloed: false,
                track_type: TrackType::Synth,
            },
        );

        project
    }

    /// Save project to ZIP file
    pub fn save_project<P: AsRef<Path>>(
        &self,
        project: &Project,
        project_path: P,
    ) -> Result<(), ProjectError> {
        let project_path = project_path.as_ref();
        let project_dir = project_path
            .parent()
            .ok_or_else(|| ProjectError::FileSystemError("Invalid project path".to_string()))?;

        // Create project directory and all parent directories if they don't exist
        std::fs::create_dir_all(project_dir).map_err(|e| {
            ProjectError::FileSystemError(format!("Failed to create project directory: {}", e))
        })?;

        // Create temporary project directory
        let temp_dir =
            project_dir.join(format!(".temp_{}", project.metadata.name.replace(" ", "_")));

        // Ensure temp directory exists
        std::fs::create_dir_all(&temp_dir).map_err(|e| {
            ProjectError::FileSystemError(format!("Failed to create temp directory: {}", e))
        })?;

        // Export samples to temp directory
        let _exported_samples = export_samples_to_directory(project, project_dir, &temp_dir)?;

        // Create project files in temp directory
        let manifest_path = temp_dir.join("manifest.json");
        let project_path_ron = temp_dir.join("project.ron");

        // Save manifest.json
        let manifest_json = serialize_metadata_to_json(&project.metadata)?;
        std::fs::write(&manifest_path, manifest_json).map_err(|e| {
            ProjectError::FileSystemError(format!("Failed to write manifest: {}", e))
        })?;

        // Save project.ron
        let project_ron = serialize_to_ron(project)?;
        std::fs::write(&project_path_ron, project_ron).map_err(|e| {
            ProjectError::FileSystemError(format!("Failed to write project: {}", e))
        })?;

        // Save tracks as individual JSON files
        let tracks_dir = temp_dir.join("tracks");
        std::fs::create_dir_all(&tracks_dir).map_err(|e| {
            ProjectError::FileSystemError(format!("Failed to create tracks directory: {}", e))
        })?;

        for (track_id, track) in &project.tracks {
            let track_file = tracks_dir.join(format!("{}.json", track_id));
            let track_json = serde_json::to_string_pretty(track).map_err(|e| {
                ProjectError::SerializationError(format!(
                    "Failed to serialize track {}: {}",
                    track_id, e
                ))
            })?;

            std::fs::write(&track_file, track_json).map_err(|e| {
                ProjectError::FileSystemError(format!("Failed to write track {}: {}", track_id, e))
            })?;
        }

        // Create ZIP file
        let zip_file = File::create(project_path).map_err(|e| {
            ProjectError::FileSystemError(format!("Failed to create ZIP file: {}", e))
        })?;

        let mut zip_writer = ZipWriter::new(zip_file);
        add_directory_to_zip(&mut zip_writer, &temp_dir, "")?;

        // Finish ZIP
        zip_writer.finish().map_err(ProjectError::Zip)?;

        // Clean up temp directory
        std::fs::remove_dir_all(&temp_dir).map_err(|e| {
            ProjectError::FileSystemError(format!("Failed to clean up temp directory: {}", e))
        })?;

        Ok(())
    }

    /// Load project from ZIP file
    pub fn load_project<P: AsRef<Path>>(
        &self,
        project_path: P,
        options: &ProjectLoadOptions,
    ) -> Result<Project, ProjectError> {
        let project_path = project_path.as_ref();

        // Open ZIP file
        let zip_file = File::open(project_path).map_err(|e| {
            ProjectError::FileSystemError(format!("Failed to open project file: {}", e))
        })?;

        let mut zip_archive = ZipArchive::new(zip_file).map_err(ProjectError::Zip)?;

        // Extract to temporary directory
        let temp_dir = std::env::temp_dir().join(format!("project_extract_{}", std::process::id()));
        zip_archive.extract(&temp_dir).map_err(ProjectError::Zip)?;

        // Load manifest.json
        let manifest_path = temp_dir.join("manifest.json");
        if !manifest_path.exists() {
            return Err(ProjectError::MissingFiles);
        }

        let manifest_json = std::fs::read_to_string(&manifest_path).map_err(|e| {
            ProjectError::FileSystemError(format!("Failed to read manifest: {}", e))
        })?;

        let metadata = deserialize_metadata_from_json(&manifest_json)?;

        // Load project.ron
        let project_ron_path = temp_dir.join("project.ron");
        if !project_ron_path.exists() {
            return Err(ProjectError::MissingFiles);
        }

        let project_ron = std::fs::read_to_string(&project_ron_path)
            .map_err(|e| ProjectError::FileSystemError(format!("Failed to read project: {}", e)))?;

        let project = deserialize_from_ron(&project_ron)?;

        // Check version compatibility and migrate if needed
        let project_version = project.metadata.version.clone();
        let compatibility = ProjectMigrator::check_compatibility(project_version)?;

        if !compatibility.can_load {
            return Err(ProjectError::InvalidVersion);
        }

        // Perform migration if needed
        let migration_result = if compatibility.needs_migration {
            // Create backup before migration
            let backup_path = ProjectMigrator::create_backup(&project, project_path)?;
            eprintln!("Created backup at: {:?}", backup_path);

            // Perform migration
            ProjectMigrator::migrate_to_current(project)?
        } else {
            MigrationResult {
                project,
                migrated: false,
                messages: vec!["No migration needed".to_string()],
            }
        };

        let mut project = migration_result.project;

        // Log migration messages if any
        if migration_result.migrated {
            for message in &migration_result.messages {
                eprintln!("Migration: {}", message);
            }
        }

        // Update project metadata (keep loaded metadata)
        project.metadata = metadata;

        // Apply sample rate override if specified
        if let Some(override_rate) = options.sample_rate_override {
            project.metadata.sample_rate = override_rate;
        }

        // Validate project structure if requested
        if options.validate {
            crate::project::validate_project_structure(&project)
                .map_err(|e| ProjectError::ValidationFailed(e.to_string()))?;
        }

        // Clean up temp directory
        std::fs::remove_dir_all(&temp_dir).map_err(|e| {
            ProjectError::FileSystemError(format!("Failed to clean up temp directory: {}", e))
        })?;

        Ok(project)
    }

    /// Get the default sample rate
    pub fn default_sample_rate(&self) -> f64 {
        self.default_sample_rate
    }
}

/// Helper function to add directory contents to ZIP
fn add_directory_to_zip<P: AsRef<Path>>(
    zip_writer: &mut ZipWriter<File>,
    dir_path: P,
    base_path: &str,
) -> Result<(), ProjectError> {
    use walkdir::WalkDir;

    let dir_path = dir_path.as_ref();

    for entry in WalkDir::new(dir_path) {
        let entry = entry.map_err(|e| {
            ProjectError::FileSystemError(format!("Failed to walk directory: {}", e))
        })?;

        let path = entry.path();
        if path.is_file() {
            let file_name = path.strip_prefix(dir_path).map_err(|e| {
                ProjectError::FileSystemError(format!("Failed to get relative path: {}", e))
            })?;

            let zip_path = if base_path.is_empty() {
                format!("{}", file_name.display())
            } else {
                format!("{}/{}", base_path, file_name.display())
            };

            let file = File::open(path).map_err(|e| {
                ProjectError::FileSystemError(format!("Failed to open file for ZIP: {}", e))
            })?;

            let options: zip::write::FileOptions<()> = zip::write::FileOptions::default();
            zip_writer.start_file(&*zip_path, options)?;

            let mut file_reader = std::io::BufReader::new(file);
            std::io::copy(&mut file_reader, zip_writer).map_err(|e| {
                ProjectError::FileSystemError(format!("Failed to write to ZIP: {}", e))
            })?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_project_manager_creation() {
        let manager = ProjectManager::new(48000.0);
        let project = manager.create_new_project("Test Project".to_string());

        assert_eq!(project.metadata.name, "Test Project");
        assert_eq!(project.metadata.sample_rate, 48000.0);
        assert_eq!(project.tracks.len(), 1);

        // Verify default track
        let track = project.tracks.get(&0).unwrap();
        assert_eq!(track.name, "Track 1");
        assert!(track.pattern_id.is_some());
        assert_eq!(track.track_type, crate::project::types::TrackType::Synth);
    }

    #[test]
    fn test_project_save_load_cycle() {
        let manager = ProjectManager::new(44100.0);

        // Create a test project
        let mut project = manager.create_new_project("Test Save/Load".to_string());
        project.metadata.author = Some("Test Author".to_string());
        project.metadata.description = Some("Test description".to_string());

        // Use current directory for test
        let project_path = std::env::temp_dir().join("test_project.mymusic");

        // Save project
        manager.save_project(&project, &project_path).unwrap();

        // Verify file was created
        assert!(project_path.exists());

        // Load project
        let options = ProjectLoadOptions::default();
        let loaded_project = manager.load_project(&project_path, &options).unwrap();

        // Verify loaded project
        assert_eq!(loaded_project.metadata.name, "Test Save/Load");
        assert_eq!(
            loaded_project.metadata.author,
            Some("Test Author".to_string())
        );
        assert_eq!(
            loaded_project.metadata.description,
            Some("Test description".to_string())
        );
        assert_eq!(loaded_project.metadata.sample_rate, 44100.0);
        assert_eq!(loaded_project.tracks.len(), 1);

        // Cleanup
        std::fs::remove_file(&project_path).ok();
    }

    #[test]
    fn test_project_validation() {
        let mut project = Project::default();
        project.metadata.name = "".to_string(); // Empty name should fail validation

        let validation_result = crate::project::validate_project_structure(&project);
        assert!(validation_result.is_err());
    }

    #[test]
    fn test_invalid_project_file() {
        let temp_dir = tempdir().unwrap();
        let manager = ProjectManager::new(48000.0);

        // Create a non-existent project path
        let invalid_path = temp_dir.path().join("nonexistent_project.mymusic");

        // Try to load non-existent project
        let options = ProjectLoadOptions::default();
        let result = manager.load_project(&invalid_path, &options);
        assert!(result.is_err());
    }

    #[test]
    fn test_project_version_compatibility() {
        let manager = ProjectManager::new(48000.0);

        // Create a project
        let project = manager.create_new_project("Version Test".to_string());
        let project_path = std::env::temp_dir().join("version_test.mymusic");

        // Save project
        manager.save_project(&project, &project_path).unwrap();

        // Load project - should work fine
        let options = ProjectLoadOptions::default();
        let result = manager.load_project(&project_path, &options);
        assert!(result.is_ok()); // Current project should load fine

        // Test that version check exists in the code
        let loaded_project = result.unwrap();
        assert_eq!(
            loaded_project.metadata.version.major,
            ProjectVersion::current().major
        ); // Current version

        // Verify version compatibility check works (current version should load)
        assert!(loaded_project.metadata.version.major <= ProjectVersion::current().major);

        // Cleanup
        std::fs::remove_file(&project_path).ok();
    }

    #[test]
    fn test_project_with_pattern() {
        let manager = ProjectManager::new(48000.0);

        // Create a project
        let mut project = manager.create_new_project("Pattern Test".to_string());

        // Add a test pattern (in addition to the default one)
        let pattern_id = crate::project::generate_pattern_id();
        let pattern =
            crate::sequencer::pattern::Pattern::new(pattern_id, "Test Pattern".to_string(), 4);

        // Convert pattern to serializable form
        let serializable_pattern = crate::project::serialization::pattern_to_serializable(&pattern);
        project.patterns.insert(pattern_id, serializable_pattern);

        // Update track to reference this new pattern instead of default
        if let Some(track) = project.tracks.get_mut(&0) {
            track.pattern_id = Some(pattern_id);
        }

        // Use current directory for test
        let project_path = std::env::temp_dir().join("pattern_test.mymusic");

        // Save and load
        manager.save_project(&project, &project_path).unwrap();
        let options = ProjectLoadOptions::default();
        let loaded_project = manager.load_project(&project_path, &options).unwrap();

        assert_eq!(loaded_project.patterns.len(), 2); // Default + new pattern
        assert!(loaded_project.tracks.get(&0).unwrap().pattern_id.is_some());

        // Cleanup
        std::fs::remove_file(&project_path).ok();
    }

    #[test]
    fn test_synth_params_serialization() {
        let manager = ProjectManager::new(48000.0);
        let project = manager.create_new_project("Synth Params Test".to_string());

        // Modify some synth parameters
        let mut modified_project = project;
        modified_project.synth_params.waveform = crate::synth::oscillator::WaveformType::Saw;
        modified_project.synth_params.volume = 0.75;
        modified_project.synth_params.adsr =
            crate::synth::envelope::AdsrParams::new(0.05, 0.2, 0.8, 0.4);

        // Use current directory for test
        let project_path = std::env::temp_dir().join("synth_params_test.mymusic");

        // Save and load
        manager
            .save_project(&modified_project, &project_path)
            .unwrap();
        let options = ProjectLoadOptions::default();
        let loaded_project = manager.load_project(&project_path, &options).unwrap();

        assert_eq!(
            loaded_project.synth_params.waveform,
            crate::synth::oscillator::WaveformType::Saw
        );
        assert_eq!(loaded_project.synth_params.volume, 0.75);
        assert_eq!(loaded_project.synth_params.adsr.attack, 0.05);

        // Cleanup
        std::fs::remove_file(&project_path).ok();
    }

    #[test]
    fn test_project_load_options() {
        let temp_dir = tempdir().unwrap();
        let manager = ProjectManager::new(48000.0);

        let project = manager.create_new_project("Options Test".to_string());
        let project_path = temp_dir.path().join("options_test.mymusic");

        // Ensure parent directory exists
        if let Some(parent) = project_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }

        manager.save_project(&project, &project_path).unwrap();

        // Test with validation disabled
        let options_no_validate = ProjectLoadOptions {
            validate: false,
            load_samples: false,
            sample_rate_override: Some(96000.0),
        };

        let loaded_project = manager
            .load_project(&project_path, &options_no_validate)
            .unwrap();
        assert_eq!(loaded_project.metadata.sample_rate, 96000.0); // Should use override
    }
}
