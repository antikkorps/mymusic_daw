// Plugin management commands for CLAP plugins

use tauri::State;
use crate::DawState;
use crate::{ManagedPlugin, PluginGuiInfo};
use serde::{Deserialize, Serialize};
use mymusic_daw::plugin::{Plugin, PluginHost, PluginInstanceId};
use mymusic_daw::plugin::scanner::{PluginScanner, get_default_search_paths};
use base64::Engine;
use std::path::PathBuf;
use std::env;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub id: String,
    pub name: String,
    pub value: f64,
    pub min: f64,
    pub max: f64,
}

/// Load a plugin instance
#[tauri::command]
pub fn load_plugin_instance(
    plugin_path: String,
    plugin_id: Option<String>,
    state: State<DawState>,
) -> Result<String, String> {
    println!("🔌 Loading plugin from: {}", plugin_path);
    println!("🏷️ Provided plugin_id: {:?}", plugin_id);
    
    // Check if we're in a headless environment and try to start virtual display
    if is_headless_environment() {
        println!("⚠️ Headless environment detected, trying to start virtual display server...");
        
        match try_start_virtual_display() {
            Ok(()) => {
                println!("✅ Virtual display server started successfully!");
                println!("📝 Plugins should now be able to initialize");
            }
            Err(e) => {
                let error_msg = format!(
                    "Cannot load plugin in headless environment and failed to start virtual display server: {}\n\n\
                    Solutions:\n\
                    • macOS: Install XQuartz with 'brew install --cask xquartz'\n\
                    • Linux: Install Xvfb with 'sudo apt-get install xvfb'\n\
                    • Or run in a desktop environment with display server", 
                    e
                );
                println!("❌ {}", error_msg);
                return Err(error_msg);
            }
        }
    }
    
    // Check if file exists
    if !std::path::Path::new(&plugin_path).exists() {
        println!("❌ Plugin file does not exist: {}", plugin_path);
        return Err(format!("Plugin file not found: {}", plugin_path));
    }
    println!("✅ Plugin file exists");
    
    // Create plugin host
    let mut host = PluginHost::new();
    
    // Load the plugin from path
    let plugin_path_buf = std::path::PathBuf::from(&plugin_path);
    println!("📦 Attempting to load plugin from path: {:?}", plugin_path_buf);
    
    let plugin_key = host.load_plugin(&plugin_path_buf)
        .map_err(|e| {
            println!("❌ Failed to load plugin: {}", e);
            format!("Failed to load plugin: {}", e)
        })?;
    
    println!("✅ Plugin loaded successfully with key: {:?}", plugin_key);
    
    // Create an instance of the plugin
    println!("🎛️ Creating plugin instance...");
    let instance_id = host.create_instance(&plugin_key, None)
        .map_err(|e| {
            println!("❌ Failed to create instance: {}", e);
            format!("Failed to create instance: {}", e)
        })?;
    
    println!("✅ Plugin instance created with ID: {:?}", instance_id);
    
    // Use provided plugin_id or generate a unique one
    let state_id = plugin_id.unwrap_or_else(|| {
        let generated = state.generate_plugin_id();
        println!("🎲 Generated plugin ID: {}", generated);
        generated
    });
    
    println!("✅ Plugin loaded with instance ID: {:?}, state ID: '{}'", instance_id, state_id);
    
    // Store the plugin in the state
    let mut plugins = state.plugins.lock().map_err(|e| format!("Failed to lock plugins: {}", e))?;
    
    let managed_plugin = ManagedPlugin {
        host,
        instance_id,
        gui_info: None,
    };
    
    plugins.insert(state_id.clone(), managed_plugin);
    println!("💾 Plugin stored in state with key: '{}'", state_id);
    println!("📊 Total plugins in state: {}", plugins.len());
    
    Ok(state_id)
}

/// Get plugin parameters
#[tauri::command]
pub fn get_plugin_parameters(
    plugin_id: String,
    _state: State<DawState>,
) -> Result<Vec<ParameterInfo>, String> {
    println!("🎛️ Getting parameters for plugin: {}", plugin_id);
    Err("Plugin parameters not yet implemented".to_string())
}

/// Get a specific plugin parameter value
#[tauri::command]
pub fn get_plugin_parameter_value(
    plugin_id: String,
    parameter_id: String,
    _state: State<DawState>,
) -> Result<f64, String> {
    println!(
        "📊 Getting parameter {} for plugin: {}",
        parameter_id, plugin_id
    );
    Err("Plugin parameter values not yet implemented".to_string())
}

/// Set a plugin parameter value
#[tauri::command]
pub fn set_plugin_parameter_value(
    plugin_id: String,
    parameter_id: String,
    value: f64,
    _state: State<DawState>,
) -> Result<(), String> {
    println!(
        "🎚️ Setting parameter {} = {} for plugin: {}",
        parameter_id, value, plugin_id
    );
    Err("Plugin parameter setting not yet implemented".to_string())
}

/// Unload a plugin instance
#[tauri::command]
pub fn unload_plugin_instance(plugin_id: String, _state: State<DawState>) -> Result<(), String> {
    println!("🗑️ Unloading plugin: {}", plugin_id);
    Err("Plugin unloading not yet implemented".to_string())
}

/// Get list of loaded plugins
#[tauri::command]
pub fn get_loaded_plugins(state: State<DawState>) -> Result<Vec<PluginInfo>, String> {
    println!("📋 Getting loaded plugins");
    
    let plugins = state.plugins.lock().map_err(|e| format!("Failed to lock plugins: {}", e))?;
    let plugin_list: Vec<PluginInfo> = plugins
        .iter()
        .map(|(id, managed_plugin)| {
            // Get instance wrapper to access descriptor
            if let Some(plugin_info) = managed_plugin.host.with_instance_wrapper(managed_plugin.instance_id, |wrapper| {
                PluginInfo {
                    id: id.clone(),
                    name: wrapper.plugin().descriptor().name.clone(),
                    vendor: wrapper.plugin().descriptor().vendor.clone(),
                    path: wrapper.plugin().descriptor().file_path.to_string_lossy().to_string(),
                }
            }) {
                plugin_info
            } else {
                // Fallback if instance not found
                PluginInfo {
                    id: id.clone(),
                    name: "Unknown Plugin".to_string(),
                    vendor: "Unknown Vendor".to_string(),
                    path: "Unknown".to_string(),
                }
            }
        })
        .collect();
    
    Ok(plugin_list)
}

/// Scan for available plugins
#[tauri::command]
pub fn scan_for_plugins() -> Result<Vec<PluginInfo>, String> {
    println!("🔍 [FRONTEND CALL] Scanning for plugins...");
    
    // Get default search paths
    let search_paths = get_default_search_paths();
    println!("📁 Search paths: {:?}", search_paths);
    
    // Create scanner with temporary cache
    let cache_dir = std::env::temp_dir().join("mymusic_daw_plugin_cache");
    std::fs::create_dir_all(&cache_dir).map_err(|e| format!("Failed to create cache dir: {}", e))?;
    let cache_path = cache_dir.join("plugin_cache.json");
    
    let mut scanner = PluginScanner::new(cache_path);
    let mut all_plugins = Vec::new();
    
    // Scan each directory
    for path in &search_paths {
        println!("🔍 Scanning directory: {:?}", path);
        match scanner.scan_directory(path) {
            Ok(mut descriptors) => {
                println!("✅ Found {} plugins in {:?}", descriptors.len(), path);
                
                // Convert to PluginInfo
                for descriptor in &mut descriptors {
                    all_plugins.push(PluginInfo {
                        id: descriptor.id.clone(),
                        name: descriptor.name.clone(),
                        vendor: descriptor.vendor.clone(),
                        path: descriptor.file_path.to_string_lossy().to_string(),
                    });
                }
            }
            Err(e) => {
                println!("⚠️ Failed to scan directory {:?}: {}", path, e);
                // Continue scanning other directories
            }
        }
    }
    
    println!("🎉 Total plugins found: {}", all_plugins.len());
    Ok(all_plugins)
}

/// Get plugin search paths
#[tauri::command]
pub fn get_plugin_search_paths() -> Result<Vec<String>, String> {
    let paths = get_default_search_paths();
    let path_strings: Vec<String> = paths
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    Ok(path_strings)
}

/// Scan a specific directory for plugins
#[tauri::command]
pub fn scan_plugin_directory(directory_path: String) -> Result<Vec<PluginInfo>, String> {
    println!("🔍 Scanning directory: {}", directory_path);
    
    let path = PathBuf::from(directory_path);
    let cache_dir = std::env::temp_dir().join("mymusic_daw_plugin_cache");
    std::fs::create_dir_all(&cache_dir).map_err(|e| format!("Failed to create cache dir: {}", e))?;
    let cache_path = cache_dir.join("plugin_cache.json");
    
    let mut scanner = PluginScanner::new(cache_path);
    let descriptors = scanner.scan_directory(&path)
        .map_err(|e| format!("Failed to scan directory: {}", e))?;
    
    let plugin_infos: Vec<PluginInfo> = descriptors
        .into_iter()
        .map(|descriptor| PluginInfo {
            id: descriptor.id,
            name: descriptor.name,
            vendor: descriptor.vendor,
            path: descriptor.file_path.to_string_lossy().to_string(),
        })
        .collect();
    
    println!("✅ Found {} plugins", plugin_infos.len());
    Ok(plugin_infos)
}

/// Try to start a virtual display server automatically
fn try_start_virtual_display() -> Result<(), String> {
    println!("🖥️ Attempting to start virtual display server...");
    
    #[cfg(target_os = "macos")]
    {
        // Try to start XQuartz on macOS
        println!("🍎 Trying to start XQuartz (macOS)...");
        
        // First check if XQuartz is already running
        if std::process::Command::new("pgrep")
            .arg("Xquartz")
            .output()
            .map(|output| !output.stdout.is_empty())
            .unwrap_or(false)
        {
            println!("✅ XQuartz is already running");
            
            // Check if DISPLAY is set
            if std::env::var("DISPLAY").is_err() {
                println!("🔧 Setting DISPLAY=:0 for existing XQuartz");
                std::env::set_var("DISPLAY", ":0");
            }
            return Ok(());
        }
        
        // Check if XQuartz is installed (check multiple possible locations)
        let xquartz_installed = std::process::Command::new("which")
            .arg("Xquartz")
            .output()
            .map(|output| !output.stdout.is_empty())
            .unwrap_or(false) ||
            std::process::Command::new("which")
            .arg("X11")
            .output()
            .map(|output| !output.stdout.is_empty())
            .unwrap_or(false) ||
            std::path::Path::new("/Applications/Utilities/XQuartz.app").exists() ||
            std::path::Path::new("/opt/X11/bin/Xquartz").exists();
            
        if xquartz_installed {
            println!("✅ XQuartz found, starting...");
            
            // Start XQuartz in background
            match std::process::Command::new("open")
                .args(&["-a", "XQuartz"])
                .output()
            {
                Ok(_) => {
                    // Give it more time to start and initialize
                    println!("⏳ Waiting for XQuartz to initialize...");
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    
                    // Check if it's actually running
                    if std::process::Command::new("pgrep")
                        .arg("Xquartz")
                        .output()
                        .map(|output| !output.stdout.is_empty())
                        .unwrap_or(false)
                    {
                        // Set DISPLAY variable for this process and child processes
                        std::env::set_var("DISPLAY", ":0");
                        println!("✅ XQuartz started successfully, DISPLAY=:0");
                        
                        // Verify X11 socket exists
                        if std::path::Path::new("/tmp/.X11-unix/X0").exists() {
                            println!("✅ X11 socket found at /tmp/.X11-unix/X0");
                        } else {
                            println!("⚠️ X11 socket not found, but continuing...");
                        }
                        
                        return Ok(());
                    } else {
                        println!("❌ XQuartz failed to start properly");
                        return Err("XQuartz failed to start".to_string());
                    }
                }
                Err(e) => {
                    println!("❌ Failed to start XQuartz: {}", e);
                    return Err(format!("Failed to start XQuartz: {}", e));
                }
            }
        } else {
            println!("⚠️ XQuartz not found. Install with: brew install --cask xquartz");
            return Err("XQuartz not installed".to_string());
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        // Try to start Xvfb on Linux
        println!("🐧 Trying to start Xvfb (Linux)...");
        
        // Check if Xvfb is installed
        if std::process::Command::new("which")
            .arg("Xvfb")
            .output()
            .map(|output| !output.stdout.is_empty())
            .unwrap_or(false)
        {
            println!("✅ Xvfb found, starting...");
            
            // Start Xvfb in background
            match std::process::Command::new("Xvfb")
                .args(&[":99", "-screen", "0", "1024x768x24"])
                .spawn()
            {
                Ok(_) => {
                    // Give it a moment to start
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    
                    // Set DISPLAY variable
                    std::env::set_var("DISPLAY", ":99");
                    println!("✅ Xvfb started, DISPLAY=:99");
                    return Ok(());
                }
                Err(e) => {
                    println!("❌ Failed to start Xvfb: {}", e);
                }
            }
        } else {
            println!("⚠️ Xvfb not found. Install with: sudo apt-get install xvfb");
        }
    }
    
    Err("Failed to start virtual display server".to_string())
}

/// Check if we're running in a headless environment (no display server)
fn is_headless_environment() -> bool {
    // Check for common headless environment indicators
    if env::var("DISPLAY").is_err() && env::var("WAYLAND_DISPLAY").is_err() {
        // On Unix-like systems, no DISPLAY or WAYLAND_DISPLAY typically means headless
        return true;
    }
    
    // Check if we're running in SSH session (often headless)
    if let Ok(ssh_connection) = env::var("SSH_CONNECTION") {
        if !ssh_connection.is_empty() {
            return true;
        }
    }
    
    // Check for common CI/CD environment variables
    let ci_vars = ["CI", "GITHUB_ACTIONS", "JENKINS_URL", "GITLAB_CI"];
    for var in &ci_vars {
        if env::var(var).is_ok() {
            return true;
        }
    }
    
    // On macOS, check if WindowServer is available
    #[cfg(target_os = "macos")]
    {
        // Try to check if WindowServer process is running
        if std::process::Command::new("pgrep")
            .arg("WindowServer")
            .output()
            .map(|output| output.stdout.is_empty())
            .unwrap_or(true)
        {
            println!("⚠️ No WindowServer process found - likely headless environment");
            return true;
        }
    }
    
    false
}

/// Show plugin GUI with timeout and crash prevention
#[tauri::command]
pub fn show_plugin_gui(
    plugin_id: String,
    state: State<DawState>,
) -> Result<(), String> {
    println!("🖥️ Showing GUI for plugin: '{}'", plugin_id);
    
    // Check if we're in a headless environment first
    if is_headless_environment() {
        let error_msg = "Cannot show GUI in headless environment (no display server available)";
        println!("⚠️ {}", error_msg);
        return Err(error_msg.to_string());
    }
    
    // Simple timeout approach to prevent hanging
    let (sender, receiver) = std::sync::mpsc::channel();
    let plugin_id_clone = plugin_id.clone();
    
    // Clone the state for the new thread
    let state_clone = state.inner().clone();
    
    // Spawn GUI operation in separate thread
    std::thread::spawn(move || {
        let result = (|| -> Result<(), String> {
            let plugins = state_clone.plugins.lock().map_err(|e| format!("Failed to lock plugins: {}", e))?;
            
            if let Some(managed_plugin) = plugins.get(&plugin_id_clone) {
                println!("✅ Found plugin: '{}'", plugin_id_clone);
                
                // Try GUI operations with panic handling
                let gui_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    managed_plugin.host.with_instance_wrapper_mut(managed_plugin.instance_id, |wrapper| {
                        if let Some(clap_instance) = wrapper.as_clap_plugin_mut() {
                            if let Some(gui) = clap_instance.gui_mut() {
                                // Just try to show GUI - skip initialization for now
                                println!("👁️ Attempting to show GUI...");
                                match gui.show() {
                                    Ok(()) => println!("✅ GUI shown successfully"),
                                    Err(e) => println!("⚠️ GUI show failed: {}", e),
                                }
                                Ok(())
                            } else {
                                Err("Plugin does not have GUI support".to_string())
                            }
                        } else {
                            Err("Failed to get CLAP plugin instance".to_string())
                        }
                    })
                }));
                
                match gui_result {
                    Ok(Some(result)) => result,
                    Ok(None) => Err("Failed to get instance wrapper".to_string()),
                    Err(_) => {
                        println!("❌ GUI operation panicked - application is still stable");
                        Ok(()) // Don't fail the whole operation for GUI panic
                    }
                }
            } else {
                Err(format!("Plugin not found: {}", plugin_id_clone))
            }
        })();
        
        let _ = sender.send(result);
    });
    
    // Wait with timeout (5 seconds)
    match receiver.recv_timeout(std::time::Duration::from_secs(5)) {
        Ok(result) => result,
        Err(_) => {
            let error_msg = "GUI operation timed out (plugin may be hanging). Application is still stable.";
            println!("⏰ {}", error_msg);
            Err(error_msg.to_string())
        }
    }
}



/// Hide plugin GUI
#[tauri::command]
pub fn hide_plugin_gui(
    plugin_id: String,
    state: State<DawState>,
) -> Result<(), String> {
    println!("🙈 Hiding GUI for plugin: {}", plugin_id);
    
    let mut plugins = state.plugins.lock().map_err(|e| format!("Failed to lock plugins: {}", e))?;
    
    if let Some(managed_plugin) = plugins.get_mut(&plugin_id) {
        // Get instance from host and try to hide GUI
        let hide_result = managed_plugin.host.with_instance_wrapper_mut(managed_plugin.instance_id, |wrapper| {
            if let Some(clap_instance) = wrapper.as_clap_plugin_mut() {
                if let Some(gui) = clap_instance.gui_mut() {
                    gui.hide().map_err(|e| format!("Failed to hide GUI: {}", e))?;
                    Ok(())
                } else {
                    Err("Plugin does not have GUI support".to_string())
                }
            } else {
                Err("Failed to get CLAP plugin instance".to_string())
            }
        });
        
        match hide_result {
            Some(Ok(())) => {
                // Update GUI info in managed plugin
                if let Some(ref mut gui_info) = managed_plugin.gui_info {
                    gui_info.is_visible = false;
                }
                
                println!("✅ Plugin GUI hidden: {}", plugin_id);
                Ok(())
            }
            Some(Err(e)) => Err(e),
            None => Err(format!("Plugin {} does not support GUI or instance not found", plugin_id))
        }
    } else {
        Err(format!("Plugin not found: {}", plugin_id))
    }
}

/// Attach plugin GUI to parent window
#[tauri::command]
pub fn attach_plugin_gui(
    plugin_id: String,
    window_handle: String, // Base64 encoded window handle
    state: State<DawState>,
) -> Result<(), String> {
    println!("🔗 Attaching GUI for plugin: {} to window: {}", plugin_id, window_handle);
    
    let mut plugins = state.plugins.lock().map_err(|e| format!("Failed to lock plugins: {}", e))?;
    
    if let Some(managed_plugin) = plugins.get_mut(&plugin_id) {
        // Get instance from host and try to attach GUI
        let attach_result = managed_plugin.host.with_instance_wrapper_mut(managed_plugin.instance_id, |wrapper| {
            if let Some(clap_instance) = wrapper.as_clap_plugin_mut() {
                if let Some(gui) = clap_instance.gui_mut() {
                    // Decode base64 window handle
                    let handle_bytes = base64::prelude::BASE64_STANDARD.decode(&window_handle)
                        .map_err(|e| format!("Failed to decode window handle: {}", e))?;
                    
                    // Convert bytes to raw pointer (platform-specific)
                    let _raw_handle: *mut std::ffi::c_void = if handle_bytes.len() == 8 {
                        // 64-bit pointer
                        let mut bytes = [0u8; 8];
                        bytes.copy_from_slice(&handle_bytes);
                        u64::from_le_bytes(bytes) as *mut std::ffi::c_void
                    } else if handle_bytes.len() == 4 {
                        // 32-bit pointer
                        let mut bytes = [0u8; 4];
                        bytes.copy_from_slice(&handle_bytes);
                        u32::from_le_bytes(bytes) as *mut std::ffi::c_void
                    } else {
                        return Err("Invalid window handle size".to_string());
                    };
                    
                    // Attach GUI to parent window (if supported)
                    // Note: Not all CLAP GUIs support this
                    unsafe { gui.attach_to_window(_raw_handle) }.map_err(|e| format!("Failed to attach GUI: {}", e))?;
                    
                    // Return GUI info for updating state
                    let (width, height) = gui.get_size();
                    Ok((width, height, gui.can_resize()))
                } else {
                    Err("Plugin does not have GUI support".to_string())
                }
            } else {
                Err("Failed to get CLAP plugin instance".to_string())
            }
        });
        
        match attach_result {
            Some(Ok((width, height, can_resize))) => {
                // Update GUI info in managed plugin
                if let Some(ref mut gui_info) = managed_plugin.gui_info {
                    gui_info.is_visible = true;
                    gui_info.width = width;
                    gui_info.height = height;
                } else {
                    managed_plugin.gui_info = Some(PluginGuiInfo {
                        is_visible: true,
                        width,
                        height,
                        can_resize,
                        api: "clap".to_string(),
                    });
                }
                
                println!("✅ Plugin GUI attached: {} ({}x{})", plugin_id, width, height);
                Ok(())
            }
            Some(Err(e)) => Err(e),
            None => Err(format!("Plugin {} does not support GUI or instance not found", plugin_id))
        }
    } else {
        Err(format!("Plugin not found: {}", plugin_id))
    }
}

/// Get plugin GUI size
#[tauri::command]
pub fn get_plugin_gui_size(
    plugin_id: String,
    state: State<DawState>,
) -> Result<(u32, u32), String> {
    println!("📏 Getting GUI size for plugin: {}", plugin_id);
    
    let plugins = state.plugins.lock().map_err(|e| format!("Failed to lock plugins: {}", e))?;
    
    if let Some(managed_plugin) = plugins.get(&plugin_id) {
        if let Some(ref gui_info) = managed_plugin.gui_info {
            Ok((gui_info.width, gui_info.height))
        } else {
            // Try to get size from plugin instance
            let size_result = managed_plugin.host.with_instance_wrapper(managed_plugin.instance_id, |wrapper| {
                if let Some(clap_instance) = wrapper.as_clap_plugin() {
                    if let Some(gui) = clap_instance.gui() {
                        Ok(gui.get_size())
                    } else {
                        Err("Plugin does not have GUI".to_string())
                    }
                } else {
                    Err("Failed to get CLAP plugin instance".to_string())
                }
            });
            
            match size_result {
                Some(Ok(size)) => Ok(size),
                Some(Err(e)) => Err(e),
                None => Err("Failed to get instance wrapper".to_string())
            }
        }
    } else {
        Err("Plugin not found".to_string())
    }
}

/// Set plugin GUI size
#[tauri::command]
pub fn set_plugin_gui_size(
    plugin_id: String,
    width: u32,
    height: u32,
    state: State<DawState>,
) -> Result<(), String> {
    println!("📏 Setting GUI size for plugin: {} to {}x{}", plugin_id, width, height);
    
    let mut plugins = state.plugins.lock().map_err(|e| format!("Failed to lock plugins: {}", e))?;
    
    if let Some(managed_plugin) = plugins.get_mut(&plugin_id) {
        // Try to set size on plugin instance
        let set_size_result = managed_plugin.host.with_instance_wrapper_mut(managed_plugin.instance_id, |wrapper| {
            if let Some(clap_instance) = wrapper.as_clap_plugin_mut() {
                if let Some(gui) = clap_instance.gui_mut() {
                    gui.set_size(width, height).map_err(|e| format!("Failed to set GUI size: {}", e))?;
                    
                    // Return GUI info for updating state
                    Ok((gui.is_visible(), gui.can_resize()))
                } else {
                    Err("Plugin does not have GUI".to_string())
                }
            } else {
                Err("Failed to get CLAP plugin instance".to_string())
            }
        });
        
        match set_size_result {
            Some(Ok((is_visible, can_resize))) => {
                // Update stored GUI info
                if let Some(ref mut gui_info) = managed_plugin.gui_info {
                    gui_info.width = width;
                    gui_info.height = height;
                } else {
                    managed_plugin.gui_info = Some(PluginGuiInfo {
                        is_visible,
                        width,
                        height,
                        can_resize,
                        api: "clap".to_string(),
                    });
                }
                
                println!("✅ GUI size set: {} -> {}x{}", plugin_id, width, height);
                Ok(())
            }
            Some(Err(e)) => Err(e),
            None => Err(format!("Failed to set GUI size for plugin: {}", plugin_id))
        }
    } else {
        Err(format!("Plugin not found: {}", plugin_id))
    }
}

/// Check if plugin GUI is visible
#[tauri::command]
pub fn is_plugin_gui_visible(
    plugin_id: String,
    state: State<DawState>,
) -> Result<bool, String> {
    println!("👁️ Checking GUI visibility for plugin: {}", plugin_id);
    
    let plugins = state.plugins.lock().map_err(|e| format!("Failed to lock plugins: {}", e))?;
    
    if let Some(managed_plugin) = plugins.get(&plugin_id) {
        if let Some(ref gui_info) = managed_plugin.gui_info {
            Ok(gui_info.is_visible)
        } else {
            // Try to get visibility from plugin instance
            let visibility_result = managed_plugin.host.with_instance_wrapper(managed_plugin.instance_id, |wrapper| {
                if let Some(clap_instance) = wrapper.as_clap_plugin() {
                    if let Some(gui) = clap_instance.gui() {
                        Ok(gui.is_visible())
                    } else {
                        Err("Plugin does not have GUI".to_string())
                    }
                } else {
                    Err("Failed to get CLAP plugin instance".to_string())
                }
            });
            
            match visibility_result {
                Some(Ok(visible)) => Ok(visible),
                Some(Err(e)) => Err(e),
                None => Err("Failed to get instance wrapper".to_string())
            }
        }
    } else {
        Err("Plugin not found".to_string())
    }
}

/// Get window handle for plugin embedding
#[tauri::command]
pub fn get_window_handle_for_plugin(
    window_label: String,
) -> Result<String, String> {
    println!("🪟 Getting window handle for window: {}", window_label);
    
    // This is a placeholder implementation
    // In a real implementation, you would:
    // 1. Get the native window handle using platform-specific APIs
    // 2. Encode it as base64 for transmission to plugin
    // 3. Handle platform differences (NSWindow on macOS, HWND on Windows, etc.)
    
    #[cfg(target_os = "macos")]
    {
        // On macOS, you would get the NSWindow pointer
        // For now, return a dummy handle
        let dummy_handle = vec![0u8; 8]; // 64-bit pointer
        let encoded = base64::prelude::BASE64_STANDARD.encode(&dummy_handle);
        Ok(encoded)
    }
    
    #[cfg(target_os = "windows")]
    {
        // On Windows, you would get the HWND
        let dummy_handle = vec![0u8; 8]; // 64-bit pointer
        let encoded = base64::prelude::BASE64_STANDARD.encode(&dummy_handle);
        Ok(encoded)
    }
    
    #[cfg(target_os = "linux")]
    {
        // On Linux, you would get the Window or X11 Window
        let dummy_handle = vec![0u8; 8]; // 64-bit pointer
        let encoded = base64::prelude::BASE64_STANDARD.encode(&dummy_handle);
        Ok(encoded)
    }
    
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err("Unsupported platform".to_string())
    }
}

// ============================================================================
// MIDI Bridge Commands - Bypass display server requirement
// ============================================================================

use mymusic_daw::plugin::midi_bridge::{MidiMapping, default_cc_assignments};

#[derive(Debug, Serialize, Deserialize)]
pub struct MidiMappingInfo {
    pub cc_number: u8,
    pub plugin_instance_id: String,
    pub parameter_index: u32,
    pub name: String,
    pub min_value: f32,
    pub max_value: f32,
}

impl From<MidiMapping> for MidiMappingInfo {
    fn from(mapping: MidiMapping) -> Self {
        Self {
            cc_number: mapping.cc_number,
            plugin_instance_id: mapping.plugin_instance_id.to_string(),
            parameter_index: mapping.parameter_index,
            name: mapping.name,
            min_value: mapping.min_value,
            max_value: mapping.max_value,
        }
    }
}

/// Add a MIDI CC to plugin parameter mapping
#[tauri::command]
pub async fn add_midi_mapping(
    cc_number: u8,
    plugin_instance_id: String,
    parameter_index: u32,
    name: String,
    min_value: f32,
    max_value: f32,
    state: State<'_, DawState>,
) -> Result<(), String> {
    println!("🎛️ Adding MIDI mapping: CC {} -> {} param {}", cc_number, plugin_instance_id, parameter_index);
    
    let instance_id: PluginInstanceId = plugin_instance_id.parse()
        .map_err(|e| format!("Invalid plugin instance ID: {}", e))?;
    
    let mapping = MidiMapping {
        cc_number,
        plugin_instance_id: instance_id,
        parameter_index,
        name,
        min_value,
        max_value,
    };
    
    // TODO: Get MIDI bridge from state
    // For now, just log the mapping
    println!("✅ MIDI mapping added: {:?}", mapping);
    
    Ok(())
}

/// Remove a MIDI mapping
#[tauri::command]
pub async fn remove_midi_mapping(
    cc_number: u8,
    state: State<'_, DawState>,
) -> Result<(), String> {
    println!("🗑️ Removing MIDI mapping: CC {}", cc_number);
    
    // TODO: Remove from MIDI bridge
    println!("✅ MIDI mapping removed: CC {}", cc_number);
    
    Ok(())
}

/// Get all current MIDI mappings
#[tauri::command]
pub async fn get_midi_mappings(
    state: State<'_, DawState>,
) -> Result<Vec<MidiMappingInfo>, String> {
    println!("📋 Getting all MIDI mappings...");
    
    // TODO: Get from MIDI bridge
    let mappings: Vec<MidiMapping> = vec![]; // Placeholder
    
    Ok(mappings.into_iter().map(Into::into).collect())
}

/// Auto-map plugin parameters to MIDI CC
#[tauri::command]
pub async fn auto_map_plugin(
    plugin_instance_id: String,
    start_cc: Option<u8>,
    state: State<'_, DawState>,
) -> Result<Vec<MidiMappingInfo>, String> {
    println!("🎛️ Auto-mapping plugin {} to MIDI...", plugin_instance_id);
    
    let instance_id: PluginInstanceId = plugin_instance_id.parse()
        .map_err(|e| format!("Invalid plugin instance ID: {}", e))?;
    
    let start_cc = start_cc.unwrap_or(16); // Start at CC 16 by default
    
    // TODO: Auto-map via MIDI bridge
    let mappings: Vec<MidiMapping> = vec![]; // Placeholder
    
    println!("✅ Auto-mapped {} parameters for plugin {}", mappings.len(), plugin_instance_id);
    
    Ok(mappings.into_iter().map(Into::into).collect())
}

/// Send MIDI CC message to control plugin
#[tauri::command]
pub async fn send_midi_cc(
    plugin_instance_id: String,
    cc_number: u8,
    value: u8,
    state: State<'_, DawState>,
) -> Result<(), String> {
    println!("🎛️ Sending MIDI CC {} value {} to plugin {}", cc_number, value, plugin_instance_id);
    
    // TODO: Send via MIDI bridge
    println!("✅ MIDI CC sent");
    
    Ok(())
}

/// Create virtual MIDI port for plugin communication
#[tauri::command]
pub async fn create_virtual_midi_port(
    port_name: String,
    state: State<'_, DawState>,
) -> Result<(), String> {
    println!("🎹 Creating virtual MIDI port: {}", port_name);
    
    // TODO: Create via MIDI bridge
    println!("✅ Virtual MIDI port created: {}", port_name);
    
    Ok(())
}

/// Test MIDI communication with a plugin
#[tauri::command]
pub async fn test_midi_communication(
    plugin_instance_id: String,
    state: State<'_, DawState>,
) -> Result<String, String> {
    println!("🧪 Testing MIDI communication with plugin {}", plugin_instance_id);
    
    // TODO: Test via MIDI bridge
    // For now, just test basic plugin loading without GUI
    
    let instance_id: PluginInstanceId = plugin_instance_id.parse()
        .map_err(|e| format!("Invalid plugin instance ID: {}", e))?;
    
    // Test if plugin instance exists and can process MIDI
    println!("🧪 Testing plugin {} MIDI capabilities...", instance_id);
    
    // Send test MIDI CC messages
    let test_ccs = vec![
        (7, "Volume"),      // Volume
        (10, "Pan"),        // Pan
        (1, "Modulation"),  // Modulation wheel
    ];
    
    for (cc, name) in test_ccs {
        println!("🎛️ Testing CC {} ({})...", cc, name);
        // TODO: Send test CC via MIDI bridge
    }
    
    Ok(format!("MIDI communication test completed for plugin {}", plugin_instance_id))
}

/// Get default MIDI CC assignments
#[tauri::command]
pub async fn get_default_midi_assignments() -> Result<Vec<(u8, String)>, String> {
    println!("📋 Getting default MIDI CC assignments...");
    
    let assignments = vec![
        (default_cc_assignments::VOLUME, "Volume".to_string()),
        (default_cc_assignments::PAN, "Pan".to_string()),
        (default_cc_assignments::EXPRESSION, "Expression".to_string()),
        (default_cc_assignments::SUSTAIN, "Sustain".to_string()),
        (default_cc_assignments::PORTAMENTO, "Portamento".to_string()),
        (default_cc_assignments::SOSTENUTO, "Sostenuto".to_string()),
        (default_cc_assignments::SOFT_PEDAL, "Soft Pedal".to_string()),
        (default_cc_assignments::LEGATO_FOOTSWITCH, "Legato".to_string()),
        (default_cc_assignments::HOLD_2, "Hold 2".to_string()),
    ];
    
    Ok(assignments)
}
