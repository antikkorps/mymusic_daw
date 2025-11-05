use crate::plugin::parameters::*;
use crate::plugin::{PluginError, PluginResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Plugin cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub file_path: PathBuf,
    pub last_modified: u64,
    pub descriptor: PluginDescriptor,
}

/// Plugin scanner for discovering and caching CLAP plugins
pub struct PluginScanner {
    cache_path: PathBuf,
    cache: HashMap<String, CacheEntry>,
    blacklist: Vec<String>,
}

impl PluginScanner {
    /// Create a new plugin scanner
    pub fn new(cache_path: PathBuf) -> Self {
        let cache = Self::load_cache(&cache_path).unwrap_or_default();

        Self {
            cache_path,
            cache,
            blacklist: Vec::new(),
        }
    }

    /// Load cache from disk
    fn load_cache(path: &Path) -> PluginResult<HashMap<String, CacheEntry>> {
        if !path.exists() {
            return Ok(HashMap::new());
        }

        let content = std::fs::read_to_string(path).map_err(PluginError::Io)?;

        serde_json::from_str(&content)
            .map_err(|_| PluginError::LoadFailed("Failed to parse cache".to_string()))
    }

    /// Save cache to disk
    fn save_cache(&self) -> PluginResult<()> {
        let content = serde_json::to_string_pretty(&self.cache)
            .map_err(|_| PluginError::LoadFailed("Failed to serialize cache".to_string()))?;

        std::fs::write(&self.cache_path, content).map_err(PluginError::Io)?;

        Ok(())
    }

    /// Scan a directory for CLAP plugins
    pub fn scan_directory(&mut self, dir_path: &Path) -> PluginResult<Vec<PluginDescriptor>> {
        let mut descriptors = Vec::new();

        if !dir_path.exists() || !dir_path.is_dir() {
            return Ok(descriptors);
        }

        let entries = std::fs::read_dir(dir_path).map_err(PluginError::Io)?;

        for entry in entries {
            let entry = entry.map_err(PluginError::Io)?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("clap")
                && let Ok(descriptor) = self.scan_file(&path)
            {
                descriptors.push(descriptor);
            }
        }

        Ok(descriptors)
    }

    /// Scan a single CLAP plugin file
    pub fn scan_file(&mut self, file_path: &Path) -> PluginResult<PluginDescriptor> {
        let file_path_str = file_path.to_string_lossy().to_string();

        // Check if file is blacklisted
        if self
            .blacklist
            .iter()
            .any(|blacklisted| file_path_str.contains(blacklisted))
        {
            return Err(PluginError::LoadFailed("Plugin is blacklisted".to_string()));
        }

        // Get file modification time
        let metadata = std::fs::metadata(file_path).map_err(PluginError::Io)?;
        let last_modified = metadata
            .modified()
            .unwrap_or(UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Check cache first
        if let Some(cached) = self.cache.get(&file_path_str)
            && cached.last_modified == last_modified
        {
            return Ok(cached.descriptor.clone());
        }

        // Load and scan the plugin
        let descriptor = self.load_plugin_descriptor(file_path)?;

        // Update cache
        let cache_entry = CacheEntry {
            file_path: file_path.to_path_buf(),
            last_modified,
            descriptor: descriptor.clone(),
        };
        self.cache.insert(file_path_str, cache_entry);

        // Save cache
        let _ = self.save_cache(); // Ignore cache save errors

        Ok(descriptor)
    }

    /// Get the actual library path for a CLAP plugin (handles macOS bundles)
    pub fn get_library_path(file_path: &Path) -> PathBuf {
        // Check if it's a macOS bundle (directory with Contents/MacOS/)
        if file_path.is_dir() {
            let macos_path = file_path.join("Contents/MacOS");
            if macos_path.exists() {
                // Find the executable in Contents/MacOS/
                if let Ok(entries) = std::fs::read_dir(&macos_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            return path;
                        }
                    }
                }
            }
        }

        // Not a bundle, return the original path
        file_path.to_path_buf()
    }

    /// Load plugin descriptor from CLAP file (placeholder implementation)
    fn load_plugin_descriptor(&self, file_path: &Path) -> PluginResult<PluginDescriptor> {
        // TODO: Implement actual CLAP plugin loading
        // For now, create a placeholder descriptor based on filename

        let file_stem = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let descriptor = PluginDescriptor::new(file_stem, format!("{} Plugin", file_stem))
            .with_version("1.0.0")
            .with_vendor("Unknown Vendor")
            .with_description("A CLAP plugin")
            .with_category(PluginCategory::Effect);

        Ok(descriptor)
    }

    /// Search for plugins by name
    pub fn search_by_name(&self, name: &str) -> Vec<&PluginDescriptor> {
        self.cache
            .values()
            .filter(|entry| {
                entry
                    .descriptor
                    .name
                    .to_lowercase()
                    .contains(&name.to_lowercase())
            })
            .map(|entry| &entry.descriptor)
            .collect()
    }

    /// Search for plugins by vendor
    pub fn search_by_vendor(&self, vendor: &str) -> Vec<&PluginDescriptor> {
        self.cache
            .values()
            .filter(|entry| {
                entry
                    .descriptor
                    .vendor
                    .to_lowercase()
                    .contains(&vendor.to_lowercase())
            })
            .map(|entry| &entry.descriptor)
            .collect()
    }

    /// Search for plugins by category
    pub fn search_by_category(&self, category: PluginCategory) -> Vec<&PluginDescriptor> {
        self.cache
            .values()
            .filter(|entry| entry.descriptor.category == category)
            .map(|entry| &entry.descriptor)
            .collect()
    }

    /// Get all cached plugins
    pub fn get_all_plugins(&self) -> Vec<&PluginDescriptor> {
        self.cache.values().map(|entry| &entry.descriptor).collect()
    }

    /// Add a plugin to the blacklist
    pub fn add_to_blacklist(&mut self, plugin_id: String) {
        self.blacklist.push(plugin_id);
    }

    /// Remove a plugin from the blacklist
    pub fn remove_from_blacklist(&mut self, plugin_id: &str) {
        self.blacklist.retain(|id| id != plugin_id);
    }

    /// Get the blacklist
    pub fn get_blacklist(&self) -> &[String] {
        &self.blacklist
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        let _ = self.save_cache();
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> CacheStats {
        CacheStats {
            total_plugins: self.cache.len(),
            blacklisted_plugins: self.blacklist.len(),
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_plugins: usize,
    pub blacklisted_plugins: usize,
}

/// Get default CLAP plugin search paths for the current platform
pub fn get_default_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Platform-specific paths
    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from("/Library/Audio/Plug-Ins/CLAP"));
        paths.push(
            dirs::home_dir()
                .unwrap_or_default()
                .join("Library/Audio/Plug-Ins/CLAP"),
        );
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(program_files) = std::env::var_os("ProgramFiles") {
            paths.push(
                PathBuf::from(program_files)
                    .join("Common Files")
                    .join("CLAP"),
            );
        }
        if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
            paths.push(
                PathBuf::from(program_files_x86)
                    .join("Common Files")
                    .join("CLAP"),
            );
        }
        if let Some(app_data) = dirs::data_dir() {
            paths.push(app_data.join("CLAP"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        paths.push(PathBuf::from("/usr/lib/clap"));
        paths.push(PathBuf::from("/usr/local/lib/clap"));
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".clap"));
        }
        if let Some(data_home) = dirs::data_dir() {
            paths.push(data_home.join("clap"));
        }
    }

    // Add common additional paths
    if let Ok(current_dir) = std::env::current_dir() {
        paths.push(current_dir.join("plugins"));
        paths.push(current_dir.join("clap"));
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::TempDir;

    #[test]
    fn test_scanner_creation() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("cache.json");

        let scanner = PluginScanner::new(cache_path);
        assert_eq!(scanner.get_all_plugins().len(), 0);
    }

    #[test]
    fn test_cache_operations() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("cache.json");

        let mut scanner = PluginScanner::new(cache_path.clone());

        // Create a fake plugin file
        let plugin_path = temp_dir.path().join("test.clap");
        File::create(&plugin_path).unwrap();

        // Scan the file
        let descriptor = scanner.scan_file(&plugin_path).unwrap();
        assert_eq!(descriptor.id, "test");

        // Check that it's in the cache
        assert_eq!(scanner.get_all_plugins().len(), 1);

        // Create a new scanner instance to test cache loading
        let scanner2 = PluginScanner::new(cache_path);
        assert_eq!(scanner2.get_all_plugins().len(), 1);
    }

    #[test]
    fn test_blacklist() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("cache.json");

        let mut scanner = PluginScanner::new(cache_path);

        // Add to blacklist
        scanner.add_to_blacklist("bad_plugin".to_string());
        assert_eq!(scanner.get_blacklist().len(), 1);

        // Remove from blacklist
        scanner.remove_from_blacklist("bad_plugin");
        assert_eq!(scanner.get_blacklist().len(), 0);
    }

    #[test]
    fn test_search_functionality() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("cache.json");

        let mut scanner = PluginScanner::new(cache_path);

        // Create some fake plugin files
        let plugin1_path = temp_dir.path().join("synth.clap");
        let plugin2_path = temp_dir.path().join("effect.clap");
        File::create(&plugin1_path).unwrap();
        File::create(&plugin2_path).unwrap();

        // Scan the files
        scanner.scan_file(&plugin1_path).unwrap();
        scanner.scan_file(&plugin2_path).unwrap();

        // Test search by name
        let results = scanner.search_by_name("synth");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "synth");

        // Test search by category (all will be Effect category by default)
        let effects = scanner.search_by_category(PluginCategory::Effect);
        assert_eq!(effects.len(), 2);

        let instruments = scanner.search_by_category(PluginCategory::Instrument);
        assert_eq!(instruments.len(), 0);
    }

    #[test]
    fn test_default_search_paths() {
        let paths = get_default_search_paths();
        assert!(!paths.is_empty());

        // Check that platform-specific paths are included
        #[cfg(target_os = "macos")]
        {
            assert!(
                paths
                    .iter()
                    .any(|p| p.to_string_lossy().contains("Library/Audio/Plug-Ins/CLAP"))
            );
        }
    }

    #[test]
    fn test_cache_stats() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("cache.json");

        let mut scanner = PluginScanner::new(cache_path);

        // Create a fake plugin file
        let plugin_path = temp_dir.path().join("test.clap");
        File::create(&plugin_path).unwrap();

        // Scan the file
        scanner.scan_file(&plugin_path).unwrap();

        // Add to blacklist
        scanner.add_to_blacklist("bad_plugin".to_string());

        // Check stats
        let stats = scanner.get_cache_stats();
        assert_eq!(stats.total_plugins, 1);
        assert_eq!(stats.blacklisted_plugins, 1);
    }

    #[test]
    fn test_clear_cache() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("cache.json");

        let mut scanner = PluginScanner::new(cache_path.clone());

        // Create a fake plugin file
        let plugin_path = temp_dir.path().join("test.clap");
        File::create(&plugin_path).unwrap();

        // Scan the file
        scanner.scan_file(&plugin_path).unwrap();
        assert_eq!(scanner.get_all_plugins().len(), 1);

        // Clear cache
        scanner.clear_cache();
        assert_eq!(scanner.get_all_plugins().len(), 0);

        // Check that cache file is cleared
        let scanner2 = PluginScanner::new(cache_path);
        assert_eq!(scanner2.get_all_plugins().len(), 0);
    }
}
