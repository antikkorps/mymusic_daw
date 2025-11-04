// Project format migration system
// Handles version upgrades and backward compatibility

use crate::project::{Project, ProjectError, ProjectMetadata, ProjectVersion};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Migration result
#[derive(Debug, Clone)]
pub struct MigrationResult {
    /// Migrated project
    pub project: Project,
    /// Whether migration was performed
    pub migrated: bool,
    /// Migration messages/warnings
    pub messages: Vec<String>,
}

/// Project format migrator
pub struct ProjectMigrator;

impl ProjectMigrator {
    /// Migrate project to current version
    pub fn migrate_to_current(
        mut project: Project,
    ) -> Result<MigrationResult, crate::project::ProjectError> {
        let mut messages = Vec::new();
        let mut migrated = false;

        let current_version = ProjectVersion::current();
        let project_version = project.metadata.version.clone();

        // Check if migration is needed
        if project_version == current_version {
            return Ok(MigrationResult {
                project,
                migrated: false,
                messages: vec!["Project is already at current version".to_string()],
            });
        }

        // Perform version-by-version migrations
        if project_version.major < 1 {
            return Err(ProjectError::InvalidVersion);
        }

        // Version 1.0 -> 1.1 migration (example)
        if project_version.major == 1 && project_version.minor < 1 {
            messages.push("Migrating from v1.0 to v1.1...".to_string());
            project = Self::migrate_1_0_to_1_1(project)?;
            migrated = true;
        }

        // Version 1.1 -> 1.2 migration (example)
        if project_version.major == 1 && project_version.minor < 2 {
            messages.push("Migrating from v1.1 to v1.2...".to_string());
            project = Self::migrate_1_1_to_1_2(project)?;
            migrated = true;
        }

        // Update version to current
        project.metadata.version = current_version.clone();

        if migrated {
            messages.push(format!("Successfully migrated to v{}", current_version));
        }

        Ok(MigrationResult {
            project,
            migrated,
            messages,
        })
    }

    /// Check if project can be loaded (compatibility check)
    pub fn check_compatibility(
        version: ProjectVersion,
    ) -> Result<CompatibilityInfo, crate::project::ProjectError> {
        let current = ProjectVersion::current();

        // Major version too new - cannot load
        if version.major > current.major {
            return Ok(CompatibilityInfo {
                can_load: false,
                needs_migration: false,
                warning: Some(format!(
                    "Project version v{}.{}.{} is newer than current v{}.{}.{}",
                    version.major,
                    version.minor,
                    version.patch,
                    current.major,
                    current.minor,
                    current.patch
                )),
            });
        }

        // Same version - fully compatible
        if version == current {
            return Ok(CompatibilityInfo {
                can_load: true,
                needs_migration: false,
                warning: None,
            });
        }

        // Older version - can load with migration
        Ok(CompatibilityInfo {
            can_load: true,
            needs_migration: true,
            warning: Some(format!(
                "Project version v{}.{}.{} will be migrated to v{}.{}.{}",
                version.major,
                version.minor,
                version.patch,
                current.major,
                current.minor,
                current.patch
            )),
        })
    }

    /// Migrate from v1.0 to v1.1
    /// Example: Add default metronome settings
    fn migrate_1_0_to_1_1(mut project: Project) -> Result<Project, crate::project::ProjectError> {
        // v1.0 didn't have metronome settings in metadata
        // Add default values
        project.metadata.metronome_enabled = Some(true);
        project.metadata.metronome_volume = Some(0.5);

        Ok(project)
    }

    /// Migrate from v1.1 to v1.2
    /// Example: Add default loop settings
    fn migrate_1_1_to_1_2(mut project: Project) -> Result<Project, crate::project::ProjectError> {
        // v1.1 didn't have loop settings in metadata
        // Add default values
        project.metadata.loop_enabled = Some(false);
        project.metadata.loop_start_bars = Some(1);
        project.metadata.loop_end_bars = Some(8);

        Ok(project)
    }

    /// Create backup of project before migration
    pub fn create_backup(
        _project: &Project,
        path: &std::path::Path,
    ) -> Result<std::path::PathBuf, crate::project::ProjectError> {
        use std::fs;

        let backup_path = path.with_extension("mymusic.backup");

        // Read original project file
        let original_data = fs::read(path).map_err(|e| {
            ProjectError::FileSystemError(format!("Failed to read project for backup: {}", e))
        })?;

        // Write backup
        fs::write(&backup_path, original_data).map_err(|e| {
            ProjectError::FileSystemError(format!("Failed to create backup: {}", e))
        })?;

        Ok(backup_path)
    }
}

/// Compatibility information for project versions
#[derive(Debug, Clone)]
pub struct CompatibilityInfo {
    /// Whether the project can be loaded
    pub can_load: bool,
    /// Whether migration is needed
    pub needs_migration: bool,
    /// Optional warning message
    pub warning: Option<String>,
}

/// Legacy project format for v1.0 (example)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyProjectV1_0 {
    pub metadata: LegacyMetadataV1_0,
    pub tracks: HashMap<u32, crate::project::Track>,
    pub synth_params: crate::project::SynthParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyMetadataV1_0 {
    pub name: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sample_rate: f64,
    pub tempo: f64,
    pub time_signature: crate::sequencer::timeline::TimeSignature,
    // Note: v1.0 doesn't have version field
}

impl From<LegacyProjectV1_0> for Project {
    fn from(legacy: LegacyProjectV1_0) -> Self {
        Self {
            metadata: ProjectMetadata {
                name: legacy.metadata.name,
                author: legacy.metadata.author,
                description: legacy.metadata.description,
                created: legacy
                    .metadata
                    .created_at
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
                modified: legacy
                    .metadata
                    .modified_at
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
                sample_rate: legacy.metadata.sample_rate,
                tempo: legacy.metadata.tempo,
                time_signature: legacy.metadata.time_signature,
                version: ProjectVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                metronome_enabled: Some(true), // Default for migrated projects
                metronome_volume: Some(0.5),
                loop_enabled: Some(false),
                loop_start_bars: Some(1),
                loop_end_bars: Some(8),
            },
            tracks: legacy.tracks,
            patterns: HashMap::new(), // Will be populated during migration
            synth_params: legacy.synth_params,
            sample_bank: None, // Default for migrated projects
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_compatibility_check() {
        let current = ProjectVersion::current();

        // Same version should be compatible
        let info = ProjectMigrator::check_compatibility(current.clone()).unwrap();
        assert!(info.can_load);
        assert!(!info.needs_migration);
        assert!(info.warning.is_none());

        // Older version should need migration
        let older = ProjectVersion {
            major: 1,
            minor: 0,
            patch: 0,
        };
        let info = ProjectMigrator::check_compatibility(older).unwrap();
        assert!(info.can_load);
        assert!(info.needs_migration);
        assert!(info.warning.is_some());

        // Newer version should not be loadable
        let newer = ProjectVersion {
            major: current.major + 1,
            minor: 0,
            patch: 0,
        };
        let info = ProjectMigrator::check_compatibility(newer).unwrap();
        assert!(!info.can_load);
        assert!(!info.needs_migration);
        assert!(info.warning.is_some());
    }

    #[test]
    fn test_migration_1_0_to_1_1() {
        // Create a v1.0 project manually (without the new fields)
        let mut project = Project::default();
        project.metadata.version = ProjectVersion {
            major: 1,
            minor: 0,
            patch: 0,
        };

        // Manually remove the newer fields to simulate v1.0
        project.metadata.metronome_enabled = None;
        project.metadata.metronome_volume = None;
        project.metadata.loop_enabled = None;
        project.metadata.loop_start_bars = None;
        project.metadata.loop_end_bars = None;

        // Ensure metronome settings are not present
        assert!(project.metadata.metronome_enabled.is_none());
        assert!(project.metadata.metronome_volume.is_none());

        let result = ProjectMigrator::migrate_to_current(project).unwrap();

        assert!(result.migrated);
        assert!(result.project.metadata.metronome_enabled.is_some());
        assert_eq!(result.project.metadata.metronome_enabled, Some(true));
        assert_eq!(result.project.metadata.metronome_volume, Some(0.5));
    }

    #[test]
    fn test_no_migration_needed() {
        let project = Project::default();
        let current_version = ProjectVersion::current();

        let result = ProjectMigrator::migrate_to_current(project).unwrap();

        assert!(!result.migrated);
        assert_eq!(result.project.metadata.version, current_version);
    }
}
