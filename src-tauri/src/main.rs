// Tauri main entry point for MyMusic DAW
// Initializes the audio engine and starts the Tauri app

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tauri::Manager;

// Import DAW modules
use mymusic_daw::audio::{AtomicF32, AudioDeviceManager, AudioEngine, CpuMonitor};
use mymusic_daw::messaging::create_channels;
use mymusic_daw::midi::MidiConnectionManager;

// Import Tauri commands
mod lib;
use lib::DawState;

fn main() {
    // Initialize the audio engine
    println!("🎵 Initializing MyMusic DAW...");

    // Create communication channels
    let (command_tx, command_rx) = create_channels();

    // Create volume atomic
    let volume_atomic = Arc::new(AtomicF32::new(0.5)); // Default 50% volume

    // Create MIDI connection manager
    let midi_manager = MidiConnectionManager::new(command_tx.clone());

    // Create CPU monitor
    let cpu_monitor = CpuMonitor::new();

    // Create notification channel (unused in Tauri for now, but required by AudioEngine)
    let (notification_tx, _notification_rx) = ringbuf::HeapRb::new(256).split();

    // Initialize audio device manager
    let audio_device_manager = AudioDeviceManager::new();
    let available_devices = audio_device_manager.list_output_devices();

    println!("📢 Available audio devices:");
    for device in &available_devices {
        println!(
            "  {} {}",
            if device.is_default { "✓" } else { " " },
            device.name
        );
    }

    // Create audio engine
    let audio_engine = AudioEngine::new(
        command_rx,
        volume_atomic.clone(),
        cpu_monitor.clone(),
        notification_tx,
    );

    // Start audio stream
    match audio_engine.start() {
        Ok(_stream) => {
            println!("✅ Audio engine started successfully");

            // Store stream to keep it alive (Tauri will manage its lifetime)
            // In a real app, you'd want to store this in managed state
            std::mem::forget(_stream);
        }
        Err(e) => {
            eprintln!("❌ Failed to start audio engine: {}", e);
            std::process::exit(1);
        }
    }

    // Create DAW state for Tauri
    let daw_state = DawState::new(command_tx, volume_atomic);

    // Build and run Tauri application
    tauri::Builder::default()
        .setup(|app| {
            println!("🚀 Tauri app initialized");
            println!("🎹 DAW is ready!");

            // Log window info
            if let Some(window) = app.get_webview_window("main") {
                println!("📱 Main window created: {:?}", window.label());
            }

            Ok(())
        })
        .manage(daw_state)
        .invoke_handler(tauri::generate_handler![
            // Audio engine commands
            lib::set_volume,
            lib::play_note,
            lib::stop_note,
            lib::get_volume,
            lib::get_engine_status,
            lib::get_engine_info,
            lib::play_test_beep,
            // Plugin management commands
            lib::load_plugin_instance,
            lib::get_plugin_parameters,
            lib::get_plugin_parameter_value,
            lib::set_plugin_parameter_value,
            lib::unload_plugin_instance,
            lib::get_loaded_plugins,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
