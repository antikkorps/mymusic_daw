// MIDI Input - Réception des événements MIDI

use crate::messaging::channels::CommandProducer;
use crate::messaging::command::Command;
use crate::midi::event::{MidiEvent, MidiEventTimed};
use midir::{MidiInput as MidirInput, MidiInputConnection};

pub struct MidiInput {
    _connection: Option<MidiInputConnection<()>>,
}

impl MidiInput {
    pub fn new(mut command_tx: CommandProducer) -> Result<Self, String> {
        let midi_in = MidirInput::new("MyMusic DAW MIDI Input")
            .map_err(|e| format!("Midi init error: {}", e))?;

        // Lister les ports MIDI disponibles
        let ports = midi_in.ports();

        if ports.is_empty() {
            println!("No MIDI port detected. The DAW will continue running without MIDI.");
            return Ok(Self { _connection: None });
        }

        println!("\n=== MIDI ports unavailable ===");
        for (i, port) in ports.iter().enumerate() {
            if let Ok(name) = midi_in.port_name(port) {
                println!("  [{}] {}", i, name);
            }
        }

        // Use the first MIDI available port
        let port = &ports[0];
        let port_name = midi_in
            .port_name(port)
            .unwrap_or_else(|_| "Unknown".to_string());

        println!("\nConnected to MIDI port: {}", port_name);

        // Create the first MIDI connexion with the callback
        let connection = midi_in
            .connect(
                port,
                "mymusic-daw-input",
                move |_timestamp, message, _| {
                    // MIDI Callback  - running on a separate thread
                    if let Some(midi_event) = MidiEvent::from_bytes(message) {
                        // Create timed MIDI event
                        // TODO: Calculate precise samples_from_now based on _timestamp
                        // For now, use 0 (immediate processing) to establish infrastructure
                        let timed_event = MidiEventTimed {
                            event: midi_event,
                            samples_from_now: 0,
                        };

                        // Send the MIDI event in the ringbuffer
                        let cmd = Command::Midi(timed_event);

                        // try_push is not blocking
                        if ringbuf::traits::Producer::try_push(&mut command_tx, cmd).is_err() {
                            // Full buffer - ignore the event
                            eprintln!("Warning: MIDI buffer full, event ignored");
                        }
                    }
                },
                (),
            )
            .map_err(|e| format!("MIDI connexion ignored: {}", e))?;

        println!("MIDI input working!");

        Ok(Self {
            _connection: Some(connection),
        })
    }
}
