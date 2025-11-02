// Main UI App UI

use crate::audio::cpu_monitor::{CpuLoad, CpuMonitor};
use crate::audio::device::{AudioDeviceInfo, AudioDeviceManager};
use crate::audio::parameters::AtomicF32;
use crate::command::commands::{
    SetAdsrCommand, SetFilterCommand, SetLfoCommand, SetModRoutingCommand, SetPolyModeCommand,
    SetPortamentoCommand, SetVoiceModeCommand, SetVolumeCommand, SetWaveformCommand,
};
use crate::command::{CommandManager, DawState};
use crate::connection::status::DeviceStatus;
use crate::messaging::channels::{CommandProducer, NotificationConsumer};
use crate::messaging::command::Command;
use crate::messaging::notification::{Notification, NotificationCategory};
use crate::midi::device::{MidiDeviceInfo, MidiDeviceManager};
use crate::midi::event::{MidiEvent, MidiEventTimed};
use crate::midi::manager::MidiConnectionManager;
use crate::sampler::SampleBank;
use crate::sampler::loader::{Sample, load_sample};
use crate::sequencer::{MusicalTime, Position, Tempo, TimeSignature, Transport, TransportState};
use crate::synth::envelope::AdsrParams;
use crate::synth::filter::FilterType;
use crate::synth::lfo::{LfoDestination, LfoParams};
use crate::synth::modulation::{ModDestination, ModRouting, ModSource};
use crate::synth::oscillator::WaveformType;
use crate::synth::poly_mode::PolyMode;
use crate::synth::portamento::PortamentoParams;
use crate::synth::voice_manager::VoiceMode;
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints, VLine};
use rfd::FileDialog;
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UiTab {
    Devices,
    Synth,
    Modulation,
    Sampler,
    Sequencer,
    Play,
    Performance,
}

pub struct DawApp {
    // Command Pattern for undo/redo
    command_manager: CommandManager,
    daw_state: DawState,
    // Command producer (shared with DawState via Arc<Mutex<>>)
    command_tx: Arc<Mutex<CommandProducer>>,
    // Legacy atomic access (kept for reading current values)
    volume_atomic: AtomicF32,
    volume_ui: f32,
    active_notes: HashSet<u8>,
    // Device management
    audio_device_manager: AudioDeviceManager,
    midi_device_manager: MidiDeviceManager,
    midi_connection_manager: MidiConnectionManager,
    available_audio_devices: Vec<AudioDeviceInfo>,
    available_midi_devices: Vec<MidiDeviceInfo>,
    selected_audio_device: String,
    selected_midi_device: String,
    // Synth parameters
    selected_waveform: WaveformType,
    // ADSR UI state
    adsr_attack: f32,
    adsr_decay: f32,
    adsr_sustain: f32,
    adsr_release: f32,
    // LFO UI state
    lfo_waveform: WaveformType,
    lfo_rate: f32,
    lfo_depth: f32,
    lfo_destination: LfoDestination,
    // Polyphony mode UI state
    poly_mode: PolyMode,
    // Portamento UI state
    portamento_time: f32,
    // CPU monitoring
    cpu_monitor: CpuMonitor,
    last_cpu_load: CpuLoad,
    // Notification system
    notification_rx: NotificationConsumer,
    notification_queue: VecDeque<Notification>,
    max_notifications: usize,
    // Modulation Matrix UI (MVP) - 4 slots
    mod_routings_ui: [ModRouting; 4],
    // Sampler state
    loaded_samples: Vec<Sample>,
    note_map_input: Vec<String>,
    // Preview state (sample_index, note)
    preview_sample_note: Option<(usize, u8)>,
    preview_timer: Option<Instant>,

    // Sequencer state
    sequencer: Transport,
    metronome_enabled: bool,
    metronome_volume: f32,
    sequencer_tempo: f64,
    time_signature_numerator: u8,
    time_signature_denominator: u8,
    loop_enabled: bool,
    loop_start_bars: u32,
    loop_end_bars: u32,

    // Position cursor and snap-to-grid state
    cursor_position: Position,
    snap_to_grid_enabled: bool,
    grid_subdivision: u16, // 1=whole note, 2=half, 4=quarter, 8=eighth, 16=sixteenth

    // Piano Roll editor
    piano_roll_editor: crate::ui::piano_roll::PianoRollEditor,
    active_pattern: crate::sequencer::Pattern,

    // Active UI tab
    active_tab: UiTab,
}

impl DawApp {
    pub fn new(
        command_tx: CommandProducer,
        volume_atomic: AtomicF32,
        midi_connection_manager: MidiConnectionManager,
        cpu_monitor: CpuMonitor,
        notification_rx: NotificationConsumer,
    ) -> Self {
        let initial_volume = volume_atomic.get();

        // Initialiser les gestionnaires de périphériques
        let audio_device_manager = AudioDeviceManager::new();
        let midi_device_manager = MidiDeviceManager::new();

        // Énumérer les périphériques disponibles
        let available_audio_devices = audio_device_manager.list_output_devices();
        let available_midi_devices = midi_device_manager.list_input_ports();

        // Sélectionner les périphériques par défaut
        let selected_audio_device = available_audio_devices
            .iter()
            .find(|d| d.is_default)
            .map(|d| d.name.clone())
            .unwrap_or_default();

        // Synchroniser avec le device cible du manager MIDI
        let selected_midi_device = midi_connection_manager.target_device().unwrap_or_else(|| {
            available_midi_devices
                .iter()
                .find(|d| d.is_default)
                .map(|d| d.name.clone())
                .unwrap_or_default()
        });

        // Initialize Command Pattern with shared command producer
        let command_manager = CommandManager::new();
        let command_tx_shared = Arc::new(Mutex::new(command_tx));
        let daw_state = DawState::new(command_tx_shared.clone());

        Self {
            command_manager,
            daw_state,
            command_tx: command_tx_shared,
            volume_atomic,
            volume_ui: initial_volume,
            active_notes: HashSet::new(),
            audio_device_manager,
            midi_device_manager,
            midi_connection_manager,
            available_audio_devices,
            available_midi_devices,
            selected_audio_device,
            selected_midi_device,
            selected_waveform: WaveformType::Sine,
            adsr_attack: 0.01,
            adsr_decay: 0.1,
            adsr_sustain: 0.7,
            adsr_release: 0.2,
            lfo_waveform: WaveformType::Sine,
            lfo_rate: 5.0,
            lfo_depth: 0.5,
            lfo_destination: LfoDestination::None,
            poly_mode: PolyMode::default(),
            portamento_time: 0.0,
            cpu_monitor,
            last_cpu_load: CpuLoad::Low,
            notification_rx,
            notification_queue: VecDeque::new(),
            max_notifications: 10,
            mod_routings_ui: [
                ModRouting {
                    source: ModSource::Lfo(0),
                    destination: ModDestination::OscillatorPitch(0),
                    amount: 2.0,
                    enabled: false,
                },
                ModRouting {
                    source: ModSource::Lfo(0),
                    destination: ModDestination::Amplitude,
                    amount: 0.5,
                    enabled: false,
                },
                ModRouting {
                    source: ModSource::Velocity,
                    destination: ModDestination::Amplitude,
                    amount: 0.5,
                    enabled: false,
                },
                ModRouting {
                    source: ModSource::Aftertouch,
                    destination: ModDestination::Amplitude,
                    amount: 0.5,
                    enabled: false,
                },
            ],
            loaded_samples: Vec::new(),
            note_map_input: Vec::new(),
            preview_sample_note: None,
            preview_timer: None,

            // Sequencer initialization (using 48kHz default sample rate)
            sequencer: Transport::new(48000.0),
            metronome_enabled: true,
            metronome_volume: 0.5,
            sequencer_tempo: 120.0,
            time_signature_numerator: 4,
            time_signature_denominator: 4,
            loop_enabled: false,
            loop_start_bars: 1,
            loop_end_bars: 8,

            // Initialize cursor position and snap-to-grid
            cursor_position: Position::zero(),
            snap_to_grid_enabled: true,
            grid_subdivision: 4, // Default to quarter note snap

            // Initialize piano roll with a default 4-bar pattern
            piano_roll_editor: crate::ui::piano_roll::PianoRollEditor::default(),
            active_pattern: crate::sequencer::Pattern::new_default(1, "Pattern 1".to_string()),

            active_tab: UiTab::Synth,
        }
    }

    fn refresh_devices(&mut self) {
        self.available_audio_devices = self.audio_device_manager.list_output_devices();
        self.available_midi_devices = self.midi_device_manager.list_input_ports();
    }

    /// Lit les nouvelles notifications depuis le ringbuffer et les ajoute à la queue
    fn update_notifications(&mut self) {
        // Lire toutes les notifications disponibles
        while let Some(notification) = ringbuf::traits::Consumer::try_pop(&mut self.notification_rx)
        {
            self.notification_queue.push_back(notification);

            // Limiter la taille de la queue
            if self.notification_queue.len() > self.max_notifications {
                self.notification_queue.pop_front();
            }
        }
    }

    /// Récupère la notification la plus récente (si elle existe)
    fn _get_latest_notification(&self) -> Option<&Notification> {
        self.notification_queue.back()
    }

    /// Récupère toutes les notifications récentes (moins de 5 secondes)
    fn get_recent_notifications(&self) -> Vec<&Notification> {
        self.notification_queue
            .iter()
            .rev()
            .filter(|n| n.is_recent(5000))
            .take(3)
            .collect()
    }

    /// Vérifie la charge CPU et envoie une notification si elle devient élevée
    fn check_cpu_load(&mut self) {
        let current_load = self.cpu_monitor.get_load_level();

        // Envoyer une notification seulement lors de la transition vers High
        if matches!(current_load, CpuLoad::High) && !matches!(self.last_cpu_load, CpuLoad::High) {
            let cpu_percentage = self.cpu_monitor.get_cpu_percentage();
            let notification = Notification::warning(
                NotificationCategory::Cpu,
                format!("High CPU load: {:.1}%", cpu_percentage),
            );
            self.notification_queue.push_back(notification);
        }

        self.last_cpu_load = current_load;
    }

    fn send_note_on(&mut self, note: u8) {
        if self.active_notes.insert(note) {
            let timed_event = MidiEventTimed {
                event: MidiEvent::NoteOn {
                    note,
                    velocity: 100,
                },
                samples_from_now: 0, // Immediate processing from UI
            };
            let cmd = Command::Midi(timed_event);
            if let Ok(mut tx) = self.command_tx.lock() {
                let _ = ringbuf::traits::Producer::try_push(&mut *tx, cmd);
            }
        }
    }

    fn send_note_off(&mut self, note: u8) {
        if self.active_notes.remove(&note) {
            let timed_event = MidiEventTimed {
                event: MidiEvent::NoteOff { note },
                samples_from_now: 0, // Immediate processing from UI
            };
            let cmd = Command::Midi(timed_event);
            if let Ok(mut tx) = self.command_tx.lock() {
                let _ = ringbuf::traits::Producer::try_push(&mut *tx, cmd);
            }
        }
    }

    /// Preview a sample by triggering a note (C4 = 60)
    fn preview_sample(&mut self, sample_index: usize) {
        // Stop any ongoing preview
        if let Some((_, prev_note)) = self.preview_sample_note {
            self.send_note_off_direct(prev_note);
        }

        // Use C4 (note 60) for preview
        let preview_note = 60;

        // Send note on
        let timed_event = MidiEventTimed {
            event: MidiEvent::NoteOn {
                note: preview_note,
                velocity: 100,
            },
            samples_from_now: 0,
        };
        let cmd = Command::Midi(timed_event);
        if let Ok(mut tx) = self.command_tx.lock() {
            let _ = ringbuf::traits::Producer::try_push(&mut *tx, cmd);
        }

        // Track preview state with a 2-second timer
        self.preview_sample_note = Some((sample_index, preview_note));
        self.preview_timer = Some(Instant::now());
    }

    /// Send note off without tracking in active_notes (for preview)
    fn send_note_off_direct(&mut self, note: u8) {
        let timed_event = MidiEventTimed {
            event: MidiEvent::NoteOff { note },
            samples_from_now: 0,
        };
        let cmd = Command::Midi(timed_event);
        if let Ok(mut tx) = self.command_tx.lock() {
            let _ = ringbuf::traits::Producer::try_push(&mut *tx, cmd);
        }
    }

    /// Check if preview timer has expired and stop preview if needed
    fn check_preview_timer(&mut self) {
        if let Some(timer) = self.preview_timer
            && timer.elapsed().as_secs_f32() > 2.0
        {
            // Stop preview after 2 seconds
            if let Some((_, note)) = self.preview_sample_note {
                self.send_note_off_direct(note);
            }
            self.preview_sample_note = None;
            self.preview_timer = None;
        }
    }

    /// Snap position to grid if enabled
    fn snap_to_grid(&self, position: Position) -> Position {
        if !self.snap_to_grid_enabled {
            return position;
        }

        let time_signature = TimeSignature::new(
            self.time_signature_numerator,
            self.time_signature_denominator,
        );

        let quantized_musical = position
            .musical
            .quantize_to_subdivision(&time_signature, self.grid_subdivision);

        Position::from_musical(
            quantized_musical,
            self.sequencer.sample_rate(),
            self.sequencer.tempo(),
            &time_signature,
        )
    }

    /// Update cursor position from sequencer current position
    fn update_cursor_position(&mut self) {
        self.cursor_position = self.sequencer.position();
    }

    /// Draw timeline with cursor and grid
    fn draw_timeline_with_cursor(&mut self, ui: &mut egui::Ui) {
        let available_width = ui.available_width();
        let timeline_height = 100.0;

        // Update cursor position from sequencer
        self.update_cursor_position();

        // Draw timeline background
        let painter = ui.painter();
        let rect = egui::Rect::from_min_size(
            ui.cursor().min,
            egui::vec2(available_width, timeline_height),
        );
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(40, 40, 40));

        // Calculate pixels per bar (show 8 bars by default)
        let bars_to_show = 8;
        let pixels_per_bar = available_width / bars_to_show as f32;

        // Draw grid lines and bar numbers
        let time_signature = TimeSignature::new(
            self.time_signature_numerator,
            self.time_signature_denominator,
        );

        for bar in 0..=bars_to_show {
            let x = rect.min.x + (bar as f32 * pixels_per_bar);

            // Bar lines (thicker)
            painter.line_segment(
                [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 80, 80)),
            );

            // Bar numbers
            if bar < bars_to_show {
                let bar_number = self.cursor_position.musical.bar + bar;
                painter.text(
                    egui::pos2(x + 5.0, rect.min.y + 5.0),
                    egui::Align2::LEFT_TOP,
                    format!("Bar {}", bar_number),
                    egui::FontId::default(),
                    egui::Color32::from_rgb(200, 200, 200),
                );
            }

            // Draw beat lines if space permits
            if pixels_per_bar >= 80.0 {
                for beat in 1..time_signature.numerator {
                    let beat_x =
                        x + (beat as f32 * pixels_per_bar / time_signature.numerator as f32);
                    painter.line_segment(
                        [
                            egui::pos2(beat_x, rect.min.y),
                            egui::pos2(beat_x, rect.max.y),
                        ],
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 60)),
                    );
                }
            }

            // Draw subdivision lines if snap is enabled and space permits
            if self.snap_to_grid_enabled && pixels_per_bar >= 160.0 {
                let subdivisions_per_beat = self.grid_subdivision;
                if subdivisions_per_beat > 1 {
                    for subdivision in 0..subdivisions_per_beat {
                        let subdivision_x = x
                            + (subdivision as f32 * pixels_per_bar / subdivisions_per_beat as f32);
                        painter.line_segment(
                            [
                                egui::pos2(subdivision_x, rect.min.y),
                                egui::pos2(subdivision_x, rect.max.y),
                            ],
                            egui::Stroke::new(0.5, egui::Color32::from_rgb(50, 50, 50)),
                        );
                    }
                }
            }
        }

        // Draw cursor position
        let cursor_bar_offset = (self.cursor_position.musical.bar - 1) % bars_to_show;
        let cursor_beat_offset =
            (self.cursor_position.musical.beat - 1) as f32 / time_signature.numerator as f32;
        let cursor_tick_offset = self.cursor_position.musical.tick as f32
            / (MusicalTime::TICKS_PER_QUARTER as f32 * time_signature.numerator as f32);

        let cursor_x = rect.min.x
            + (cursor_bar_offset as f32 + cursor_beat_offset + cursor_tick_offset) * pixels_per_bar;

        // Only draw cursor if within visible range
        if cursor_x >= rect.min.x && cursor_x <= rect.max.x {
            painter.line_segment(
                [
                    egui::pos2(cursor_x, rect.min.y),
                    egui::pos2(cursor_x, rect.max.y),
                ],
                egui::Stroke::new(2.0, egui::Color32::RED),
            );

            // Cursor position text
            painter.text(
                egui::pos2(cursor_x + 5.0, rect.max.y - 20.0),
                egui::Align2::LEFT_BOTTOM,
                format!("{}", self.cursor_position.musical),
                egui::FontId::default(),
                egui::Color32::RED,
            );
        }

        // Handle mouse interaction for cursor positioning
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(available_width, timeline_height),
            egui::Sense::click(),
        );

        if response.clicked()
            && let Some(pointer_pos) = response.interact_pointer_pos()
        {
            // Calculate clicked position in timeline
            let relative_x = pointer_pos.x - rect.min.x;
            let bar_offset = relative_x / pixels_per_bar;

            // Convert to musical time
            let clicked_bar = self.cursor_position.musical.bar - 1 + bar_offset as u32;
            let clicked_beat = ((bar_offset % 1.0) * time_signature.numerator as f32) as u8 + 1;
            let clicked_tick = ((bar_offset % 1.0 * time_signature.numerator as f32) % 1.0
                * MusicalTime::TICKS_PER_QUARTER as f32) as u16;

            let mut clicked_musical = MusicalTime::new(
                clicked_bar.max(1),
                clicked_beat.min(time_signature.numerator),
                clicked_tick,
            );

            // Apply snap-to-grid if enabled
            if self.snap_to_grid_enabled {
                clicked_musical =
                    clicked_musical.quantize_to_subdivision(&time_signature, self.grid_subdivision);
            }

            // Create new position and set it
            let new_position = Position::from_musical(
                clicked_musical,
                self.sequencer.sample_rate(),
                self.sequencer.tempo(),
                &time_signature,
            );

            self.sequencer.set_position(new_position);
            self.cursor_position = new_position;

            // Send position update to audio thread
            let cmd = Command::SetTransportPosition(new_position.samples);
            if let Ok(mut tx) = self.command_tx.lock() {
                let _ = ringbuf::traits::Producer::try_push(&mut *tx, cmd);
            }
        }
    }

    /// Handle PC keyboard input globally (independent of the current tab)
    ///
    /// This allows playing notes while editing other sections.
    /// Does not render any UI.
    fn process_pc_keyboard_input(&mut self, ctx: &egui::Context) {
        // Avoid capturing keys when UI requests text input (e.g., text fields)
        if ctx.wants_keyboard_input() {
            return;
        }

        // Mapping QWERTY keyboard → MIDI notes (C4 = 60)
        let key_map = [
            ('a', 60), // C4
            ('w', 61), // C#4
            ('s', 62), // D4
            ('e', 63), // D#4
            ('d', 64), // E4
            ('f', 65), // F4
            ('t', 66), // F#4
            ('g', 67), // G4
            ('y', 68), // G#4
            ('h', 69), // A4
            ('u', 70), // A#4
            ('j', 71), // B4
            ('k', 72), // C5
        ];

        for (key, note) in &key_map {
            let key_code =
                egui::Key::from_name(&key.to_string().to_uppercase()).unwrap_or(egui::Key::A);
            if ctx.input(|i| i.key_pressed(key_code)) {
                self.send_note_on(*note);
            }
            if ctx.input(|i| i.key_released(key_code)) {
                self.send_note_off(*note);
            }
        }
    }

    /// Render the visual virtual keyboard (no input handling)
    fn draw_keyboard_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Virtual keyboard");
        ui.label("Use the keyboard keys to play the notes:");
        ui.label("A W S E D F T G Y H U J K = notes (Do to Do)");
        ui.add_space(10.0);

        // Mapping QWERTY keyboard → MIDI notes (C4 = 60)
        let key_map = [
            ('a', 60), // C4
            ('w', 61), // C#4
            ('s', 62), // D4
            ('e', 63), // D#4
            ('d', 64), // E4
            ('f', 65), // F4
            ('t', 66), // F#4
            ('g', 67), // G4
            ('y', 68), // G#4
            ('h', 69), // A4
            ('u', 70), // A#4
            ('j', 71), // B4
            ('k', 72), // C5
        ];

        // Display the visual keyboard only
        ui.horizontal(|ui| {
            for (key, note) in &key_map {
                let is_active = self.active_notes.contains(note);
                let is_black = matches!(key, 'w' | 'e' | 't' | 'y' | 'u');

                let note_name = match note % 12 {
                    0 => "C",
                    1 => "C#",
                    2 => "D",
                    3 => "D#",
                    4 => "E",
                    5 => "F",
                    6 => "F#",
                    7 => "G",
                    8 => "G#",
                    9 => "A",
                    10 => "A#",
                    11 => "B",
                    _ => "?",
                };

                let octave = note / 12 - 1;
                let label = format!("{}{}\n({})", note_name, octave, key.to_uppercase());

                let button = if is_black {
                    egui::Button::new(label)
                        .fill(if is_active {
                            egui::Color32::from_rgb(100, 100, 255)
                        } else {
                            egui::Color32::from_gray(50)
                        })
                        .min_size(egui::vec2(50.0, 80.0))
                } else {
                    egui::Button::new(label)
                        .fill(if is_active {
                            egui::Color32::from_rgb(150, 150, 255)
                        } else {
                            egui::Color32::WHITE
                        })
                        .stroke(egui::Stroke::new(1.0, egui::Color32::BLACK))
                        .min_size(egui::vec2(50.0, 80.0))
                };

                if ui.add(button).clicked() {
                    // Toggle note on click
                    if is_active {
                        self.send_note_off(*note);
                    } else {
                        self.send_note_on(*note);
                    }
                }
            }
        });

        ui.add_space(10.0);
        ui.label(format!("Notes actives : {}", self.active_notes.len()));
    }

    /// Affiche la barre de statut en bas de la fenêtre
    fn draw_status_bar(&self, ui: &mut egui::Ui) {
        ui.separator();
        ui.horizontal(|ui| {
            // Afficher les notifications récentes (moins de 5s)
            let recent_notifications = self.get_recent_notifications();

            if recent_notifications.is_empty() {
                ui.label("Ready");
            } else {
                for notification in recent_notifications {
                    // Couleur selon le niveau
                    let (icon, color) = match notification.level {
                        crate::messaging::notification::NotificationLevel::Info => {
                            ("ℹ", egui::Color32::from_rgb(100, 150, 255))
                        }
                        crate::messaging::notification::NotificationLevel::Warning => {
                            ("⚠", egui::Color32::from_rgb(255, 165, 0))
                        }
                        crate::messaging::notification::NotificationLevel::Error => {
                            ("✖", egui::Color32::RED)
                        }
                    };

                    ui.colored_label(color, icon);
                    ui.colored_label(color, &notification.message);
                    ui.add_space(10.0);
                }
            }
        });
    }

    /// Save current sample bank to file
    fn save_sample_bank(&self, path: &std::path::Path) -> Result<(), String> {
        let bank_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled Bank")
            .to_string();

        // Convert note_map_input to expected format
        let note_mappings: Vec<Option<String>> = (0..128)
            .map(|i| {
                if i < self.note_map_input.len() && !self.note_map_input[i].is_empty() {
                    Some(self.note_map_input[i].clone())
                } else {
                    None
                }
            })
            .collect();

        let bank = SampleBank::from_samples_and_mappings(
            bank_name,
            &self.loaded_samples,
            &note_mappings,
            path.parent().unwrap_or_else(|| std::path::Path::new(".")),
        );

        bank.save_to_file(path)
    }

    /// Load sample bank from file
    fn load_sample_bank(&mut self, path: &std::path::Path) -> Result<(), String> {
        let bank = SampleBank::load_from_file(path)?;

        // Clear current samples and mappings
        self.loaded_samples.clear();
        self.note_map_input.clear();

        // Get base directory for resolving relative paths
        let base_dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));

        // Load samples from bank
        for mapping in bank.get_sorted_mappings() {
            let sample_path = if mapping.sample_path.is_absolute() {
                mapping.sample_path.clone()
            } else {
                base_dir.join(&mapping.sample_path)
            };

            match load_sample(&sample_path) {
                Ok(mut sample) => {
                    // Apply bank settings to sample
                    sample.name = mapping.name.clone();
                    sample.volume = mapping.volume;
                    sample.pan = mapping.pan;
                    sample.loop_mode = mapping.loop_mode;
                    sample.loop_start = mapping.loop_start;
                    sample.loop_end = mapping.loop_end;
                    sample.reverse = mapping.reverse;
                    sample.pitch_offset = mapping.pitch_offset;

                    // Clone sample: one for UI, one for audio thread
                    let sample_for_audio = Arc::new(sample.clone());
                    let cmd = Command::AddSample(sample_for_audio);
                    if let Ok(mut tx) = self.command_tx.lock()
                        && ringbuf::traits::Producer::try_push(&mut *tx, cmd).is_err()
                    {
                        eprintln!("Failed to send AddSample command: ringbuffer full");
                    }

                    self.loaded_samples.push(sample);

                    // Extend note_map_input if needed
                    while self.note_map_input.len() <= mapping.note as usize {
                        self.note_map_input.push(String::new());
                    }

                    self.note_map_input[mapping.note as usize] = mapping.note.to_string();

                    // Send note mapping command
                    let cmd = Command::SetNoteSampleMapping {
                        note: mapping.note,
                        sample_index: self.loaded_samples.len() - 1,
                    };
                    if let Ok(mut tx) = self.command_tx.lock()
                        && ringbuf::traits::Producer::try_push(&mut *tx, cmd).is_err()
                    {
                        eprintln!("Failed to send SetNoteSampleMapping command: ringbuffer full");
                    }
                }
                Err(e) => {
                    eprintln!("Failed to load sample '{}': {}", mapping.name, e);
                    // Continue loading other samples instead of failing completely
                }
            }
        }

        Ok(())
    }
}

impl eframe::App for DawApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Ask for a refresh to capture keyboard events
        ctx.request_repaint();

        // Always process PC keyboard input, regardless of the current tab
        self.process_pc_keyboard_input(ctx);

        // Check if preview timer has expired
        self.check_preview_timer();

        // Handle Undo/Redo keyboard shortcuts
        ctx.input(|i| {
            // Ctrl+Z for Undo
            if i.modifiers.command
                && i.key_pressed(egui::Key::Z)
                && !i.modifiers.shift
                && self.command_manager.can_undo()
            {
                match self.command_manager.undo(&mut self.daw_state) {
                    Ok(description) => {
                        // Update UI state from DawState after undo
                        self.volume_ui = self.daw_state.volume;
                        self.selected_waveform = self.daw_state.waveform;
                        self.adsr_attack = self.daw_state.adsr.attack;
                        self.adsr_decay = self.daw_state.adsr.decay;
                        self.adsr_sustain = self.daw_state.adsr.sustain;
                        self.adsr_release = self.daw_state.adsr.release;
                        self.lfo_waveform = self.daw_state.lfo.waveform;
                        self.lfo_rate = self.daw_state.lfo.rate;
                        self.lfo_depth = self.daw_state.lfo.depth;
                        self.lfo_destination = self.daw_state.lfo.destination;
                        self.poly_mode = self.daw_state.poly_mode;
                        self.portamento_time = self.daw_state.portamento.time;
                        // Sync modulation UI from state mirror
                        for idx in 0..self.mod_routings_ui.len() {
                            self.mod_routings_ui[idx] = self.daw_state.mod_routings[idx];
                        }
                        self.volume_atomic.set(self.daw_state.volume);
                        println!("Undo: {}", description);
                    }
                    Err(e) => eprintln!("Undo failed: {}", e),
                }
            }

            // Ctrl+Shift+Z or Ctrl+Y for Redo
            if ((i.modifiers.command && i.key_pressed(egui::Key::Z) && i.modifiers.shift)
                || (i.modifiers.command && i.key_pressed(egui::Key::Y)))
                && self.command_manager.can_redo()
            {
                match self.command_manager.redo(&mut self.daw_state) {
                    Ok(description) => {
                        // Update UI state from DawState after redo
                        self.volume_ui = self.daw_state.volume;
                        self.selected_waveform = self.daw_state.waveform;
                        self.adsr_attack = self.daw_state.adsr.attack;
                        self.adsr_decay = self.daw_state.adsr.decay;
                        self.adsr_sustain = self.daw_state.adsr.sustain;
                        self.adsr_release = self.daw_state.adsr.release;
                        self.lfo_waveform = self.daw_state.lfo.waveform;
                        self.lfo_rate = self.daw_state.lfo.rate;
                        self.lfo_depth = self.daw_state.lfo.depth;
                        self.lfo_destination = self.daw_state.lfo.destination;
                        self.poly_mode = self.daw_state.poly_mode;
                        self.portamento_time = self.daw_state.portamento.time;
                        // Sync modulation UI from state mirror
                        for idx in 0..self.mod_routings_ui.len() {
                            self.mod_routings_ui[idx] = self.daw_state.mod_routings[idx];
                        }
                        self.volume_atomic.set(self.daw_state.volume);
                        println!("Redo: {}", description);
                    }
                    Err(e) => eprintln!("Redo failed: {}", e),
                }
            }
        });

        // Update notifications from ringbuffer
        self.update_notifications();

        // Check CPU load and notify if high
        self.check_cpu_load();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("MyMusic DAW - MVP");
            ui.separator();

            // Simple tab bar (no scrolling): show one category at a time
            ui.horizontal(|ui| {
                let button = |ui: &mut egui::Ui, label: &str, tab: UiTab, current: &mut UiTab| {
                    let selected = *current == tab;
                    if ui.selectable_label(selected, label).clicked() {
                        *current = tab;
                    }
                };
                button(ui, "Devices", UiTab::Devices, &mut self.active_tab);
                button(ui, "Synth", UiTab::Synth, &mut self.active_tab);
                button(ui, "Modulation", UiTab::Modulation, &mut self.active_tab);
                button(ui, "Sampler", UiTab::Sampler, &mut self.active_tab);
                button(ui, "Sequencer", UiTab::Sequencer, &mut self.active_tab);
                button(ui, "Play", UiTab::Play, &mut self.active_tab);
                button(ui, "Performance", UiTab::Performance, &mut self.active_tab);
            });

            ui.separator();

            match self.active_tab {
                UiTab::Devices => {
                    // Devices tab
                    ui.heading("Devices");

                    ui.horizontal(|ui| {
                        ui.label("MIDI Input:");

                        // Status indicator avec couleur
                        let midi_status = self.midi_connection_manager.status();
                        let (status_text, status_color) = match midi_status {
                            DeviceStatus::Connected => ("●", egui::Color32::GREEN),
                            DeviceStatus::Connecting => ("●", egui::Color32::YELLOW),
                            DeviceStatus::Disconnected => ("○", egui::Color32::GRAY),
                            DeviceStatus::Error => ("●", egui::Color32::RED),
                        };
                        ui.colored_label(status_color, status_text);

                        let previous_device = self.selected_midi_device.clone();
                        egui::ComboBox::from_id_salt("midi_device_selector")
                            .selected_text(&self.selected_midi_device)
                            .show_ui(ui, |ui| {
                                if self.available_midi_devices.is_empty() {
                                    ui.label("No MIDI device available");
                                } else {
                                    for device in &self.available_midi_devices {
                                        let label = if device.is_default {
                                            format!("{} (default)", device.name)
                                        } else {
                                            device.name.clone()
                                        };
                                        ui.selectable_value(
                                            &mut self.selected_midi_device,
                                            device.name.clone(),
                                            label,
                                        );
                                    }
                                }
                            });

                        // Si le device a changé, déclencher la reconnexion
                        if previous_device != self.selected_midi_device {
                            self.midi_connection_manager
                                .set_target_device(self.selected_midi_device.clone());
                        }

                        if ui.button("🔄").on_hover_text("Refresh devices").clicked() {
                            self.refresh_devices();
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Audio Output:");
                        egui::ComboBox::from_id_salt("audio_device_selector")
                            .selected_text(&self.selected_audio_device)
                            .show_ui(ui, |ui| {
                                if self.available_audio_devices.is_empty() {
                                    ui.label("No audio device available");
                                } else {
                                    for device in &self.available_audio_devices {
                                        let label = if device.is_default {
                                            format!("{} (default)", device.name)
                                        } else {
                                            device.name.clone()
                                        };
                                        ui.selectable_value(
                                            &mut self.selected_audio_device,
                                            device.name.clone(),
                                            label,
                                        );
                                    }
                                }
                            });
                    });
                }
                UiTab::Modulation => {
                    // Modulation tab
                    ui.heading("Modulation Matrix (MVP)");

                    let src_labels = ["LFO 1", "Velocity", "Aftertouch", "Envelope"];
                    let dst_labels = ["Pitch", "Amplitude", "Pan"];

                    for (i, routing) in self.mod_routings_ui.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("Slot {}:", i + 1));

                            // Enabled toggle
                            let mut enabled = routing.enabled;
                            if ui.checkbox(&mut enabled, "On").changed() {
                                let old = *routing;
                                routing.enabled = enabled;
                                let cmd = Box::new(SetModRoutingCommand::new_with_old(
                                    i as u8, *routing, old,
                                ));
                                let _ = self.command_manager.execute(cmd, &mut self.daw_state);
                            }

                            // Source selector
                            let prev_source = routing.source;
                            egui::ComboBox::from_id_salt(format!("mod_src_{}", i))
                                .selected_text(match routing.source {
                                    ModSource::Lfo(0) => src_labels[0],
                                    ModSource::Velocity => src_labels[1],
                                    ModSource::Aftertouch => src_labels[2],
                                    ModSource::Envelope => src_labels[3],
                                    _ => "Unused",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut routing.source,
                                        ModSource::Lfo(0),
                                        src_labels[0],
                                    );
                                    ui.selectable_value(
                                        &mut routing.source,
                                        ModSource::Velocity,
                                        src_labels[1],
                                    );
                                    ui.selectable_value(
                                        &mut routing.source,
                                        ModSource::Aftertouch,
                                        src_labels[2],
                                    );
                                    ui.selectable_value(
                                        &mut routing.source,
                                        ModSource::Envelope,
                                        src_labels[3],
                                    );
                                });
                            if routing.source != prev_source {
                                let old = ModRouting {
                                    source: prev_source,
                                    ..*routing
                                };
                                let cmd = Box::new(SetModRoutingCommand::new_with_old(
                                    i as u8, *routing, old,
                                ));
                                let _ = self.command_manager.execute(cmd, &mut self.daw_state);
                            }

                            // Destination selector
                            let prev_dest = routing.destination;
                            egui::ComboBox::from_id_salt(format!("mod_dst_{}", i))
                                .selected_text(match routing.destination {
                                    ModDestination::OscillatorPitch(0) => dst_labels[0],
                                    ModDestination::Amplitude => dst_labels[1],
                                    ModDestination::Pan => dst_labels[2],
                                    _ => "Unused",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut routing.destination,
                                        ModDestination::OscillatorPitch(0),
                                        dst_labels[0],
                                    );
                                    ui.selectable_value(
                                        &mut routing.destination,
                                        ModDestination::Amplitude,
                                        dst_labels[1],
                                    );
                                    ui.selectable_value(
                                        &mut routing.destination,
                                        ModDestination::Pan,
                                        dst_labels[2],
                                    );
                                });
                            if routing.destination != prev_dest {
                                let old = ModRouting {
                                    destination: prev_dest,
                                    ..*routing
                                };
                                let cmd = Box::new(SetModRoutingCommand::new_with_old(
                                    i as u8, *routing, old,
                                ));
                                let _ = self.command_manager.execute(cmd, &mut self.daw_state);
                            }

                            // Amount slider
                            let prev_amount = routing.amount;
                            let range = match routing.destination {
                                ModDestination::OscillatorPitch(_) => -12.0..=12.0, // semitones
                                ModDestination::Amplitude => -1.0..=1.0,            // multiplier delta
                                ModDestination::Pan => -1.0..=1.0,                  // pan L/R
                                ModDestination::FilterCutoff => 0.0..=10.0, // cutoff multiplier (0.1x to 10x)
                            };
                            if ui
                                .add(egui::Slider::new(&mut routing.amount, range).fixed_decimals(2))
                                .changed()
                            {
                                let old = *routing;
                                // Clamp for safety
                                routing.amount = match routing.destination {
                                    ModDestination::OscillatorPitch(_) => {
                                        routing.amount.clamp(-24.0, 24.0)
                                    }
                                    _ => routing.amount.clamp(-1.0, 1.0), // For Amplitude and Pan
                                };
                                let cmd = Box::new(SetModRoutingCommand::new_with_old(
                                    i as u8, *routing, old,
                                ));
                                let _ = self.command_manager.execute(cmd, &mut self.daw_state);
                            } else if (routing.amount - prev_amount).abs() > 0.0 {
                                // no-op
                            }
                        });
                    }

                    ui.label("Sources are normalized to [-1,1]; pitch amount is semitones.");
                    ui.label("Aftertouch requires a controller that sends Channel Pressure.");

                    ui.add_space(10.0);
                    ui.separator();

                    // LFO controls
                    ui.heading("LFO (Modulation)");
                    ui.horizontal(|ui| {
                        ui.label("LFO Waveform:");
                        let previous_lfo_waveform = self.lfo_waveform;
                        egui::ComboBox::from_id_salt("lfo_waveform_selector")
                            .selected_text(match self.lfo_waveform {
                                WaveformType::Sine => "Sine",
                                WaveformType::Square => "Square",
                                WaveformType::Saw => "Saw",
                                WaveformType::Triangle => "Triangle",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.lfo_waveform, WaveformType::Sine, "Sine");
                                ui.selectable_value(
                                    &mut self.lfo_waveform,
                                    WaveformType::Square,
                                    "Square",
                                );
                                ui.selectable_value(&mut self.lfo_waveform, WaveformType::Saw, "Saw");
                                ui.selectable_value(
                                    &mut self.lfo_waveform,
                                    WaveformType::Triangle,
                                    "Triangle",
                                );
                            });

                        if previous_lfo_waveform != self.lfo_waveform {
                            let params = LfoParams::new(
                                self.lfo_waveform,
                                self.lfo_rate,
                                self.lfo_depth,
                                self.lfo_destination,
                            );
                            let cmd = Box::new(SetLfoCommand::new(params));
                            let _ = self.command_manager.execute(cmd, &mut self.daw_state);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("LFO Rate:");
                        if ui
                            .add(
                                egui::Slider::new(&mut self.lfo_rate, 0.1..=20.0)
                                    .text("Hz")
                                    .logarithmic(true),
                            )
                            .changed()
                        {
                            let params = LfoParams::new(
                                self.lfo_waveform,
                                self.lfo_rate,
                                self.lfo_depth,
                                self.lfo_destination,
                            );
                            let cmd = Box::new(SetLfoCommand::new(params));
                            let _ = self.command_manager.execute(cmd, &mut self.daw_state);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("LFO Depth:");
                        if ui
                            .add(egui::Slider::new(&mut self.lfo_depth, 0.0..=1.0))
                            .changed()
                        {
                            let params = LfoParams::new(
                                self.lfo_waveform,
                                self.lfo_rate,
                                self.lfo_depth,
                                self.lfo_destination,
                            );
                            let cmd = Box::new(SetLfoCommand::new(params));
                            let _ = self.command_manager.execute(cmd, &mut self.daw_state);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("LFO Destination:");
                        let previous_destination = self.lfo_destination;
                        egui::ComboBox::from_id_salt("lfo_destination_selector")
                            .selected_text(match self.lfo_destination {
                                LfoDestination::None => "None",
                                LfoDestination::Pitch => "Pitch (Vibrato)",
                                LfoDestination::Volume => "Volume (Tremolo)",
                                LfoDestination::FilterCutoff => "Filter Cutoff (Phase 3a)",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.lfo_destination,
                                    LfoDestination::None,
                                    "None",
                                );
                                ui.selectable_value(
                                    &mut self.lfo_destination,
                                    LfoDestination::Pitch,
                                    "Pitch (Vibrato)",
                                );
                                ui.selectable_value(
                                    &mut self.lfo_destination,
                                    LfoDestination::Volume,
                                    "Volume (Tremolo)",
                                );
                                ui.selectable_value(
                                    &mut self.lfo_destination,
                                    LfoDestination::FilterCutoff,
                                    "Filter Cutoff (Phase 3a)",
                                );
                            });

                        if previous_destination != self.lfo_destination {
                            let params = LfoParams::new(
                                self.lfo_waveform,
                                self.lfo_rate,
                                self.lfo_depth,
                                self.lfo_destination,
                            );
                            let cmd = Box::new(SetLfoCommand::new(params));
                            let _ = self.command_manager.execute(cmd, &mut self.daw_state);
                        }
                    });
                }
                UiTab::Sampler => {
                    ui.heading("Sampler");
                    // Sample bank management
                    ui.horizontal(|ui| {
                        if ui.button("Load Sample").clicked() {
                            let file = FileDialog::new()
                                .add_filter("Audio Files", &["wav", "flac", "mp3"])
                                .pick_file();

                            if let Some(path) = file {
                                match load_sample(&path) {
                                    Ok(sample) => {
                    // Clone sample: one for UI, one for audio thread
                    let sample_for_audio = Arc::new(sample.clone());
                    let cmd = Command::AddSample(sample_for_audio);
                    if let Ok(mut tx) = self.command_tx.lock()
                        && ringbuf::traits::Producer::try_push(&mut *tx, cmd).is_err()
                    {
                        eprintln!("Failed to send AddSample command: ringbuffer full");
                    }
                                        self.loaded_samples.push(sample);
                                        self.note_map_input.push(String::new());
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to load sample: {}", e);
                                    }
                                }
                            }
                        }

                        if ui.button("Save Bank").clicked()
                            && let Some(path) = FileDialog::new()
                                .add_filter("Sample Bank", &["json"])
                                .set_file_name("sample_bank.json")
                                .save_file()
                        {
                            match self.save_sample_bank(&path) {
                                Ok(()) => {
                                    println!("Sample bank saved to: {:?}", path);
                                }
                                Err(e) => {
                                    eprintln!("Failed to save sample bank: {}", e);
                                }
                            }
                        }
                        if ui.button("Load Bank").clicked()
                            && let Some(path) = FileDialog::new()
                                .add_filter("Sample Bank", &["json"])
                                .pick_file()
                        {
                            match self.load_sample_bank(&path) {
                                Ok(()) => {
                                    println!("Sample bank loaded from: {:?}", path);
                                }
                                Err(e) => {
                                    eprintln!("Failed to load sample bank: {}", e);
                                }
                            }
                        }
                    });

                    ui.add_space(10.0);
                    ui.heading("Loaded Samples");

                    // Track actions to perform after rendering UI (to avoid borrow conflicts)
                    let mut preview_action: Option<(usize, bool)> = None; // (index, is_stop)
                    let mut delete_action: Option<usize> = None; // index to delete

                    for (i, sample) in self.loaded_samples.iter_mut().enumerate() {
                        // Extract preview state before ui.horizontal to avoid borrow issues
                        let is_previewing =
                            self.preview_sample_note.map(|(idx, _)| idx == i).unwrap_or(false);

                        ui.horizontal(|ui| {
                            ui.label(&sample.name);

                            // Preview button
                            let preview_button_text = if is_previewing { "⏸ Stop" } else { "▶ Preview" };
                            if ui.button(preview_button_text).clicked() {
                                preview_action = Some((i, is_previewing));
                            }

                            // Delete button
                            if ui.button("🗑️ Delete").clicked() {
                                delete_action = Some(i);
                            }

                            let mut is_looping =
                                sample.loop_mode == crate::sampler::loader::LoopMode::Forward;
                            if ui.checkbox(&mut is_looping, "Loop").changed() {
                                sample.loop_mode = if is_looping {
                                    crate::sampler::loader::LoopMode::Forward
                                } else {
                                    crate::sampler::loader::LoopMode::Off
                                };
                                let sample_arc = Arc::new(sample.clone());
                                let cmd = Command::UpdateSample(i, sample_arc);
                                if let Ok(mut tx) = self.command_tx.lock() && ringbuf::traits::Producer::try_push(&mut *tx, cmd).is_err() {
                                    eprintln!("Failed to send UpdateSample command: ringbuffer full");
                                }
                            }

                            // Reverse checkbox
                            if ui.checkbox(&mut sample.reverse, "Reverse").changed() {
                                let sample_arc = Arc::new(sample.clone());
                                let cmd = Command::UpdateSample(i, sample_arc);
                                if let Ok(mut tx) = self.command_tx.lock() && ringbuf::traits::Producer::try_push(&mut *tx, cmd).is_err() {
                                    eprintln!("Failed to send UpdateSample command: ringbuffer full");
                                }
                            }

                            if is_looping {
                                let data_len = match &sample.data {
                                    crate::sampler::loader::SampleData::F32(data) => data.len(),
                                };

                                // Helper function to convert samples to milliseconds
                                let samples_to_ms = |samples: usize| -> f32 {
                                    (samples as f32 / sample.sample_rate as f32) * 1000.0
                                };

                                ui.label(format!(
                                    "Start: {} samples ({:.1} ms)",
                                    sample.loop_start,
                                    samples_to_ms(sample.loop_start)
                                ));
                                if ui
                                    .add(
                                        egui::Slider::new(&mut sample.loop_start, 0..=sample.loop_end)
                                            .suffix(" samples"),
                                    )
                                    .changed()
                                {
                                    let sample_arc = Arc::new(sample.clone());
                                    let cmd = Command::UpdateSample(i, sample_arc);
                                    if let Ok(mut tx) = self.command_tx.lock() && ringbuf::traits::Producer::try_push(&mut *tx, cmd).is_err() {
                                        eprintln!("Failed to send UpdateSample command: ringbuffer full");
                                    }
                                }
                                ui.label(format!(
                                    "End: {} samples ({:.1} ms)",
                                    sample.loop_end,
                                    samples_to_ms(sample.loop_end)
                                ));
                                if ui
                                    .add(
                                        egui::Slider::new(
                                            &mut sample.loop_end,
                                            sample.loop_start..=data_len,
                                        )
                                        .suffix(" samples"),
                                    )
                                    .changed()
                                {
                                    let sample_arc = Arc::new(sample.clone());
                                    let cmd = Command::UpdateSample(i, sample_arc);
                                    if let Ok(mut tx) = self.command_tx.lock() && ringbuf::traits::Producer::try_push(&mut *tx, cmd).is_err() {
                                        eprintln!("Failed to send UpdateSample command: ringbuffer full");
                                    }
                                }
                            }

                            ui.label("Note:");
                            ui.text_edit_singleline(&mut self.note_map_input[i]);
                            if ui.button("Assign").clicked()
                && let Ok(note) = self.note_map_input[i].parse::<u8>()
            {
                let cmd =
                    Command::SetNoteSampleMapping { note, sample_index: i };
                if let Ok(mut tx) = self.command_tx.lock() && ringbuf::traits::Producer::try_push(&mut *tx, cmd).is_err() {
                    eprintln!(
                    "Failed to send SetNoteSampleMapping command: ringbuffer full"
                );
                }
            }
                        });

                        // Waveform Plot with loop markers
                        let (waveform_line, _data_len) = match &sample.data {
                            crate::sampler::loader::SampleData::F32(data) => {
                                let num_points = data.len().min(1024);
                                let skip_factor = (data.len() / num_points).max(1);
                                let plot_points: PlotPoints = (0..num_points)
                                    .map(|i| {
                                        let idx = (i * skip_factor).min(data.len() - 1);
                                        [idx as f64, data[idx] as f64]
                                    })
                                    .collect::<Vec<[f64; 2]>>()
                                    .into();
                                (Line::new(plot_points), data.len())
                            }
                        };

                        Plot::new(format!("sample_plot_{}", i))
                            .show_background(false)
                            .height(50.0)
                            .show_axes([false, true])
                            .show(ui, |plot_ui| {
                                plot_ui.line(waveform_line);
                                // Add visual markers for loop points when looping is enabled
                                if sample.loop_mode == crate::sampler::loader::LoopMode::Forward {
                                    // Loop start marker (green)
                                    plot_ui.vline(
                                        VLine::new(sample.loop_start as f64)
                                            .color(egui::Color32::from_rgb(0, 200, 0))
                                            .width(2.0)
                                            .name("Loop Start"),
                                    );
                                    // Loop end marker (red)
                                    plot_ui.vline(
                                        VLine::new(sample.loop_end as f64)
                                            .color(egui::Color32::from_rgb(200, 0, 0))
                                            .width(2.0)
                                            .name("Loop End"),
                                    );
                                }
                            });

                        ui.horizontal(|ui| {
                            ui.label("Volume:");
                            if ui
                                .add(egui::Slider::new(&mut sample.volume, 0.0..=1.0))
                                .changed()
                            {
                                let sample_arc = Arc::new(sample.clone());
                                let cmd = Command::UpdateSample(i, sample_arc);
                                if let Ok(mut tx) = self.command_tx.lock() && ringbuf::traits::Producer::try_push(&mut *tx, cmd).is_err() {
                                    eprintln!("Failed to send UpdateSample command: ringbuffer full");
                                }
                            }
                            ui.label("Pan:");
                            if ui
                                .add(egui::Slider::new(&mut sample.pan, -1.0..=1.0))
                                .changed()
                            {
                                let sample_arc = Arc::new(sample.clone());
                                let cmd = Command::UpdateSample(i, sample_arc);
                                if let Ok(mut tx) = self.command_tx.lock() && ringbuf::traits::Producer::try_push(&mut *tx, cmd).is_err() {
                                    eprintln!("Failed to send UpdateSample command: ringbuffer full");
                                }
                            }
                            ui.label("Pitch Offset:");
                            if ui
                                .add(egui::Slider::new(&mut sample.pitch_offset, -12..=12).suffix(" st"))
                                .changed()
                            {
                                let sample_arc = Arc::new(sample.clone());
                                let cmd = Command::UpdateSample(i, sample_arc);
                                if let Ok(mut tx) = self.command_tx.lock() && ringbuf::traits::Producer::try_push(&mut *tx, cmd).is_err() {
                                    eprintln!("Failed to send UpdateSample command: ringbuffer full");
                                }
                            }
                        });
                    }

                    // Handle preview action after the loop to avoid borrow conflicts
                    if let Some((idx, is_stop)) = preview_action {
                        if is_stop {
                            // Stop preview
                            if let Some((_, note)) = self.preview_sample_note {
                                self.send_note_off_direct(note);
                            }
                            self.preview_sample_note = None;
                            self.preview_timer = None;
                        } else {
                            // Start preview
                            self.preview_sample(idx);
                        }
                    }

                    // Handle delete action after the loop to avoid borrow conflicts
                    if let Some(idx) = delete_action {
                        // Stop preview if deleting the currently previewed sample
                        if let Some((preview_idx, note)) = self.preview_sample_note {
                            if preview_idx == idx {
                                self.send_note_off_direct(note);
                                self.preview_sample_note = None;
                                self.preview_timer = None;
                            } else if preview_idx > idx {
                                // Update preview index if it's after the deleted sample
                                self.preview_sample_note = Some((preview_idx - 1, note));
                            }
                        }

                        // Send command to audio thread
                        let cmd = Command::RemoveSample(idx);
                        if let Ok(mut tx) = self.command_tx.lock() && ringbuf::traits::Producer::try_push(&mut *tx, cmd).is_err() {
                            eprintln!("Failed to send RemoveSample command: ringbuffer full");
                        }

                        // Remove from UI
                        self.loaded_samples.remove(idx);
                        self.note_map_input.remove(idx);
                    }
                }
                UiTab::Sequencer => {
                    // Sequencer tab - Timeline, transport controls, and metronome
                    ui.heading("Sequencer");

                    // Transport controls
                    ui.horizontal(|ui| {
                        ui.label("Transport:");

                        let transport_state = self.sequencer.state();
                        let (play_button, _pause_button, stop_button, record_button) = match transport_state {
                            TransportState::Playing => ("⏸ Pause", "⏸ Pause", "⏹ Stop", "⏺ Record"),
                            TransportState::Recording => ("⏸ Pause", "⏸ Pause", "⏹ Stop", "⏺ Recording..."),
                            TransportState::Paused => ("▶ Play", "⏸ Pause", "⏹ Stop", "⏺ Record"),
                            TransportState::Stopped => ("▶ Play", "⏸ Pause", "⏹ Stop", "⏺ Record"),
                        };

                        if ui.button(play_button).clicked() {
                            if transport_state.is_playing() {
                                self.sequencer.pause();
                                // Send transport state to audio thread
                                let cmd = Command::SetTransportPlaying(false);
                                if let Ok(mut tx) = self.command_tx.lock() {
                                    let _ = ringbuf::traits::Producer::try_push(&mut *tx, cmd);
                                }
                            } else {
                                self.sequencer.play();
                                // Send transport state to audio thread
                                let cmd = Command::SetTransportPlaying(true);
                                if let Ok(mut tx) = self.command_tx.lock() {
                                    let _ = ringbuf::traits::Producer::try_push(&mut *tx, cmd);
                                }
                            }
                        }

                        if ui.button(stop_button).clicked() {
                            self.sequencer.stop();
                            // Send transport state to audio thread
                            let cmd = Command::SetTransportPlaying(false);
                            if let Ok(mut tx) = self.command_tx.lock() {
                                let _ = ringbuf::traits::Producer::try_push(&mut *tx, cmd);
                            }
                        }

                        if ui.button(record_button).clicked() {
                            if transport_state.is_recording() {
                                self.sequencer.pause();
                            } else {
                                self.sequencer.record();
                            }
                        }
                    });

                    ui.add_space(10.0);

                    // Position and tempo display
                    ui.horizontal(|ui| {
                        let current_position = self.sequencer.position();
                        ui.label(format!(
                            "Position: {} ({} samples)",
                            current_position.musical,
                            current_position.samples
                        ));
                    });

                    // Tempo and time signature controls
                    ui.horizontal(|ui| {
                        ui.label("Tempo (BPM):");
                        if ui.add(
                            egui::Slider::new(&mut self.sequencer_tempo, 60.0..=200.0)
                                .text("BPM")
                                .fixed_decimals(1)
                        ).changed() {
                            self.sequencer.set_tempo(Tempo::new(self.sequencer_tempo));
                            // Send tempo to audio thread
                            let cmd = Command::SetTempo(self.sequencer_tempo);
                            if let Ok(mut tx) = self.command_tx.lock() {
                                let _ = ringbuf::traits::Producer::try_push(&mut *tx, cmd);
                            }
                        }

                        ui.add_space(20.0);

                        ui.label("Time Signature:");
                        ui.horizontal(|ui| {
                            if ui.add(egui::DragValue::new(&mut self.time_signature_numerator).range(1..=16)).changed() {
                                self.sequencer.set_time_signature(TimeSignature::new(
                                    self.time_signature_numerator,
                                    self.time_signature_denominator
                                ));
                                // Send time signature to audio thread
                                let cmd = Command::SetTimeSignature(
                                    self.time_signature_numerator,
                                    self.time_signature_denominator
                                );
                                if let Ok(mut tx) = self.command_tx.lock() {
                                    let _ = ringbuf::traits::Producer::try_push(&mut *tx, cmd);
                                }
                            };
                            ui.label("/");
                            // Denominator must be power of 2 - restrict to common values
                            let denominator_options = [1, 2, 4, 8, 16];
                            let current_index = denominator_options.iter().position(|&x| x == self.time_signature_denominator).unwrap_or(1);
                            let mut selected_index = current_index;

                            egui::ComboBox::from_id_salt("time_sig_denominator")
                                .selected_text(format!("{}", self.time_signature_denominator))
                                .show_ui(ui, |ui| {
                                    for (i, &denom) in denominator_options.iter().enumerate() {
                                        ui.selectable_value(&mut selected_index, i, format!("{}", denom));
                                    }
                                });

                            if selected_index != current_index {
                                self.time_signature_denominator = denominator_options[selected_index];
                                self.sequencer.set_time_signature(TimeSignature::new(
                                    self.time_signature_numerator,
                                    self.time_signature_denominator
                                ));
                                // Send time signature to audio thread
                                let cmd = Command::SetTimeSignature(
                                    self.time_signature_numerator,
                                    self.time_signature_denominator
                                );
                                if let Ok(mut tx) = self.command_tx.lock() {
                                    let _ = ringbuf::traits::Producer::try_push(&mut *tx, cmd);
                                }
                            }
                        });
                    });

                    ui.add_space(10.0);

                    // Loop controls
                    ui.horizontal(|ui| {
                        ui.label("Loop:");
                        if ui.checkbox(&mut self.loop_enabled, "Enable").changed() {
                            self.sequencer.set_loop_enabled(self.loop_enabled);
                        }

                        if self.loop_enabled {
                            ui.label("From:");
                            if ui.add(egui::DragValue::new(&mut self.loop_start_bars).range(1..=999)).changed() {
                                let start_pos = Position::from_musical(
                                    MusicalTime::new(self.loop_start_bars, 1, 0),
                                    self.sequencer.sample_rate(),
                                    self.sequencer.tempo(),
                                    self.sequencer.time_signature(),
                                );
                                let end_pos = Position::from_musical(
                                    MusicalTime::new(self.loop_end_bars, 1, 0),
                                    self.sequencer.sample_rate(),
                                    self.sequencer.tempo(),
                                    self.sequencer.time_signature(),
                                );
                                self.sequencer.set_loop_region(start_pos, end_pos);
                            }

                            ui.label("To:");
                            if ui.add(egui::DragValue::new(&mut self.loop_end_bars).range(1..=999)).changed() {
                                if self.loop_end_bars <= self.loop_start_bars {
                                    self.loop_end_bars = self.loop_start_bars + 1;
                                }
                                let start_pos = Position::from_musical(
                                    MusicalTime::new(self.loop_start_bars, 1, 0),
                                    self.sequencer.sample_rate(),
                                    self.sequencer.tempo(),
                                    self.sequencer.time_signature(),
                                );
                                let end_pos = Position::from_musical(
                                    MusicalTime::new(self.loop_end_bars, 1, 0),
                                    self.sequencer.sample_rate(),
                                    self.sequencer.tempo(),
                                    self.sequencer.time_signature(),
                                );
                                self.sequencer.set_loop_region(start_pos, end_pos);
                            }
                        }
                    });

                    ui.add_space(10.0);

                    // Metronome controls
                    ui.horizontal(|ui| {
                        ui.label("Metronome:");
                        if ui.checkbox(&mut self.metronome_enabled, "Enable").changed() {
                            // Send metronome enable command to audio thread
                            let cmd = Command::SetMetronomeEnabled(self.metronome_enabled);
                            if let Ok(mut tx) = self.command_tx.lock() {
                                let _ = ringbuf::traits::Producer::try_push(&mut *tx, cmd);
                            }
                        }

                        ui.label("Volume:");
                        if ui.add(egui::Slider::new(&mut self.metronome_volume, 0.0..=1.0)).changed() {
                            // Send metronome volume command to audio thread
                            let cmd = Command::SetMetronomeVolume(self.metronome_volume);
                            if let Ok(mut tx) = self.command_tx.lock() {
                                let _ = ringbuf::traits::Producer::try_push(&mut *tx, cmd);
                            }
                        }
                    });

                    ui.add_space(10.0);

                    // Snap-to-grid controls
                    ui.heading("Timeline & Cursor");
                    ui.horizontal(|ui| {
                        ui.label("Snap to Grid:");
                        if ui.checkbox(&mut self.snap_to_grid_enabled, "Enable").changed() {
                            // Cursor position will be snapped on next update
                        }

                        if self.snap_to_grid_enabled {
                            ui.label("Grid:");
                            let subdivision_options = [(1, "Whole"), (2, "Half"), (4, "Quarter"), (8, "Eighth"), (16, "Sixteenth")];
                            let current_index = subdivision_options.iter().position(|&(div, _)| div == self.grid_subdivision).unwrap_or(2);
                            let mut selected_index = current_index;

                            egui::ComboBox::from_id_salt("grid_subdivision")
                                .selected_text(format!("{} note", subdivision_options[current_index].1))
                                .show_ui(ui, |ui| {
                                    for (i, &(_div, label)) in subdivision_options.iter().enumerate() {
                                        ui.selectable_value(&mut selected_index, i, format!("{} note", label));
                                    }
                                });

                            if selected_index != current_index {
                                self.grid_subdivision = subdivision_options[selected_index].0;
                            }
                        }
                    });

                    ui.add_space(10.0);

                    // Draw timeline with cursor
                    ui.heading("Timeline");
                    self.draw_timeline_with_cursor(ui);

                    ui.add_space(10.0);

                    // Current position display with snap info
                    ui.horizontal(|ui| {
                        let display_position = if self.snap_to_grid_enabled {
                            self.snap_to_grid(self.cursor_position)
                        } else {
                            self.cursor_position
                        };

                        ui.label(format!("Cursor: {}", display_position.musical));
                        ui.label(format!("Samples: {}", display_position.samples));

                        if self.snap_to_grid_enabled {
                            ui.colored_label(egui::Color32::from_rgb(100, 200, 100),
                                format!("📐 Snapped to {} note",
                                    match self.grid_subdivision {
                                        1 => "whole",
                                        2 => "half", 
                                        4 => "quarter",
                                        8 => "eighth",
                                        16 => "sixteenth",
                                        _ => "custom"
                                    }
                                )
                            );
                        }
                    });

                    ui.add_space(10.0);

                    // Piano Roll editor
                    ui.heading("Piano Roll");
                    ui.label(format!("Pattern: {} ({} bars, {} notes)",
                        self.active_pattern.name,
                        self.active_pattern.length_bars,
                        self.active_pattern.note_count()
                    ));

                    ui.add_space(10.0);

                    // Show piano roll (returns true if pattern was modified)
                    let pattern_changed = self.piano_roll_editor.show(
                        ui,
                        &mut self.active_pattern,
                        self.sequencer.tempo(),
                        self.sequencer.time_signature(),
                        self.sequencer.sample_rate(),
                        self.sequencer.shared_state().position_samples(),
                    );

                    // Auto-send pattern to audio thread when modified
                    if pattern_changed {
                        let cmd = Command::SetPattern(self.active_pattern.clone());
                        if let Ok(mut tx) = self.command_tx.lock() {
                            let _ = ringbuf::traits::Producer::try_push(&mut *tx, cmd);
                        }
                    }

                    ui.add_space(10.0);

                    // Information display
                    ui.label("The sequencer provides timeline-based playback control.");
                    ui.label("Use transport controls to play, pause, stop, and record.");
                    ui.label("Piano Roll: Click to add notes, use tools to edit.");
                    ui.label("Métronome helps maintain timing during playback.");
                    ui.label("Click on the timeline to set cursor position (snaps to grid if enabled).");
                }
                UiTab::Synth => {
                    // Synth tab
                    ui.heading("Synth");

                    // Volume control (using undoable commands)
                    ui.horizontal(|ui| {
                        ui.label("Volume:");
                        if ui.add(egui::Slider::new(&mut self.volume_ui, 0.0..=1.0)).changed() {
                            let cmd = Box::new(SetVolumeCommand::new(self.volume_ui));
                            if let Err(e) = self.command_manager.execute(cmd, &mut self.daw_state) {
                                eprintln!("Failed to execute volume command: {}", e);
                            }
                            self.volume_atomic.set(self.volume_ui);
                        }
                    });

                    // Waveform selection
                    ui.horizontal(|ui| {
                        ui.label("Waveform:");
                        let previous_waveform = self.selected_waveform;
                        egui::ComboBox::from_id_salt("waveform_selector")
                            .selected_text(match self.selected_waveform {
                                WaveformType::Sine => "Sine",
                                WaveformType::Square => "Square",
                                WaveformType::Saw => "Saw",
                                WaveformType::Triangle => "Triangle",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.selected_waveform, WaveformType::Sine, "Sine");
                                ui.selectable_value(
                                    &mut self.selected_waveform,
                                    WaveformType::Square,
                                    "Square",
                                );
                                ui.selectable_value(&mut self.selected_waveform, WaveformType::Saw, "Saw");
                                ui.selectable_value(
                                    &mut self.selected_waveform,
                                    WaveformType::Triangle,
                                    "Triangle",
                                );
                            });

                        if previous_waveform != self.selected_waveform {
                            let cmd = Box::new(SetWaveformCommand::new(self.selected_waveform));
                            if let Err(e) = self.command_manager.execute(cmd, &mut self.daw_state) {
                                eprintln!("Failed to execute waveform command: {}", e);
                            }
                        }
                    });

                    ui.add_space(10.0);
                    ui.separator();

                    // ADSR Envelope Section
                    ui.heading("ADSR Envelope");

                    ui.horizontal(|ui| {
                        ui.label("Attack:");
                        if ui
                            .add(
                                egui::Slider::new(&mut self.adsr_attack, 0.001..=2.0)
                                    .text("s")
                                    .logarithmic(true),
                            )
                            .changed()
                        {
                            let params = AdsrParams::new(
                                self.adsr_attack,
                                self.adsr_decay,
                                self.adsr_sustain,
                                self.adsr_release,
                            );
                            let cmd = Box::new(SetAdsrCommand::new(params));
                            if let Err(e) = self.command_manager.execute(cmd, &mut self.daw_state) {
                                eprintln!("Failed to execute ADSR command: {}", e);
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Decay:");
                        if ui
                            .add(
                                egui::Slider::new(&mut self.adsr_decay, 0.001..=2.0)
                                    .text("s")
                                    .logarithmic(true),
                            )
                            .changed()
                        {
                            let params = AdsrParams::new(
                                self.adsr_attack,
                                self.adsr_decay,
                                self.adsr_sustain,
                                self.adsr_release,
                            );
                            let cmd = Box::new(SetAdsrCommand::new(params));
                            if let Err(e) = self.command_manager.execute(cmd, &mut self.daw_state) {
                                eprintln!("Failed to execute ADSR command: {}", e);
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Sustain:");
                        if ui
                            .add(egui::Slider::new(&mut self.adsr_sustain, 0.0..=1.0))
                            .changed()
                        {
                            let params = AdsrParams::new(
                                self.adsr_attack,
                                self.adsr_decay,
                                self.adsr_sustain,
                                self.adsr_release,
                            );
                            let cmd = Box::new(SetAdsrCommand::new(params));
                            if let Err(e) = self.command_manager.execute(cmd, &mut self.daw_state) {
                                eprintln!("Failed to execute ADSR command: {}", e);
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Release:");
                        if ui
                            .add(
                                egui::Slider::new(&mut self.adsr_release, 0.001..=5.0)
                                    .text("s")
                                    .logarithmic(true),
                            )
                            .changed()
                        {
                            let params = AdsrParams::new(
                                self.adsr_attack,
                                self.adsr_decay,
                                self.adsr_sustain,
                                self.adsr_release,
                            );
                            let cmd = Box::new(SetAdsrCommand::new(params));
                            if let Err(e) = self.command_manager.execute(cmd, &mut self.daw_state) {
                                eprintln!("Failed to execute ADSR command: {}", e);
                            }
                        }
                    });

                    ui.add_space(10.0);
                    ui.separator();

                    // Polyphony Mode Section
                    ui.heading("Polyphony Mode");

                    ui.horizontal(|ui| {
                        ui.label("Mode:");
                        let previous_mode = self.poly_mode;
                        egui::ComboBox::from_id_salt("poly_mode_selector")
                            .selected_text(match self.poly_mode {
                                PolyMode::Poly => "Poly (Multiple notes)",
                                PolyMode::Mono => "Mono (One note, retriggered)",
                                PolyMode::Legato => "Legato (Smooth pitch slide)",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.poly_mode,
                                    PolyMode::Poly,
                                    "Poly (Multiple notes)",
                                );
                                ui.selectable_value(
                                    &mut self.poly_mode,
                                    PolyMode::Mono,
                                    "Mono (One note, retriggered)",
                                );
                                ui.selectable_value(
                                    &mut self.poly_mode,
                                    PolyMode::Legato,
                                    "Legato (Smooth pitch slide)",
                                );
                            });

                        if previous_mode != self.poly_mode {
                            let cmd = Box::new(SetPolyModeCommand::new(self.poly_mode));
                            if let Err(e) = self.command_manager.execute(cmd, &mut self.daw_state) {
                                eprintln!("Failed to execute PolyMode command: {}", e);
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Glide Time:");
                        if ui
                            .add(
                                egui::Slider::new(&mut self.portamento_time, 0.0..=2.0)
                                    .text("s")
                                    .logarithmic(false),
                            )
                            .changed()
                        {
                            let params = PortamentoParams::new(self.portamento_time);
                            let cmd = Box::new(SetPortamentoCommand::new(params));
                            if let Err(e) = self.command_manager.execute(cmd, &mut self.daw_state) {
                                eprintln!("Failed to execute Portamento command: {}", e);
                            }
                        }
                    });
                    ui.label("Set to 0 for instant pitch changes, >0 for smooth glides.");
                    ui.label("Works best in Mono/Legato modes.");

                    ui.add_space(10.0);
                    ui.separator();

                    // Voice Mode Section
                    ui.heading("Voice Mode");
                    let current_mode = self.daw_state.voice_mode;
                    let new_mode = match current_mode {
                        VoiceMode::Synth => {
                            if ui.button("Switch to Sampler").clicked() {
                                Some(VoiceMode::Sampler)
                            } else {
                                None
                            }
                        }
                        VoiceMode::Sampler => {
                            if ui.button("Switch to Synth").clicked() {
                                Some(VoiceMode::Synth)
                            } else {
                                None
                            }
                        }
                    };

                    if let Some(mode) = new_mode {
                        let cmd = Box::new(SetVoiceModeCommand::new(mode));
                        if let Err(e) = self.command_manager.execute(cmd, &mut self.daw_state) {
                            eprintln!("Failed to execute voice mode command: {}", e);
                        }
                    }

                    ui.add_space(10.0);
                    ui.separator();

                    // Filter Section
                    ui.heading("Filter");
                    let mut filter_params = self.daw_state.filter;

                    // Filter enable/disable
                    if ui.checkbox(&mut filter_params.enabled, "Enable").changed() {
                        let cmd = Box::new(SetFilterCommand::new(filter_params));
                        let _ = self.command_manager.execute(cmd, &mut self.daw_state);
                    }

                    // Filter type
                    ui.horizontal(|ui| {
                        ui.label("Type:");
                        let filter_type_changed = egui::ComboBox::from_id_salt("filter_type")
                            .selected_text(format!("{:?}", filter_params.filter_type))
                            .show_ui(ui, |ui| {
                                let mut changed = false;
                                changed |= ui
                                    .selectable_value(
                                        &mut filter_params.filter_type,
                                        FilterType::LowPass,
                                        "LowPass",
                                    )
                                    .changed();
                                changed |= ui
                                    .selectable_value(
                                        &mut filter_params.filter_type,
                                        FilterType::HighPass,
                                        "HighPass",
                                    )
                                    .changed();
                                changed |= ui
                                    .selectable_value(
                                        &mut filter_params.filter_type,
                                        FilterType::BandPass,
                                        "BandPass",
                                    )
                                    .changed();
                                changed |= ui
                                    .selectable_value(
                                        &mut filter_params.filter_type,
                                        FilterType::Notch,
                                        "Notch",
                                    )
                                    .changed();
                                changed
                            })
                            .inner
                            .unwrap_or(false);

                        if filter_type_changed {
                            let cmd = Box::new(SetFilterCommand::new(filter_params));
                            let _ = self.command_manager.execute(cmd, &mut self.daw_state);
                        }
                    });

                    // Cutoff frequency
                    ui.horizontal(|ui| {
                        ui.label("Cutoff:");
                        if ui
                            .add(
                                egui::Slider::new(&mut filter_params.cutoff, 20.0..=10000.0)
                                    .text("Hz")
                                    .logarithmic(true),
                            )
                            .changed()
                        {
                            let cmd = Box::new(SetFilterCommand::new(filter_params));
                            let _ = self.command_manager.execute(cmd, &mut self.daw_state);
                        }
                    });

                    // Resonance (Q factor)
                    ui.horizontal(|ui| {
                        ui.label("Resonance (Q):");
                        if ui
                            .add(egui::Slider::new(&mut filter_params.resonance, 0.5..=20.0).logarithmic(true))
                            .changed()
                        {
                            let cmd = Box::new(SetFilterCommand::new(filter_params));
                            let _ = self.command_manager.execute(cmd, &mut self.daw_state);
                        }
                    });

                    ui.label("Cutoff can be modulated via the Modulation Matrix (Envelope → FilterCutoff).");
                }
                UiTab::Play => {
                    // Play tab: virtual keyboard
                    self.draw_keyboard_ui(ui);
                    ui.add_space(10.0);
                    ui.label("Info : Play with your computer keyboard or an external MIDI Keyboard");
                }
                UiTab::Performance => {
                    // Performance tab: CPU + notifications
                    ui.heading("Performance");
                    ui.horizontal(|ui| {
                        let cpu_percentage = self.cpu_monitor.get_cpu_percentage();
                        let load_level = self.cpu_monitor.get_load_level();
                        ui.label("CPU:");
                        let (cpu_color, status_text) = match load_level {
                            crate::audio::cpu_monitor::CpuLoad::Low => (egui::Color32::GREEN, "●"),
                            crate::audio::cpu_monitor::CpuLoad::Medium => {
                                (egui::Color32::from_rgb(255, 165, 0), "●")
                            }
                            crate::audio::cpu_monitor::CpuLoad::High => (egui::Color32::RED, "●"),
                        };
                        ui.colored_label(cpu_color, status_text);
                        ui.label(format!("{:.1}%", cpu_percentage));
                        if matches!(load_level, crate::audio::cpu_monitor::CpuLoad::High) {
                            ui.colored_label(egui::Color32::RED, "⚠ High CPU load!");
                        }
        });
    }


}

            // Status bar at the bottom
            ui.add_space(10.0);
            self.draw_status_bar(ui);
        });
    }
}
