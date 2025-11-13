// Tauri main entry point for MyMusic DAW
// Initializes the audio engine and starts the Tauri app

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tauri::Manager;

// Import DAW modules
use mymusic_daw::audio::device::AudioDeviceManager;
use mymusic_daw::{
    create_command_channel, create_notification_channel, AudioEngine, MidiConnectionManager,
};

// Import library with commands and state
use app_lib::{register_commands, DawState};
use app_lib::events::AUDIO_EVENT_EMITTER;

fn main() {
    // Initialize the audio engine
    println!("🎵 Initializing MyMusic DAW...");

    // Create communication channels
    // We need two separate channels: one for UI commands and one for MIDI commands
    let (command_tx_ui, command_rx_ui) = create_command_channel(1024);
    let (command_tx_midi, command_rx_midi) = create_command_channel(1024);

    // Create notification channel
    let (notification_tx, _notification_rx) = create_notification_channel(256);
    let notification_tx_arc = Arc::new(std::sync::Mutex::new(notification_tx));

    // Create MIDI connection manager
    let _midi_manager = MidiConnectionManager::new(command_tx_midi, notification_tx_arc.clone());

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
    let audio_engine = match AudioEngine::new(
        command_rx_ui,
        command_rx_midi,
        notification_tx_arc.clone(),
    ) {
        Ok(engine) => {
            println!("✅ Audio engine started successfully");
            engine
        }
        Err(e) => {
            eprintln!("❌ Failed to start audio engine: {}", e);
            std::process::exit(1);
        }
    };

    // Get volume from audio engine (it's created internally)
    // Wrap it in Arc to match DawState::new() signature
    let volume_atomic = Arc::new(audio_engine.volume.clone());

    // Create DAW state for Tauri
    let daw_state = DawState::new(command_tx_ui, volume_atomic);

    // Keep the audio engine alive (Tauri will manage its lifetime)
    std::mem::forget(audio_engine);

    // Build and run Tauri application
    let builder = tauri::Builder::default()
        .setup(|app| {
            println!("🚀 Tauri app initialized");
            println!("🎹 DAW is ready!");

            // Initialize event system
            if let Ok(mut emitter) = AUDIO_EVENT_EMITTER.lock() {
                emitter.set_app_handle(app.handle().clone());
                println!("📡 Event system initialized");
            } else {
                eprintln!("❌ Failed to initialize event system");
            }

            // Log window info
            if let Some(window) = app.get_webview_window("main") {
                println!("📱 Main window created: {:?}", window.label());
            }

            Ok(())
        })
        .manage(daw_state);

    // Register all Tauri commands
    register_commands(builder)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
