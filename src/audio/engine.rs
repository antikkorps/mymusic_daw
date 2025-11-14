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
use cpal::{Device, FromSample, SampleFormat, SizedSample, Stream, StreamConfig};
use std::sync::{Arc, Mutex};

use crate::audio::cpu_monitor::CpuMonitor;
use crate::audio::dsp_utils::{OnePoleSmoother, flush_denormals_to_zero, soft_clip};
use crate::audio::format_conversion::write_stereo_to_interleaved_frame;
use crate::audio::parameters::AtomicF32;
use crate::connection::status::{AtomicDeviceStatus, DeviceStatus};
use crate::messaging::channels::{CommandConsumer, NotificationProducer};
use crate::messaging::command::Command;
use crate::messaging::notification::{Notification, NotificationCategory};
use crate::midi::event::{MidiEvent, MidiEventTimed};
use crate::sequencer::metronome::{Metronome, MetronomeScheduler};
use crate::sequencer::timeline::{Tempo, TimeSignature};
use crate::synth::voice_manager::VoiceManager;
use crate::plugin::PluginHost;

pub struct AudioEngine {
    _device: Device,
    _stream: Stream,
    sample_rate: f32,
    pub volume: AtomicF32,
    pub cpu_monitor: CpuMonitor,
    pub status: AtomicDeviceStatus,
    pub plugin_host: Arc<PluginHost>,
}

impl AudioEngine {
    pub fn new(
        command_rx_ui: CommandConsumer,
        command_rx_midi: CommandConsumer,
        notification_tx: Arc<Mutex<NotificationProducer>>,
        plugin_host: Arc<PluginHost>,
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
        let buffer_size = config.buffer_size;

        // Calculate buffer size (default to 512 if not specified)
        let buffer_frames = match buffer_size {
            cpal::BufferSize::Fixed(size) => size as usize,
            cpal::BufferSize::Default => 512,
        };

        // Create CPU monitor (measure 1 out of 10 callbacks to minimize overhead)
        let cpu_monitor = CpuMonitor::new(sample_rate, buffer_frames, 10);
        let cpu_monitor_clone = cpu_monitor.clone();

        // Create atomic volume parameter (shared between UI and audio thread via atomic)
        let volume = AtomicF32::new(0.5); // Default volume: 50%
        let volume_clone = volume.clone();

        // Create VoiceManager (will be moved into audio callback)
        let voice_manager = VoiceManager::new(sample_rate);

        // Create volume smoother (10ms smoothing to avoid clicks, moved into callback)
        let volume_smoother = OnePoleSmoother::new(
            0.5,  // Initial value (50%)
            10.0, // 10ms time constant
            sample_rate,
        );

        // Create sequencer components (metronome + scheduler)
        let metronome = Metronome::new(sample_rate);
        let metronome_scheduler = MetronomeScheduler::new();

        // Create device status (initially connecting, atomic for UI access)
        let status = AtomicDeviceStatus::new(DeviceStatus::Connecting);
        let status_clone = status.clone();

        // Clone notification_tx for the error callback
        let notification_tx_err = notification_tx.clone();

        // Build stream based on the detected sample format
        // Each format gets its own stream with moved values (no Arc/Mutex in callback)
        let stream = match sample_format {
            SampleFormat::F32 => Self::build_stream::<f32>(
                &device,
                &config,
                channels,
                command_rx_ui,               // Moved (no Arc/Mutex)
                command_rx_midi,             // Moved (no Arc/Mutex)
                voice_manager,               // Moved (no Arc/Mutex)
                volume_clone,                // Clone (AtomicF32 is Arc internally)
                volume_smoother,             // Moved (no Arc/Mutex)
                cpu_monitor_clone,           // Clone (CpuMonitor is Arc internally for stats)
                status_clone,                // Clone (AtomicDeviceStatus is Arc internally)
                notification_tx_err,         // Clone (Arc<Mutex> only for error callback)
                metronome.clone(),           // Clone (for this stream)
                metronome_scheduler.clone(), // Clone (for this stream)
                crate::sequencer::SequencerPlayer::new(sample_rate as f64), // New instance
                sample_rate,                 // Pass sample rate for scheduler
                plugin_host.clone(),          // Clone for plugin access
            ),
            SampleFormat::I16 => Self::build_stream::<i16>(
                &device,
                &config,
                channels,
                command_rx_ui,
                command_rx_midi,
                voice_manager,
                volume_clone,
                volume_smoother,
                cpu_monitor_clone,
                status_clone,
                notification_tx_err,
                metronome.clone(),
                metronome_scheduler.clone(),
                crate::sequencer::SequencerPlayer::new(sample_rate as f64), // New instance
                sample_rate,
                plugin_host.clone(),
            ),
            SampleFormat::U16 => Self::build_stream::<u16>(
                &device,
                &config,
                channels,
                command_rx_ui,
                command_rx_midi,
                voice_manager,
                volume_clone,
                volume_smoother,
                cpu_monitor_clone,
                status_clone,
                notification_tx_err,
                metronome.clone(),
                metronome_scheduler.clone(),
                crate::sequencer::SequencerPlayer::new(sample_rate as f64), // New instance
                sample_rate,
                plugin_host.clone(),
            ),
            _ => {
                return Err(format!(
                    "Unsupported sample format: {:?}. Supported formats: F32, I16, U16",
                    sample_format
                ));
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
                            Command::SetMetronomeEnabled(_enabled) => {
                                // Metronome is handled in the metronome module
                                // TODO: Integrate with metronome state
                            }
                            Command::SetMetronomeVolume(_volume) => {
                                // Metronome is handled in the metronome module
                                // TODO: Integrate with metronome state
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
            plugin_host,
        })
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Build an audio stream with automatic format conversion (RT-safe)
    ///
    /// This is a generic helper that creates a stream for any sample type (f32, i16, u16)
    /// The audio callback generates f32 internally and converts to the target format.
    ///
    /// # RT-Safety
    /// All mutable state is moved into the closure by value (no Arc<Mutex>), ensuring:
    /// - Zero lock contention
    /// - Deterministic access times
    /// - No allocations in the audio callback
    #[allow(clippy::too_many_arguments)]
    fn build_stream<T>(
        device: &Device,
        config: &StreamConfig,
        channels: usize,
        mut command_rx_ui: CommandConsumer, // Moved into closure (no Mutex)
        mut command_rx_midi: CommandConsumer, // Moved into closure (no Mutex)
        mut voice_manager: VoiceManager,    // Moved into closure (no Mutex)
        volume: AtomicF32,                  // Clone (Arc internally, read-only atomic)
        mut volume_smoother: OnePoleSmoother, // Moved into closure (no Mutex)
        cpu_monitor: CpuMonitor,            // Clone (Arc internally for stats)
        status: AtomicDeviceStatus,         // Clone (Arc internally, atomic)
        notification_tx: Arc<Mutex<NotificationProducer>>, // Keep Mutex (only error callback)
        mut metronome: Metronome,           // Moved into closure (no Mutex)
        mut metronome_scheduler: MetronomeScheduler, // Moved into closure (no Mutex)
        mut sequencer_player: crate::sequencer::SequencerPlayer, // Moved into closure (no Mutex)
        sample_rate: f32,                   // Sample rate for scheduler calculations
        plugin_host: Arc<PluginHost>,      // Clone for plugin access
    ) -> Result<Stream, String>
    where
        T: SizedSample + FromSample<f32> + Send + 'static,
    {
        // Sequencer state (captured by closure, persists across callbacks)
        let mut current_position: u64 = 0;
        let mut current_tempo = Tempo::new(120.0);
        let mut current_time_signature = TimeSignature::four_four();
        let mut is_playing = false;

        // Active pattern for sequencer playback (default: empty pattern)
        let mut active_pattern = crate::sequencer::Pattern::new_default(1, "Empty".to_string());

        let stream = device
            .build_output_stream(
                config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    // ========== SACRED ZONE ==========
                    // No allocations, No I/O, No blocking locks

                    // Start CPU monitoring (only samples some callbacks)
                    let measure_start = cpu_monitor.start_measure();

                    // helper function to process MIDI events
                    let process_midi_event =
                        |timed_event: MidiEventTimed, vm: &mut VoiceManager, plugin_host: &PluginHost| {
                            // TODO Phase 4+: Implement proper sample-accurate scheduling
                            // For now, process all events immediately at buffer start
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
                                MidiEvent::PolyAftertouch {
                                    note: _n,
                                    value: _v,
                                } => {
                                    // TODO: Poly aftertouch per-note support (Phase 2+)
                                }
                                _ => {} // Ignore other events for now
                            }
                            
                            // Route MIDI events to all loaded plugins
                            plugin_host.process_midi_for_all_plugins(&timed_event);
                        };

                    // helper function to process commands
                    let mut process_command = |cmd: Command, vm: &mut VoiceManager| {
                        match cmd {
                            Command::Midi(timed_event) => {
                                process_midi_event(timed_event, vm, &plugin_host);
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
                            Command::RemoveSample(index) => {
                                vm.remove_sample(index);
                            }
                            Command::SetNoteSampleMapping { note, sample_index } => {
                                vm.set_note_to_sample(note, sample_index);
                            }
                            Command::UpdateSample(index, sample) => {
                                vm.update_sample(index, sample);
                            }
                            Command::SetMetronomeEnabled(enabled) => {
                                metronome.set_enabled(enabled);
                            }
                            Command::SetMetronomeVolume(volume) => {
                                metronome.set_volume(volume);
                            }
                            Command::SetTempo(bpm) => {
                                current_tempo = Tempo::new(bpm);
                            }
                            Command::SetTimeSignature(numerator, denominator) => {
                                current_time_signature = TimeSignature::new(numerator, denominator);
                            }
                            Command::SetTransportPlaying(playing) => {
                                if playing && !is_playing {
                                    // Starting playback
                                    is_playing = true;
                                } else if !playing && is_playing {
                                    // Stopping playback
                                    is_playing = false;
                                    current_position = 0;
                                    metronome_scheduler.reset();
                                }
                            }
                            Command::SetTransportPosition(position_samples) => {
                                current_position = position_samples;
                                metronome_scheduler.reset();
                            }
                            Command::SetPattern(pattern) => {
                                active_pattern = pattern;
                            }
                            Command::Quit => {}
                        }
                    };

                    // Process UI commands (direct access, no locks!)
                    while let Some(cmd) = ringbuf::traits::Consumer::try_pop(&mut command_rx_ui) {
                        process_command(cmd, &mut voice_manager);
                    }

                    // Process MIDI commands (direct access, no locks!)
                    while let Some(cmd) = ringbuf::traits::Consumer::try_pop(&mut command_rx_midi) {
                        process_command(cmd, &mut voice_manager);
                    }

                    // Process sequencer pattern (generates MIDI events from notes)
                    // IMPORTANT: Always call process() even when stopped, so it can send NoteOff events
                    let buffer_size = data.len() / channels;

                    // Generate MIDI events from pattern (RT-safe, no allocations)
                    let sequencer_events = sequencer_player.process(
                        &active_pattern,
                        current_position,
                        is_playing,
                        &current_tempo,
                        &current_time_signature,
                        buffer_size,
                    );

                    // Process generated MIDI events
                    for timed_event in sequencer_events {
                        process_midi_event(timed_event, &mut voice_manager, &plugin_host);
                    }

                    // Check for metronome clicks (if playing)
                    if is_playing {
                        let buffer_size = data.len() / channels;
                        if let Some((_offset, click_type)) = metronome_scheduler.check_for_click(
                            current_position,
                            buffer_size,
                            sample_rate as f64,
                            &current_tempo,
                            &current_time_signature,
                        ) {
                            // Trigger metronome click
                            // Note: For now, we trigger at buffer start regardless of offset
                            // TODO: Handle sample-accurate offset within buffer for perfect timing
                            metronome.trigger_click(click_type);
                        }
                    }

                    // Generate audio samples (direct access, no locks!)
                    let buffer_size = data.len() / channels;
                    
                    // Create temporary buffers for plugin processing
                    let mut input_buffers = std::collections::HashMap::new();
                    let mut output_buffers = std::collections::HashMap::new();
                    
                    // Create separate input and output buffers for plugins
                    let mut input_left = vec![0.0f32; buffer_size];
                    let mut input_right = vec![0.0f32; buffer_size];
                    let mut output_left = vec![0.0f32; buffer_size];
                    let mut output_right = vec![0.0f32; buffer_size];
                    
                    // Generate samples from voice manager and metronome into input buffers
                    for i in 0..buffer_size {
                        // Read target volume from atomic (once per sample for smoothing)
                        let target_volume = volume.get();

                        // Smooth volume to avoid clicks/pops
                        let smoothed_volume = volume_smoother.process(target_volume);

                        // Generate stereo sample
                        let (mut left, mut right) = voice_manager.next_sample();

                        // Generate metronome click sample
                        let metronome_sample = metronome.process_sample();

                        // Anti-denormals (flush tiny values to zero)
                        left = flush_denormals_to_zero(left);
                        right = flush_denormals_to_zero(right);
                        let metronome_sample = flush_denormals_to_zero(metronome_sample);

                        // Apply volume
                        left *= smoothed_volume;
                        right *= smoothed_volume;

                        // Mix in metronome (additive, doesn't affect main audio level)
                        left += metronome_sample * 0.3; // Metronome at 30% of main volume
                        right += metronome_sample * 0.3;

                        // Store in input buffers for plugins
                        input_left[i] = left;
                        input_right[i] = right;
                        
                        // Advance position counter if playing
                        if is_playing {
                            current_position += 1;
                        }
                    }
                    
                    // Create audio buffers for plugin processing
                    let mut left_input_buffer = crate::audio::buffer::AudioBuffer::new(buffer_size);
                    let mut right_input_buffer = crate::audio::buffer::AudioBuffer::new(buffer_size);
                    let mut left_output_buffer = crate::audio::buffer::AudioBuffer::new(buffer_size);
                    let mut right_output_buffer = crate::audio::buffer::AudioBuffer::new(buffer_size);
                    
                    // Copy input data to buffers
                    left_input_buffer.data_mut().copy_from_slice(&input_left);
                    right_input_buffer.data_mut().copy_from_slice(&input_right);
                    left_output_buffer.data_mut().copy_from_slice(&output_left);
                    right_output_buffer.data_mut().copy_from_slice(&output_right);
                    
                    // Set up input and output buffers for plugins
                    input_buffers.insert("input_left".to_string(), &left_input_buffer);
                    input_buffers.insert("input_right".to_string(), &right_input_buffer);
                    output_buffers.insert("output_left".to_string(), &mut left_output_buffer);
                    output_buffers.insert("output_right".to_string(), &mut right_output_buffer);
                    
                    // Process all plugins
                    if let Err(e) = plugin_host.process_all_instances(&input_buffers, &mut output_buffers, buffer_size) {
                        // Log error but continue with audio processing
                        eprintln!("Plugin processing error: {:?}", e);
                    }
                    
                    // Copy processed audio back to output buffer
                    for (i, _frame) in data.chunks_mut(channels).enumerate() {
                        let left = left_output_buffer.data()[i];
                        let right = right_output_buffer.data()[i];
                        
                        // Soft saturation (protection against hard clipping)
                        let left = soft_clip(left);
                        let right = soft_clip(right);

                        // Write stereo sample to frame
                        write_stereo_to_interleaved_frame((left, right), _frame);
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
