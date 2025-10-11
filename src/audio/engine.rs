// Moteur audio - Callback CPAL temps-réel

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use std::sync::Arc;

use crate::audio::cpu_monitor::CpuMonitor;
use crate::audio::dsp_utils::{flush_denormals_to_zero, soft_clip, OnePoleSmoother};
use crate::audio::parameters::AtomicF32;
use crate::messaging::channels::CommandConsumer;
use crate::messaging::command::Command;
use crate::midi::event::{MidiEvent, MidiEventTimed};
use crate::synth::voice_manager::VoiceManager;

pub struct AudioEngine {
    _device: Device,
    _stream: Stream,
    sample_rate: f32,
    pub volume: AtomicF32,
    pub cpu_monitor: CpuMonitor,
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
        let buffer_size = config.buffer_size.clone();

        // Calculate buffer size (default to 512 if not specified)
        let buffer_frames = match buffer_size {
            cpal::BufferSize::Fixed(size) => size as usize,
            cpal::BufferSize::Default => 512,
        };

        // Create CPU monitor (measure 1 out of 10 callbacks to minimize overhead)
        let cpu_monitor = CpuMonitor::new(sample_rate, buffer_frames, 10);
        let cpu_monitor_clone = cpu_monitor.clone();

        // Create atomic volume parameter (shared between UI and audio thread)
        let volume = AtomicF32::new(0.5); // Default volume: 50%
        let volume_clone = volume.clone();

        // Créer le VoiceManager (pré-alloué, partagé avec le callback)
        let voice_manager = Arc::new(std::sync::Mutex::new(VoiceManager::new(sample_rate)));
        let voice_manager_clone = Arc::clone(&voice_manager);

        // Créer le smoother pour le volume (10ms de smoothing pour éviter les clics)
        let volume_smoother = Arc::new(std::sync::Mutex::new(OnePoleSmoother::new(
            0.5,        // Valeur initiale (50%)
            10.0,       // 10ms time constant
            sample_rate,
        )));
        let volume_smoother_clone = Arc::clone(&volume_smoother);

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

                    // Start CPU monitoring (only samples some callbacks)
                    let measure_start = cpu_monitor_clone.start_measure();

                    // helper function to process MIDI events
                    let process_midi_event = |timed_event: MidiEventTimed, vm: &mut VoiceManager| {
                        // TODO: Implement proper scheduling based on samples_from_now
                        // For now, process immediately if samples_from_now == 0
                        if timed_event.samples_from_now == 0 {
                            match timed_event.event {
                                MidiEvent::NoteOn { note, velocity } => {
                                    vm.note_on(note, velocity);
                                }
                                MidiEvent::NoteOff { note } => {
                                    vm.note_off(note);
                                }
                                _ => {} // Ignore other events for now
                            }
                        }
                        // Events with samples_from_now > 0 are ignored for now
                        // Future: store in pre-allocated queue and process at the right time
                    };

                    // helper function to process commands
                    let process_command = |cmd: Command, vm: &mut VoiceManager| {
                        match cmd {
                            Command::Midi(timed_event) => {
                                process_midi_event(timed_event, vm);
                            }
                            Command::SetVolume(_vol) => {
                                // TODO: implement volume control
                            }
                            Command::SetWaveform(waveform) => {
                                vm.set_waveform(waveform);
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
                        // Try to get smoother (non-blocking)
                        if let Ok(mut smoother) = volume_smoother_clone.try_lock() {
                            for frame in data.chunks_mut(channels) {
                                // Read target volume from atomic (once per sample for smoothing)
                                let target_volume = volume_clone.get();

                                // Smooth volume pour éviter clics/pops
                                let smoothed_volume = smoother.process(target_volume);

                                // Generate raw sample
                                let mut sample = vm.next_sample();

                                // Anti-denormals (flush tiny values to zero)
                                sample = flush_denormals_to_zero(sample);

                                // Apply volume
                                sample *= smoothed_volume;

                                // Soft saturation (protection contre clipping dur)
                                sample = soft_clip(sample);

                                // write in all channels (mono → stereo)
                                for channel_sample in frame.iter_mut() {
                                    *channel_sample = sample;
                                }
                            }
                        } else {
                            // Fallback sans smoother (toujours mieux que silence)
                            let current_volume = volume_clone.get();
                            for frame in data.chunks_mut(channels) {
                                let mut sample = vm.next_sample();
                                sample = flush_denormals_to_zero(sample);
                                sample *= current_volume;
                                sample = soft_clip(sample);

                                for channel_sample in frame.iter_mut() {
                                    *channel_sample = sample;
                                }
                            }
                        }
                    } else {
                        // Fallback: silence if we cannot aquire the lock
                        for sample in data.iter_mut() {
                            *sample = 0.0;
                        }
                    }

                    // End CPU monitoring
                    cpu_monitor_clone.end_measure(measure_start);
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
            cpu_monitor,
        })
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }
}
