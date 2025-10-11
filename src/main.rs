mod audio;
mod connection;
mod messaging;
mod midi;
mod synth;
mod ui;

use audio::engine::AudioEngine;
use messaging::channels::{create_command_channel, create_notification_channel};
use midi::manager::MidiConnectionManager;
use ui::app::DawApp;
use std::sync::{Arc, Mutex};

fn main() {
    println!("=== MyMusic DAW ===");
    println!("Version 0.1.0 - MVP\n");

    // Create the communication channels
    // Need 2 ringbufs : one for MIDI, One for UI
    let (command_tx_ui, command_rx_ui) = create_command_channel(512);
    let (command_tx_midi, command_rx_midi) = create_command_channel(512);

    // Create notification channel (for error handling)
    let (notification_tx, notification_rx) = create_notification_channel(256);
    let notification_tx = Arc::new(Mutex::new(notification_tx));

    println!("Audio engine initialisation...");
    let audio_engine = match AudioEngine::new(command_rx_ui, command_rx_midi) {
        Ok(engine) => engine,
        Err(e) => {
            eprintln!("ERROR: {}", e);
            return;
        }
    };

    println!("\nMIDI Initialisation...");
    let midi_manager = MidiConnectionManager::new(command_tx_midi, notification_tx);

    println!("\n=== DAW started ! ===\n");
    println!("Graphical UI launching...\n");

    // Lancer l'UI egui
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 450.0])
            .with_title("MyMusic DAW"),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "MyMusic DAW",
        native_options,
        Box::new(|_cc| {
            Ok(Box::new(DawApp::new(
                command_tx_ui,
                audio_engine.volume.clone(),
                midi_manager,
                audio_engine.cpu_monitor.clone(),
                notification_rx,
            )))
        }),
    );
}
