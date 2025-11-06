// CLAP Plugin GUI - Window embedding and management
//
// This module provides GUI support for CLAP plugins, enabling native
// window embedding into the host application.

use crate::plugin::clap_ffi::*;
use crate::plugin::PluginError;
use std::ffi::{CStr, CString};
use std::ptr;

/// CLAP plugin GUI wrapper
pub struct ClapPluginGui {
    plugin_ptr: *const clap_plugin,
    gui_ext: *const clap_plugin_gui,
    is_created: bool,
    is_visible: bool,
    width: u32,
    height: u32,
    api: String,
}

impl ClapPluginGui {
    /// Create a new CLAP plugin GUI wrapper
    ///
    /// Returns None if the plugin doesn't support GUI extension
    pub fn new(plugin_ptr: *const clap_plugin) -> Option<Self> {
        if plugin_ptr.is_null() {
            return None;
        }

        unsafe {
            let plugin = &*plugin_ptr;

            // Get GUI extension
            let gui_id = CStr::from_bytes_with_nul(CLAP_EXT_GUI).ok()?;
            let gui_ext_ptr = (plugin.get_extension)(plugin_ptr, gui_id.as_ptr());

            if gui_ext_ptr.is_null() {
                return None; // Plugin doesn't support GUI
            }

            let gui_ext = gui_ext_ptr as *const clap_plugin_gui;

            // Determine which API to use (platform-specific)
            let api = Self::get_platform_api(plugin_ptr, gui_ext)?;

            Some(Self {
                plugin_ptr,
                gui_ext,
                is_created: false,
                is_visible: false,
                width: 0,
                height: 0,
                api,
            })
        }
    }

    /// Get the best window API for the current platform
    fn get_platform_api(
        plugin_ptr: *const clap_plugin,
        gui_ext: *const clap_plugin_gui,
    ) -> Option<String> {
        unsafe {
            let gui = &*gui_ext;

            // Try platform-specific APIs in order of preference
            #[cfg(target_os = "macos")]
            let apis = [CLAP_WINDOW_API_COCOA];

            #[cfg(target_os = "linux")]
            let apis = [CLAP_WINDOW_API_X11, CLAP_WINDOW_API_WAYLAND];

            #[cfg(target_os = "windows")]
            let apis = [CLAP_WINDOW_API_WIN32];

            for &api_bytes in &apis {
                let api_cstr = CStr::from_bytes_with_nul(api_bytes).ok()?;
                let supported = (gui.is_api_supported)(plugin_ptr, api_cstr.as_ptr(), false);

                if supported {
                    return api_cstr.to_str().ok().map(|s| s.to_string());
                }
            }

            None
        }
    }

    /// Create the GUI
    pub fn create(&mut self) -> Result<(), PluginError> {
        if self.is_created {
            return Ok(());
        }

        unsafe {
            let gui = &*self.gui_ext;

            let api_cstring = CString::new(self.api.clone())
                .map_err(|_| PluginError::GuiFailed("Invalid API string".to_string()))?;

            let result = (gui.create)(self.plugin_ptr, api_cstring.as_ptr(), false);

            if !result {
                return Err(PluginError::GuiFailed("Failed to create GUI".to_string()));
            }

            // Get initial size
            let mut width: u32 = 0;
            let mut height: u32 = 0;
            (gui.get_size)(self.plugin_ptr, &mut width, &mut height);

            self.width = width;
            self.height = height;
            self.is_created = true;

            Ok(())
        }
    }

    /// Attach the plugin GUI to a parent window
    ///
    /// # Safety
    /// window_handle must be a valid platform-specific window handle
    pub unsafe fn attach_to_window(&mut self, window_handle: *mut std::ffi::c_void) -> Result<(), PluginError> {
        if !self.is_created {
            self.create()?;
        }

        let gui = &*self.gui_ext;

        // Create platform-specific window handle
        let clap_handle = self.create_window_handle(window_handle)?;

        let api_cstring = CString::new(self.api.clone())
            .map_err(|_| PluginError::GuiFailed("Invalid API string".to_string()))?;

        let window = clap_window {
            api: api_cstring.as_ptr(),
            handle: clap_handle,
        };

        let result = (gui.set_parent)(self.plugin_ptr, &window);

        if !result {
            return Err(PluginError::GuiFailed("Failed to set parent window".to_string()));
        }

        Ok(())
    }

    /// Create platform-specific window handle
    unsafe fn create_window_handle(
        &self,
        handle: *mut std::ffi::c_void,
    ) -> Result<clap_window_handle, PluginError> {
        #[cfg(target_os = "macos")]
        {
            Ok(clap_window_handle { cocoa: handle })
        }

        #[cfg(target_os = "windows")]
        {
            Ok(clap_window_handle { win32: handle })
        }

        #[cfg(target_os = "linux")]
        {
            // For X11, handle is a Window (u64)
            if self.api == "x11" {
                Ok(clap_window_handle {
                    x11: handle as u64,
                })
            } else if self.api == "wayland" {
                Ok(clap_window_handle { wayland: handle })
            } else {
                Err(PluginError::GuiFailed(
                    "Unsupported window API".to_string(),
                ))
            }
        }
    }

    /// Show the plugin GUI
    pub fn show(&mut self) -> Result<(), PluginError> {
        if !self.is_created {
            return Err(PluginError::GuiFailed("GUI not created".to_string()));
        }

        unsafe {
            let gui = &*self.gui_ext;
            let result = (gui.show)(self.plugin_ptr);

            if !result {
                return Err(PluginError::GuiFailed("Failed to show GUI".to_string()));
            }

            self.is_visible = true;
            Ok(())
        }
    }

    /// Hide the plugin GUI
    pub fn hide(&mut self) -> Result<(), PluginError> {
        if !self.is_created {
            return Ok(());
        }

        unsafe {
            let gui = &*self.gui_ext;
            let result = (gui.hide)(self.plugin_ptr);

            if !result {
                return Err(PluginError::GuiFailed("Failed to hide GUI".to_string()));
            }

            self.is_visible = false;
            Ok(())
        }
    }

    /// Get GUI size
    pub fn get_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Set GUI size
    pub fn set_size(&mut self, width: u32, height: u32) -> Result<(), PluginError> {
        if !self.is_created {
            return Err(PluginError::GuiFailed("GUI not created".to_string()));
        }

        unsafe {
            let gui = &*self.gui_ext;
            let result = (gui.set_size)(self.plugin_ptr, width, height);

            if !result {
                return Err(PluginError::GuiFailed("Failed to set size".to_string()));
            }

            self.width = width;
            self.height = height;
            Ok(())
        }
    }

    /// Check if GUI can be resized
    pub fn can_resize(&self) -> bool {
        if !self.is_created {
            return false;
        }

        unsafe {
            let gui = &*self.gui_ext;
            (gui.can_resize)(self.plugin_ptr)
        }
    }

    /// Check if GUI is visible
    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    /// Check if GUI is created
    pub fn is_created(&self) -> bool {
        self.is_created
    }

    /// Get window API being used
    pub fn api(&self) -> &str {
        &self.api
    }
}

impl Drop for ClapPluginGui {
    fn drop(&mut self) {
        if self.is_created {
            unsafe {
                let gui = &*self.gui_ext;
                (gui.destroy)(self.plugin_ptr);
            }
            self.is_created = false;
        }
    }
}

// Safety: GUI operations should be done on main thread only
// The struct itself can be Send but operations must be serialized
unsafe impl Send for ClapPluginGui {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_api_selection() {
        // Platform API selection is tested via integration tests
        // with real plugins since we need a valid plugin pointer
    }

    #[test]
    fn test_gui_lifecycle() {
        // Lifecycle tests require a real plugin
        // Will be tested in integration tests
    }
}
