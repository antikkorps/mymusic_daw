// Moteur audio - Callback CPAL temps-réel
//
// # Format Support
//
// Ce moteur audio supporte automatiquement plusieurs formats de sample :
// - **F32**: Floating point 32-bit (natif, pas de conversion nécessaire)
// - **I16**: Signed 16-bit integer (commun sur Windows/WASAPI)
// - **U16**: Unsigned 16-bit integer (moins courant)
//
// Le système détecte automatiquement le format préféré du device audio via
// `sample_format()` et crée le stream approprié. En interne, tout le traitement
// audio se fait en f32, puis la conversion vers le format du device se fait
// au moment de l'écriture dans le buffer de sortie (sans allocation).
//
// La fonction `write_mono_to_interleaved_frame()` gère la conversion automatique
// via le trait `FromSample<f32>` de CPAL, ce qui garantit des conversions optimisées
// et conformes aux standards audio.
//
// # Stream Limitations
//
// Note: Sur macOS (CoreAudio), le Stream n'est pas Send/Sync, ce qui empêche
// la reconnexion automatique via un thread de monitoring (comme pour MIDI).
// L'error callback détecte les erreurs et envoie des notifications à l'UI,
// mais la reconnexion doit être gérée manuellement.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig};
use std::sync::{Arc, Mutex};

use crate::audio::cpu_monitor::CpuMonitor;
use crate::audio::dsp_utils::{flush_denormals_to_zero, soft_clip, OnePoleSmoother};
use crate::audio::format_conversion::write_stereo_to_interleaved_frame;
use crate::audio::parameters::AtomicF32;
use crate::connection::status::{AtomicDeviceStatus, DeviceStatus};
use crate::messaging::channels::{CommandConsumer, NotificationProducer};
use crate::messaging::command::Command;
use crate::messaging::notification::{Notification, NotificationCategory};
use crate::midi::event::{MidiEvent, MidiEventTimed};
use crate::synth::voice_manager::VoiceManager;

pub struct AudioEngine {
    _device: Device,
    _stream: Stream,
    sample_rate: f32,
    pub volume: AtomicF32,
    pub cpu_monitor: CpuMonitor,
    pub status: AtomicDeviceStatus,
}

impl AudioEngine {
    pub fn new(
        command_rx_ui: CommandConsumer,
        command_rx_midi: CommandConsumer,
        notification_tx: Arc<Mutex<NotificationProducer>>,
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
        let supported_config = device
            .default_output_config()
            .map_err(|e| format!("Erreur de configuration: {}", e))?;

        let sample_format = supported_config.sample_format();
        println!("Config audio: {:?}", supported_config);
        println!("Sample format: {:?}", sample_format);

        let sample_rate = supported_config.sample_rate().0 as f32;
        let channels = supported_config.channels() as usize;

        let config: StreamConfig = supported_config.into();
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

        // Create device status (initially disconnected, will be set to connected after stream starts)
        let status = AtomicDeviceStatus::new(DeviceStatus::Connecting);
        let status_clone = status.clone();

        // Clone notification_tx for the error callback
        let notification_tx_err = notification_tx.clone();

        // Ringbuffers pour les commandes (shared avec le callback)
        let command_rx_ui = Arc::new(std::sync::Mutex::new(command_rx_ui));
        let command_rx_ui_clone = Arc::clone(&command_rx_ui);

        let command_rx_midi = Arc::new(std::sync::Mutex::new(command_rx_midi));
        let command_rx_midi_clone = Arc::clone(&command_rx_midi);

        // Build stream based on the detected sample format
        // We match on the format and create the appropriate stream type
        let stream = match sample_format {
            SampleFormat::F32 => Self::build_stream::<f32>(
                &device,
                &config,
                channels,
                command_rx_ui_clone,
                command_rx_midi_clone,
                voice_manager_clone,
                volume_clone,
                volume_smoother_clone,
                cpu_monitor_clone,
                status_clone,
                notification_tx_err,
            ),
            SampleFormat::I16 => Self::build_stream::<i16>(
                &device,
                &config,
                channels,
                command_rx_ui_clone,
                command_rx_midi_clone,
                voice_manager_clone,
                volume_clone,
                volume_smoother_clone,
                cpu_monitor_clone,
                status_clone,
                notification_tx_err,
            ),
            SampleFormat::U16 => Self::build_stream::<u16>(
                &device,
                &config,
                channels,
                command_rx_ui_clone,
                command_rx_midi_clone,
                voice_manager_clone,
                volume_clone,
                volume_smoother_clone,
                cpu_monitor_clone,
                status_clone,
                notification_tx_err,
            ),
            _ => {
                return Err(format!(
                    "Unsupported sample format: {:?}. Supported formats: F32, I16, U16",
                    sample_format
                ))
            }
        }?;

        // Old stream creation code - REPLACED by match above
        /*
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
                                MidiEvent::ChannelAftertouch { value } => {
                                    vm.set_aftertouch(value);
                                }
                                MidiEvent::PolyAftertouch { note: _n, value: _v } => {
                                    // TODO: Poly aftertouch per-note support (Phase 2+)
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
                            Command::SetAdsr(adsr_params) => {
                                vm.set_adsr(adsr_params);
                            }
                            Command::SetLfo(lfo_params) => {
                                vm.set_lfo(lfo_params);
                            }
                            Command::SetPolyMode(poly_mode) => {
                                vm.set_poly_mode(poly_mode);
                            }
                            Command::SetPortamento(portamento_params) => {
                                vm.set_portamento(portamento_params);
                            }
                            Command::SetFilter(filter_params) => {
                                vm.set_filter(filter_params);
                            }
                            Command::SetModRouting { index, routing } => {
                                vm.set_mod_routing(index as usize, routing);
                            }
                            Command::ClearModRouting { index } => {
                                vm.clear_mod_routing(index as usize);
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
                move |err| {
                    // ========== ERROR CALLBACK ==========
                    // This runs outside the audio callback, so we can do I/O here
                    eprintln!("Audio stream error: {}", err);

                    // Set status to Error (atomic operation, safe)
                    status_clone.set(DeviceStatus::Error);

                    // Send notification to UI (non-blocking)
                    if let Ok(mut tx) = notification_tx_err.try_lock() {
                        let notif = Notification::error(
                            NotificationCategory::Audio,
                            format!("Audio stream error: {}", err),
                        );
                        let _ = ringbuf::traits::Producer::try_push(&mut *tx, notif);
                    }
                },
                None,
            )
            .map_err(|e| format!("Error in stream creation: {}", e))?;
        */

        // Start stream
        stream
            .play()
            .map_err(|e| format!("Error in stream beginning: {}", e))?;

        // Set status to Connected after successful start
        status.set(DeviceStatus::Connected);

        println!(
            "Audio engine started: {} Hz, {} canaux",
            sample_rate, channels
        );

        // Send success notification
        if let Ok(mut tx) = notification_tx.try_lock() {
            let notif = Notification::info(
                NotificationCategory::Audio,
                format!("Audio connected: {} Hz", sample_rate),
            );
            let _ = ringbuf::traits::Producer::try_push(&mut *tx, notif);
        }

        Ok(Self {
            _device: device,
            _stream: stream,
            sample_rate,
            volume,
            cpu_monitor,
            status,
        })
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Build an audio stream with automatic format conversion
    ///
    /// This is a generic helper that creates a stream for any sample type (f32, i16, u16)
    /// The audio callback generates f32 internally and converts to the target format.
    #[allow(clippy::too_many_arguments)]
    fn build_stream<T>(
        device: &Device,
        config: &StreamConfig,
        channels: usize,
        command_rx_ui: Arc<Mutex<CommandConsumer>>,
        command_rx_midi: Arc<Mutex<CommandConsumer>>,
        voice_manager: Arc<Mutex<VoiceManager>>,
        volume: AtomicF32,
        volume_smoother: Arc<Mutex<OnePoleSmoother>>,
        cpu_monitor: CpuMonitor,
        status: AtomicDeviceStatus,
        notification_tx: Arc<Mutex<NotificationProducer>>,
    ) -> Result<Stream, String>
    where
        T: SizedSample + FromSample<f32> + Send + 'static,
    {
        let stream = device
            .build_output_stream(
                config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    // ========== SACRED ZONE ==========
                    // No allocations, No I/O, No blocking locks

                    // Start CPU monitoring (only samples some callbacks)
                    let measure_start = cpu_monitor.start_measure();

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
                                MidiEvent::ChannelAftertouch { value } => {
                                    vm.set_aftertouch(value);
                                }
                                MidiEvent::PolyAftertouch { note: _n, value: _v } => {
                                    // TODO: Poly aftertouch per-note support (Phase 2+)
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
                                // Volume is handled via atomic
                            }
                            Command::SetWaveform(waveform) => {
                                vm.set_waveform(waveform);
                            }
                            Command::SetAdsr(adsr_params) => {
                                vm.set_adsr(adsr_params);
                            }
                            Command::SetLfo(lfo_params) => {
                                vm.set_lfo(lfo_params);
                            }
                            Command::SetPolyMode(poly_mode) => {
                                vm.set_poly_mode(poly_mode);
                            }
                            Command::SetPortamento(portamento_params) => {
                                vm.set_portamento(portamento_params);
                            }
                            Command::SetFilter(filter_params) => {
                                vm.set_filter(filter_params);
                            }
                            Command::SetModRouting { index, routing } => {
                                vm.set_mod_routing(index as usize, routing);
                            }
                            Command::ClearModRouting { index } => {
                                vm.clear_mod_routing(index as usize);
                            }
                            Command::SetVoiceMode(mode) => {
                                vm.set_voice_mode(mode);
                            }
                            Command::AddSample(sample) => {
                                vm.add_sample(sample);
                            }
                            Command::SetNoteSampleMapping { note, sample_index } => {
                                vm.set_note_to_sample(note, sample_index);
                            }
                            Command::UpdateSample(index, sample) => {
                                vm.update_sample(index, sample);
                            }
                            Command::Quit => {}
                        }
                    };

                    // treat UI commands
                    if let Ok(mut rx) = command_rx_ui.try_lock() {
                        if let Ok(mut vm) = voice_manager.try_lock() {
                            while let Some(cmd) = ringbuf::traits::Consumer::try_pop(&mut *rx) {
                                process_command(cmd, &mut vm);
                            }
                        }
                    }

                    // Treat MIDI commands
                    if let Ok(mut rx) = command_rx_midi.try_lock() {
                        if let Ok(mut vm) = voice_manager.try_lock() {
                            while let Some(cmd) = ringbuf::traits::Consumer::try_pop(&mut *rx) {
                                process_command(cmd, &mut vm);
                            }
                        }
                    }

                    // Generate audio samples
                    if let Ok(mut vm) = voice_manager.try_lock() {
                        // Try to get smoother (non-blocking)
                        if let Ok(mut smoother) = volume_smoother.try_lock() {
                            for frame in data.chunks_mut(channels) {
                                // Read target volume from atomic (once per sample for smoothing)
                                let target_volume = volume.get();

                                // Smooth volume pour éviter clics/pops
                                let smoothed_volume = smoother.process(target_volume);

                                // Generate stereo sample
                                let (mut left, mut right) = vm.next_sample();

                                // Anti-denormals (flush tiny values to zero)
                                left = flush_denormals_to_zero(left);
                                right = flush_denormals_to_zero(right);

                                // Apply volume
                                left *= smoothed_volume;
                                right *= smoothed_volume;

                                // Soft saturation (protection contre clipping dur)
                                left = soft_clip(left);
                                right = soft_clip(right);

                                // Write stereo sample to frame
                                write_stereo_to_interleaved_frame((left, right), frame);
                            }
                        } else {
                            // Fallback sans smoother (toujours mieux que silence)
                            let current_volume = volume.get();
                            for frame in data.chunks_mut(channels) {
                                let (mut left, mut right) = vm.next_sample();

                                left = flush_denormals_to_zero(left);
                                right = flush_denormals_to_zero(right);

                                left *= current_volume;
                                right *= current_volume;

                                left = soft_clip(left);
                                right = soft_clip(right);

                                write_stereo_to_interleaved_frame((left, right), frame);
                            }
                        }
                    } else {
                        // Fallback: silence if we cannot acquire the lock
                        for sample in data.iter_mut() {
                            *sample = Sample::from_sample::<f32>(0.0);
                        }
                    }

                    // End CPU monitoring
                    cpu_monitor.end_measure(measure_start);
                    // ========== SACRED ZONE END ==========
                },
                move |err| {
                    // ========== ERROR CALLBACK ==========
                    // This runs outside the audio callback, so we can do I/O here
                    eprintln!("Audio stream error: {}", err);

                    // Set status to Error (atomic operation, safe)
                    status.set(DeviceStatus::Error);

                    // Send notification to UI (non-blocking)
                    if let Ok(mut tx) = notification_tx.try_lock() {
                        let notif = Notification::error(
                            NotificationCategory::Audio,
                            format!("Audio stream error: {}", err),
                        );
                        let _ = ringbuf::traits::Producer::try_push(&mut *tx, notif);
                    }
                },
                None,
            )
            .map_err(|e| format!("Error in stream creation: {}", e))?;

        Ok(stream)
    }
}
