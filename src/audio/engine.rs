// Moteur audio - Callback CPAL temps-réel

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use std::sync::Arc;

use crate::audio::parameters::AtomicF32;
use crate::messaging::channels::CommandConsumer;
use crate::messaging::command::Command;
use crate::midi::event::MidiEvent;
use crate::synth::voice_manager::VoiceManager;

pub struct AudioEngine {
    _device: Device,
    _stream: Stream,
    sample_rate: f32,
    pub volume: AtomicF32,
}

impl AudioEngine {
    pub fn new(
        command_rx_ui: CommandConsumer,
        command_rx_midi: CommandConsumer,
    ) -> Result<Self, String> {
        // Obtenir le host audio par défaut
        let host = cpal::default_host();

        // Obtenir le device de sortie par défaut
        let device = host
            .default_output_device()
            .ok_or("No audio device found")?;

        println!(
            "Device audio: {}",
            device.name().unwrap_or("Unknown".to_string())
        );

        // Configuration du stream
        let config = device
            .default_output_config()
            .map_err(|e| format!("Erreur de configuration: {}", e))?;

        println!("Config audio: {:?}", config);

        let sample_rate = config.sample_rate().0 as f32;
        let channels = config.channels() as usize;

        let config: StreamConfig = config.into();

        // Create atomic volume parameter (shared between UI and audio thread)
        let volume = AtomicF32::new(0.5); // Default volume: 50%
        let volume_clone = volume.clone();

        // Créer le VoiceManager (pré-alloué, partagé avec le callback)
        let voice_manager = Arc::new(std::sync::Mutex::new(VoiceManager::new(sample_rate)));
        let voice_manager_clone = Arc::clone(&voice_manager);

        // Ringbuffers pour les commandes (shared avec le callback)
        let command_rx_ui = Arc::new(std::sync::Mutex::new(command_rx_ui));
        let command_rx_ui_clone = Arc::clone(&command_rx_ui);

        let command_rx_midi = Arc::new(std::sync::Mutex::new(command_rx_midi));
        let command_rx_midi_clone = Arc::clone(&command_rx_midi);

        // Créer le stream audio avec le callback
        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // ========== SACRED ZONE ==========
                    // No allocations, No I/O, No blocking locks

                    // helper function to process commands
                    let process_command = |cmd: Command, vm: &mut VoiceManager| {
                        match cmd {
                            Command::Midi(midi_event) => {
                                match midi_event {
                                    MidiEvent::NoteOn { note, velocity } => {
                                        vm.note_on(note, velocity);
                                    }
                                    MidiEvent::NoteOff { note } => {
                                        vm.note_off(note);
                                    }
                                    _ => {} // Ignore other events for now
                                }
                            }
                            Command::SetVolume(_vol) => {
                                // TODO: implement volume control
                            }
                            Command::Quit => {}
                        }
                    };

                    // treat UI commands
                    if let Ok(mut rx) = command_rx_ui_clone.try_lock() {
                        if let Ok(mut vm) = voice_manager_clone.try_lock() {
                            while let Some(cmd) = ringbuf::traits::Consumer::try_pop(&mut *rx) {
                                process_command(cmd, &mut vm);
                            }
                        }
                    }

                    // Treat MIDI commands
                    if let Ok(mut rx) = command_rx_midi_clone.try_lock() {
                        if let Ok(mut vm) = voice_manager_clone.try_lock() {
                            while let Some(cmd) = ringbuf::traits::Consumer::try_pop(&mut *rx) {
                                process_command(cmd, &mut vm);
                            }
                        }
                    }

                    // Generate audio samples
                    if let Ok(mut vm) = voice_manager_clone.try_lock() {
                        // Read volume once per buffer (atomic read)
                        let current_volume = volume_clone.get();

                        for frame in data.chunks_mut(channels) {
                            let sample = vm.next_sample() * current_volume;

                            // write in all channels (mono → stereo)
                            for channel_sample in frame.iter_mut() {
                                *channel_sample = sample;
                            }
                        }
                    } else {
                        // Fallback: silence if we cannot aquire the lock
                        for sample in data.iter_mut() {
                            *sample = 0.0;
                        }
                    }
                    // ========== SACRED ZONE END ==========
                },
                |err| {
                    eprintln!("Error in audio stream: {}", err);
                },
                None,
            )
            .map_err(|e| format!("Error in stream creation: {}", e))?;

        // Start stream
        stream
            .play()
            .map_err(|e| format!("Error in stream beginning: {}", e))?;

        println!(
            "Audio engine started: {} Hz, {} canaux",
            sample_rate, channels
        );

        Ok(Self {
            _device: device,
            _stream: stream,
            sample_rate,
            volume,
        })
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }
}
