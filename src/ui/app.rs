// Main UI App UI

use crate::audio::cpu_monitor::{CpuLoad, CpuMonitor};
use crate::audio::device::{AudioDeviceInfo, AudioDeviceManager};
use crate::audio::parameters::AtomicF32;
use crate::command::{CommandManager, DawState};
use crate::command::commands::{SetVolumeCommand, SetWaveformCommand, SetAdsrCommand, SetLfoCommand, SetPolyModeCommand, SetPortamentoCommand, SetModRoutingCommand, SetFilterCommand, SetVoiceModeCommand};
use crate::synth::voice_manager::VoiceMode;
use crate::sampler::loader::{load_sample, Sample};
use crate::synth::filter::FilterType;
use crate::synth::envelope::AdsrParams;
use crate::synth::lfo::{LfoParams, LfoDestination};
use crate::synth::poly_mode::PolyMode;
use crate::synth::portamento::PortamentoParams;
use crate::connection::status::DeviceStatus;
use crate::messaging::channels::{CommandProducer, NotificationConsumer};
use crate::messaging::command::Command;
use crate::messaging::notification::{Notification, NotificationCategory};
use crate::midi::device::{MidiDeviceInfo, MidiDeviceManager};
use crate::midi::event::{MidiEvent, MidiEventTimed};
use crate::midi::manager::MidiConnectionManager;
use crate::synth::oscillator::WaveformType;
use crate::synth::modulation::{ModSource, ModDestination, ModRouting};
use rfd::FileDialog;
use eframe::egui;
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UiTab {
    Devices,
    Synth,
    Modulation,
    Sampler,
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
        let selected_midi_device = midi_connection_manager
            .target_device()
            .unwrap_or_else(|| {
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
                ModRouting { source: ModSource::Lfo(0), destination: ModDestination::OscillatorPitch(0), amount: 2.0, enabled: false },
                ModRouting { source: ModSource::Lfo(0), destination: ModDestination::Amplitude, amount: 0.5, enabled: false },
                ModRouting { source: ModSource::Velocity, destination: ModDestination::Amplitude, amount: 0.5, enabled: false },
                ModRouting { source: ModSource::Aftertouch, destination: ModDestination::Amplitude, amount: 0.5, enabled: false },
            ],
            loaded_samples: Vec::new(),
            note_map_input: Vec::new(),
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
        while let Some(notification) = ringbuf::traits::Consumer::try_pop(&mut self.notification_rx) {
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
            let key_code = egui::Key::from_name(&key.to_string().to_uppercase()).unwrap_or(egui::Key::A);
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
}

impl eframe::App for DawApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Ask for a refresh to capture keyboard events
        ctx.request_repaint();

        // Always process PC keyboard input, regardless of the current tab
        self.process_pc_keyboard_input(ctx);

            // Handle Undo/Redo keyboard shortcuts
            ctx.input(|i| {
                // Ctrl+Z for Undo
                if i.modifiers.command && i.key_pressed(egui::Key::Z) && !i.modifiers.shift {
                    if self.command_manager.can_undo() {
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
                }

            // Ctrl+Shift+Z or Ctrl+Y for Redo
                if (i.modifiers.command && i.key_pressed(egui::Key::Z) && i.modifiers.shift)
                    || (i.modifiers.command && i.key_pressed(egui::Key::Y))
                {
                    if self.command_manager.can_redo() {
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
                                ui.selectable_value(&mut self.selected_midi_device, device.name.clone(), label);
                            }
                        }
                    });

                // Si le device a changé, déclencher la reconnexion
                if previous_device != self.selected_midi_device {
                    self.midi_connection_manager.set_target_device(self.selected_midi_device.clone());
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
                                ui.selectable_value(&mut self.selected_audio_device, device.name.clone(), label);
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
                        let cmd = Box::new(SetModRoutingCommand::new_with_old(i as u8, *routing, old));
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
                            ui.selectable_value(&mut routing.source, ModSource::Lfo(0), src_labels[0]);
                            ui.selectable_value(&mut routing.source, ModSource::Velocity, src_labels[1]);
                            ui.selectable_value(&mut routing.source, ModSource::Aftertouch, src_labels[2]);
                            ui.selectable_value(&mut routing.source, ModSource::Envelope, src_labels[3]);
                        });
                    if routing.source != prev_source {
                        let old = ModRouting { source: prev_source, ..*routing };
                        let cmd = Box::new(SetModRoutingCommand::new_with_old(i as u8, *routing, old));
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
                            ui.selectable_value(&mut routing.destination, ModDestination::OscillatorPitch(0), dst_labels[0]);
                            ui.selectable_value(&mut routing.destination, ModDestination::Amplitude, dst_labels[1]);
                            ui.selectable_value(&mut routing.destination, ModDestination::Pan, dst_labels[2]);
                        });
                    if routing.destination != prev_dest {
                        let old = ModRouting { destination: prev_dest, ..*routing };
                        let cmd = Box::new(SetModRoutingCommand::new_with_old(i as u8, *routing, old));
                        let _ = self.command_manager.execute(cmd, &mut self.daw_state);
                    }

                    // Amount slider
                    let prev_amount = routing.amount;
                    let range = match routing.destination {
                        ModDestination::OscillatorPitch(_) => -12.0..=12.0, // semitones
                        ModDestination::Amplitude => -1.0..=1.0,            // multiplier delta
                        ModDestination::Pan => -1.0..=1.0,                  // pan L/R
                        ModDestination::FilterCutoff => 0.0..=10.0,         // cutoff multiplier (0.1x to 10x)
                    };
                    if ui.add(egui::Slider::new(&mut routing.amount, range).fixed_decimals(2)).changed() {
                        let old = *routing;
                        // Clamp for safety
                        routing.amount = match routing.destination {
                            ModDestination::OscillatorPitch(_) => routing.amount.clamp(-24.0, 24.0),
                            _ => routing.amount.clamp(-1.0, 1.0), // For Amplitude and Pan
                        };
                        let cmd = Box::new(SetModRoutingCommand::new_with_old(i as u8, *routing, old));
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
                                ui.selectable_value(&mut self.lfo_waveform, WaveformType::Square, "Square");
                                ui.selectable_value(&mut self.lfo_waveform, WaveformType::Saw, "Saw");
                                ui.selectable_value(&mut self.lfo_waveform, WaveformType::Triangle, "Triangle");
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
                        if ui.add(egui::Slider::new(&mut self.lfo_rate, 0.1..=20.0).text("Hz").logarithmic(true)).changed() {
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
                        if ui.add(egui::Slider::new(&mut self.lfo_depth, 0.0..=1.0)).changed() {
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
                                ui.selectable_value(&mut self.lfo_destination, LfoDestination::None, "None");
                                ui.selectable_value(&mut self.lfo_destination, LfoDestination::Pitch, "Pitch (Vibrato)");
                                ui.selectable_value(&mut self.lfo_destination, LfoDestination::Volume, "Volume (Tremolo)");
                                ui.selectable_value(&mut self.lfo_destination, LfoDestination::FilterCutoff, "Filter Cutoff (Phase 3a)");
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
                },
                UiTab::Sampler => {
                    ui.heading("Sampler");
                    if ui.button("Load Sample").clicked() {
                        let file = FileDialog::new()
                            .add_filter("Audio Files", &["wav", "flac"])
                            .pick_file();

                        if let Some(path) = file {
                            match load_sample(&path) {
                                Ok(sample) => {
                                    // Clone the sample: one for the UI, one for the audio thread
                                    let sample_for_audio = Arc::new(sample.clone());
                                    let cmd = Command::AddSample(sample_for_audio);
                                    if let Ok(mut tx) = self.command_tx.lock() {
                                        if ringbuf::traits::Producer::try_push(&mut *tx, cmd).is_err() {
                                            eprintln!("Failed to send AddSample command: ringbuffer full");
                                        }
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

                    ui.add_space(10.0);
                    ui.heading("Loaded Samples");
                    for (i, sample) in self.loaded_samples.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(&sample.name);
                            let mut is_looping = sample.loop_mode == crate::sampler::loader::LoopMode::Forward;
                            if ui.checkbox(&mut is_looping, "Loop").changed() {
                                sample.loop_mode = if is_looping {
                                    crate::sampler::loader::LoopMode::Forward
                                } else {
                                    crate::sampler::loader::LoopMode::Off
                                };
                                let sample_arc = Arc::new(sample.clone());
                                let cmd = Command::UpdateSample(i, sample_arc);
                                if let Ok(mut tx) = self.command_tx.lock() {
                                    if ringbuf::traits::Producer::try_push(&mut *tx, cmd).is_err() {
                                        eprintln!("Failed to send UpdateSample command: ringbuffer full");
                                    }
                                }
                            }

                            if is_looping {
                                ui.label("Start:");
                                if ui.add(egui::DragValue::new(&mut sample.loop_start)).changed() {
                                    let sample_arc = Arc::new(sample.clone());
                                    let cmd = Command::UpdateSample(i, sample_arc);
                                    if let Ok(mut tx) = self.command_tx.lock() {
                                        if ringbuf::traits::Producer::try_push(&mut *tx, cmd).is_err() {
                                            eprintln!("Failed to send UpdateSample command: ringbuffer full");
                                        }
                                    }
                                }
                                ui.label("End:");
                                if ui.add(egui::DragValue::new(&mut sample.loop_end)).changed() {
                                    let sample_arc = Arc::new(sample.clone());
                                    let cmd = Command::UpdateSample(i, sample_arc);
                                    if let Ok(mut tx) = self.command_tx.lock() {
                                        if ringbuf::traits::Producer::try_push(&mut *tx, cmd).is_err() {
                                            eprintln!("Failed to send UpdateSample command: ringbuffer full");
                                        }
                                    }
                                }
                            }

                            ui.label("Note:");
                            ui.text_edit_singleline(&mut self.note_map_input[i]);
                            if ui.button("Assign").clicked() {
                                if let Ok(note) = self.note_map_input[i].parse::<u8>() {
                                    let cmd = Command::SetNoteSampleMapping { note, sample_index: i };
                                    if let Ok(mut tx) = self.command_tx.lock() {
                                        if ringbuf::traits::Producer::try_push(&mut *tx, cmd).is_err() {
                                            eprintln!("Failed to send SetNoteSampleMapping command: ringbuffer full");
                                        }
                                    }
                                }
                            }
                        });
                    }
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
                                ui.selectable_value(&mut self.selected_waveform, WaveformType::Square, "Square");
                                ui.selectable_value(&mut self.selected_waveform, WaveformType::Saw, "Saw");
                                ui.selectable_value(&mut self.selected_waveform, WaveformType::Triangle, "Triangle");
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
                if ui.add(egui::Slider::new(&mut self.adsr_attack, 0.001..=2.0)
                    .text("s")
                    .logarithmic(true))
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
                if ui.add(egui::Slider::new(&mut self.adsr_decay, 0.001..=2.0)
                    .text("s")
                    .logarithmic(true))
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
                if ui.add(egui::Slider::new(&mut self.adsr_sustain, 0.0..=1.0))
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
                if ui.add(egui::Slider::new(&mut self.adsr_release, 0.001..=5.0)
                    .text("s")
                    .logarithmic(true))
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
                        ui.selectable_value(&mut self.poly_mode, PolyMode::Poly, "Poly (Multiple notes)");
                        ui.selectable_value(&mut self.poly_mode, PolyMode::Mono, "Mono (One note, retriggered)");
                        ui.selectable_value(&mut self.poly_mode, PolyMode::Legato, "Legato (Smooth pitch slide)");
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
                if ui.add(egui::Slider::new(&mut self.portamento_time, 0.0..=2.0)
                    .text("s")
                    .logarithmic(false))
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
                                changed |= ui.selectable_value(&mut filter_params.filter_type, FilterType::LowPass, "LowPass").changed();
                                changed |= ui.selectable_value(&mut filter_params.filter_type, FilterType::HighPass, "HighPass").changed();
                                changed |= ui.selectable_value(&mut filter_params.filter_type, FilterType::BandPass, "BandPass").changed();
                                changed |= ui.selectable_value(&mut filter_params.filter_type, FilterType::Notch, "Notch").changed();
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
                        if ui.add(egui::Slider::new(&mut filter_params.cutoff, 20.0..=10000.0)
                            .text("Hz")
                            .logarithmic(true))
                            .changed()
                        {
                            let cmd = Box::new(SetFilterCommand::new(filter_params));
                            let _ = self.command_manager.execute(cmd, &mut self.daw_state);
                        }
                    });

                    // Resonance (Q factor)
                    ui.horizontal(|ui| {
                        ui.label("Resonance (Q):");
                        if ui.add(egui::Slider::new(&mut filter_params.resonance, 0.5..=20.0)
                            .logarithmic(true))
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
                            crate::audio::cpu_monitor::CpuLoad::Medium => (egui::Color32::from_rgb(255, 165, 0), "●"),
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
