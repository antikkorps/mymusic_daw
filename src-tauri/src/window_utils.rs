// Window handle utilities for Tauri
//
// This module provides platform-specific utilities for getting window handles
// from Tauri windows for plugin GUI embedding.

use tauri::WebviewWindow;
use std::ffi::c_void;
use base64::Engine;

/// Get platform-specific window handle for plugin GUI embedding
pub fn get_window_handle(window: &WebviewWindow) -> Result<*mut c_void, String> {
    #[cfg(target_os = "macos")]
    {
        get_macos_window_handle(window)
    }
    
    #[cfg(target_os = "windows")]
    {
        get_windows_window_handle(window)
    }
    
    #[cfg(target_os = "linux")]
    {
        get_linux_window_handle(window)
    }
}

/// Get macOS NSView handle from Tauri window
#[cfg(target_os = "macos")]
fn get_macos_window_handle(window: &WebviewWindow) -> Result<*mut c_void, String> {
    // Get the NSView from the window
    let ns_view = window
        .ns_view()
        .map_err(|e| format!("Failed to get NSView: {}", e))?;
    
    // Convert NSView pointer to raw pointer
    let view_ptr: *mut c_void = ns_view as *mut c_void;
    
    println!("🍎 Got macOS NSView handle: {:p}", view_ptr);
    Ok(view_ptr)
}

/// Get Windows HWND from Tauri window
#[cfg(target_os = "windows")]
fn get_windows_window_handle(window: &WebviewWindow) -> Result<*mut c_void, String> {
    use windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW;
    use windows::Win32::UI::WindowsAndMessaging::GWL_USERDATA;
    use windows::core::PCWSTR;
    
    // Get the HWND from the window
    let hwnd = window
        .hwnd()
        .map_err(|e| format!("Failed to get HWND: {}", e))?;
    
    // Convert HWND to raw pointer
    let hwnd_ptr: *mut c_void = hwnd.0 as *mut c_void;
    
    println!("🪟 Got Windows HWND handle: {:p}", hwnd_ptr);
    Ok(hwnd_ptr)
}

/// Get X11 Window handle from Tauri window
#[cfg(target_os = "linux")]
fn get_linux_window_handle(window: &WebviewWindow) -> Result<*mut c_void, String> {
    use gtk::prelude::*;
    use gtk::{Window as GtkWindow, WidgetExt};
    
    // Try to get the GTK window
    if let Some(gtk_window) = window.gtk_window() {
        // Get the X11 Window ID
        let xid = gtk_window.window().and_then(|w| {
            w.display().get_default_screen()
                .and_then(|s| s.root_window())
                .map(|rw| unsafe {
                    // This is a simplified approach - in practice you'd need
                    // more sophisticated X11 handling
                    rw.xid() as *mut c_void
                })
        });
        
        if let Some(xid_ptr) = xid {
            println!("🐧 Got Linux X11 Window handle: {:p}", xid_ptr);
            return Ok(xid_ptr);
        }
    }
    
    // Fallback: try Wayland (more complex)
    Err("Linux window handle extraction not fully implemented".to_string())
}

/// Encode window handle as base64 string for safe transport
pub fn encode_window_handle(handle: *mut c_void) -> Result<String, String> {
    use std::mem;
    
    // Convert pointer to bytes based on platform pointer size
    let ptr_bytes = if mem::size_of::<*mut c_void>() == 8 {
        (handle as u64).to_le_bytes().to_vec()
    } else {
        (handle as u32).to_le_bytes().to_vec()
    };
    
    let encoded = base64::prelude::BASE64_STANDARD.encode(&ptr_bytes);
    Ok(encoded)
}

/// Decode window handle from base64 string
pub fn decode_window_handle(encoded: &str) -> Result<*mut c_void, String> {
    use std::mem;
    
    let decoded = base64::prelude::BASE64_STANDARD.decode(encoded)
        .map_err(|e| format!("Failed to decode window handle: {}", e))?;
    
    let handle = if mem::size_of::<*mut c_void>() == 8 && decoded.len() == 8 {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&decoded);
        u64::from_le_bytes(bytes) as *mut c_void
    } else if mem::size_of::<*mut c_void>() == 4 && decoded.len() == 4 {
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&decoded);
        u32::from_le_bytes(bytes) as *mut c_void
    } else {
        return Err("Invalid window handle size".to_string());
    };
    
    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_window_handle_encoding() {
        let handle = 0x12345678 as *mut c_void;
        let encoded = encode_window_handle(handle).unwrap();
        let decoded = decode_window_handle(&encoded).unwrap();
        
        assert_eq!(handle, decoded);
    }
}