use mymusic_daw::ui::app::DawApp;
use mymusic_daw::{
    AudioEngine, MidiConnectionManager, create_command_channel, create_notification_channel,
};
use mymusic_daw::plugin::PluginHost;
use std::sync::{Arc, Mutex};

// Ringbuffer capacity constants
// Sized for worst-case MIDI burst scenarios:
// - MIDI can theoretically send ~1000 messages/second (31250 baud)
// - With typical audio buffer of 10-20ms, we expect <20 messages per callback
// - 512 capacity provides >500ms buffer at max MIDI rate
// - Safe for buffer sizes up to 24576 samples (~500ms at 48kHz)
const MIDI_RINGBUFFER_CAPACITY: usize = 512;
const UI_RINGBUFFER_CAPACITY: usize = 512;
const NOTIFICATION_RINGBUFFER_CAPACITY: usize = 256;

fn main() {
    println!("=== MyMusic DAW ===");
    println!("Version 0.1.0 - MVP\n");

    // Create the communication channels
    // Need 2 ringbufs : one for MIDI, One for UI
    let (command_tx_ui, command_rx_ui) = create_command_channel(UI_RINGBUFFER_CAPACITY);
    let (command_tx_midi, command_rx_midi) = create_command_channel(MIDI_RINGBUFFER_CAPACITY);

    // Create notification channel (for error handling)
    let (notification_tx, notification_rx) =
        create_notification_channel(NOTIFICATION_RINGBUFFER_CAPACITY);
    let notification_tx = Arc::new(Mutex::new(notification_tx));

    // Create plugin host for plugin management
    let plugin_host = Arc::new(PluginHost::new());
    println!("Plugin host initialized");

    println!("Audio engine initialisation...");
    let audio_engine =
        match AudioEngine::new(command_rx_ui, command_rx_midi, notification_tx.clone(), plugin_host.clone()) {
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
            let mut app = DawApp::new(
                command_tx_ui,
                audio_engine.volume.clone(),
                midi_manager,
                audio_engine.cpu_monitor.clone(),
                notification_rx,
            );

            // Load cached plugins on startup
            app.load_cached_plugins();

            Ok(Box::new(app))
        }),
    );
}
