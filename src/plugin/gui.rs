use crate::plugin::PluginError;
use std::ffi::c_void;

/// GUI API for plugins
pub trait PluginGuiApi {
    /// Show the plugin GUI
    fn show(&mut self) -> Result<(), PluginError>;
    
    /// Hide the plugin GUI
    fn hide(&mut self) -> Result<(), PluginError>;
    
    /// Check if GUI is currently visible
    fn is_visible(&self) -> bool;
    
    /// Get current GUI size
    fn get_size(&self) -> (u32, u32);
    
    /// Set GUI size (if resizable)
    fn set_size(&mut self, width: u32, height: u32) -> Result<(), PluginError>;
    
    /// Get preferred GUI size
    fn get_preferred_size(&self) -> (u32, u32);
    
    /// Check if GUI is resizable
    fn is_resizable(&self) -> bool;
    
    /// Set GUI scale factor
    fn set_scale(&mut self, scale: f32) -> Result<(), PluginError>;
    
    /// Get current scale factor
    fn get_scale(&self) -> f32;
    
    /// Can the GUI be closed by the user?
    fn can_close(&self) -> bool;
    
    /// Request the GUI to close
    fn close(&mut self) -> Result<(), PluginError>;
}

/// Parent window information for embedding plugin GUIs
#[derive(Debug, Clone)]
pub struct GuiParent {
    /// Native window handle
    pub window_handle: *mut c_void,
    /// Window position
    pub x: i32,
    pub y: i32,
    /// Window size
    pub width: u32,
    pub height: u32,
}

impl GuiParent {
    /// Create a new parent window info
    pub fn new(window_handle: *mut c_void, x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            window_handle,
            x,
            y,
            width,
            height,
        }
    }
}

unsafe impl Send for GuiParent {}
unsafe impl Sync for GuiParent {}

/// GUI backend types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiBackend {
    /// No GUI
    None,
    /// Native platform GUI
    Native,
    /// Cross-platform toolkit (e.g., GLFW, SDL)
    CrossPlatform,
    /// Web-based GUI
    Web,
}

/// GUI threading model
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiThreadingModel {
    /// GUI must run on main thread
    MainThread,
    /// GUI can run on any thread
    AnyThread,
    /// GUI has its own thread management
    DedicatedThread,
}

/// GUI capabilities
#[derive(Debug, Clone)]
pub struct GuiCapabilities {
    pub backend: GuiBackend,
    pub threading_model: GuiThreadingModel,
    pub is_resizable: bool,
    pub supports_scaling: bool,
    pub supports_transparency: bool,
    pub minimum_size: Option<(u32, u32)>,
    pub maximum_size: Option<(u32, u32)>,
}

impl GuiCapabilities {
    pub fn new(backend: GuiBackend) -> Self {
        Self {
            backend,
            threading_model: GuiThreadingModel::MainThread,
            is_resizable: false,
            supports_scaling: false,
            supports_transparency: false,
            minimum_size: None,
            maximum_size: None,
        }
    }

    pub fn with_threading_model(mut self, model: GuiThreadingModel) -> Self {
        self.threading_model = model;
        self
    }

    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.is_resizable = resizable;
        self
    }

    pub fn with_scaling(mut self, supports: bool) -> Self {
        self.supports_scaling = supports;
        self
    }

    pub fn with_transparency(mut self, supports: bool) -> Self {
        self.supports_transparency = supports;
        self
    }

    pub fn with_minimum_size(mut self, width: u32, height: u32) -> Self {
        self.minimum_size = Some((width, height));
        self
    }

    pub fn with_maximum_size(mut self, width: u32, height: u32) -> Self {
        self.maximum_size = Some((width, height));
        self
    }
}

/// GUI event types
#[derive(Debug, Clone)]
pub enum GuiEvent {
    /// Window close requested
    CloseRequested,
    /// Window resized
    Resized { width: u32, height: u32 },
    /// Window moved
    Moved { x: i32, y: i32 },
    /// Mouse button pressed
    MouseDown { x: f32, y: f32, button: MouseButton },
    /// Mouse button released
    MouseUp { x: f32, y: f32, button: MouseButton },
    /// Mouse moved
    MouseMove { x: f32, y: f32 },
    /// Mouse wheel scrolled
    MouseWheel { delta_x: f32, delta_y: f32 },
    /// Key pressed
    KeyDown { key_code: u32, modifiers: KeyModifiers },
    /// Key released
    KeyUp { key_code: u32, modifiers: KeyModifiers },
    /// Focus gained
    FocusGained,
    /// Focus lost
    FocusLost,
}

/// Mouse buttons
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

/// Key modifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

impl KeyModifiers {
    pub fn new() -> Self {
        Self {
            shift: false,
            ctrl: false,
            alt: false,
            meta: false,
        }
    }

    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    pub fn with_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    pub fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }

    pub fn with_meta(mut self) -> Self {
        self.meta = true;
        self
    }
}

/// GUI rendering context
pub trait GuiRenderContext {
    /// Get the native window handle
    fn get_window_handle(&self) -> *mut c_void;
    
    /// Get the rendering API type
    fn get_render_api(&self) -> GuiRenderApi;
    
    /// Swap buffers (for double buffering)
    fn swap_buffers(&self) -> Result<(), PluginError>;
    
    /// Make the context current
    fn make_current(&self) -> Result<(), PluginError>;
    
    /// Get frame buffer size
    fn get_framebuffer_size(&self) -> (u32, u32);
    
    /// Get DPI scale factor
    fn get_dpi_scale(&self) -> f32;
}

/// Rendering APIs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiRenderApi {
    OpenGL,
    Vulkan,
    Metal,
    Direct3D11,
    Direct3D12,
    Software,
}

/// GUI manager for handling plugin GUIs
pub struct GuiManager {
    /// Active GUI instances
    guis: std::collections::HashMap<String, Box<dyn PluginGuiApi>>,
    /// GUI event queue
    event_queue: std::sync::mpsc::Receiver<GuiEvent>,
    /// Event sender
    event_sender: std::sync::mpsc::Sender<GuiEvent>,
}

impl GuiManager {
    /// Create a new GUI manager
    pub fn new() -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        
        Self {
            guis: std::collections::HashMap::new(),
            event_queue: receiver,
            event_sender: sender,
        }
    }

    /// Register a GUI instance
    pub fn register_gui(&mut self, id: String, gui: Box<dyn PluginGuiApi>) {
        self.guis.insert(id, gui);
    }

    /// Unregister a GUI instance
    pub fn unregister_gui(&mut self, id: &str) {
        self.guis.remove(id);
    }

    /// Get a GUI instance
    pub fn get_gui(&self, id: &str) -> Option<&dyn PluginGuiApi> {
        self.guis.get(id).map(|gui| gui.as_ref())
    }

    /// Get a mutable GUI instance
    pub fn get_gui_mut(&mut self, id: &str) -> Option<&mut dyn PluginGuiApi> {
        self.guis.get_mut(id).map(|gui| gui.as_mut())
    }

    /// Process GUI events
    pub fn process_events(&mut self) -> Vec<GuiEvent> {
        let mut events = Vec::new();
        
        while let Ok(event) = self.event_queue.try_recv() {
            events.push(event);
        }

        events
    }

    /// Send an event to all GUIs
    pub fn broadcast_event(&self, event: GuiEvent) {
        // This would need to be implemented to send events to all GUIs
        // For now, it's a placeholder
    }

    /// Get event sender for external event sources
    pub fn get_event_sender(&self) -> std::sync::mpsc::Sender<GuiEvent> {
        self.event_sender.clone()
    }

    /// Update all GUIs
    pub fn update(&mut self) -> Result<(), PluginError> {
        for gui in self.guis.values_mut() {
            // Update GUI state
            // This would be implemented based on the specific GUI backend
        }
        
        Ok(())
    }

    /// Render all visible GUIs
    pub fn render(&self) -> Result<(), PluginError> {
        for gui in self.guis.values() {
            // Render GUI if visible
            // This would be implemented based on the specific GUI backend
        }
        
        Ok(())
    }

    /// Get the number of registered GUIs
    pub fn gui_count(&self) -> usize {
        self.guis.len()
    }

    /// Get all registered GUI IDs
    pub fn get_gui_ids(&self) -> Vec<String> {
        self.guis.keys().cloned().collect()
    }
}

impl Default for GuiManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gui_capabilities() {
        let capabilities = GuiCapabilities::new(GuiBackend::Native)
            .with_resizable(true)
            .with_scaling(true)
            .with_minimum_size(200, 150)
            .with_maximum_size(1920, 1080);

        assert_eq!(capabilities.backend, GuiBackend::Native);
        assert!(capabilities.is_resizable);
        assert!(capabilities.supports_scaling);
        assert_eq!(capabilities.minimum_size, Some((200, 150)));
        assert_eq!(capabilities.maximum_size, Some((1920, 1080)));
    }

    #[test]
    fn test_key_modifiers() {
        let modifiers = KeyModifiers::new()
            .with_shift()
            .with_ctrl();

        assert!(modifiers.shift);
        assert!(modifiers.ctrl);
        assert!(!modifiers.alt);
        assert!(!modifiers.meta);
    }

    #[test]
    fn test_gui_parent() {
        let handle = std::ptr::null_mut();
        let parent = GuiParent::new(handle, 100, 100, 800, 600);

        assert_eq!(parent.window_handle, handle);
        assert_eq!(parent.x, 100);
        assert_eq!(parent.y, 100);
        assert_eq!(parent.width, 800);
        assert_eq!(parent.height, 600);
    }

    #[test]
    fn test_gui_manager() {
        let manager = GuiManager::new();
        
        assert_eq!(manager.gui_count(), 0);
        assert!(manager.get_gui_ids().is_empty());
    }
}