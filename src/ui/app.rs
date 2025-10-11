// Main UI App UI

use crate::audio::cpu_monitor::{CpuLoad, CpuMonitor};
use crate::audio::device::{AudioDeviceInfo, AudioDeviceManager};
use crate::audio::parameters::AtomicF32;
use crate::connection::status::DeviceStatus;
use crate::messaging::channels::{CommandProducer, NotificationConsumer};
use crate::messaging::command::Command;
use crate::messaging::notification::{Notification, NotificationCategory};
use crate::midi::device::{MidiDeviceInfo, MidiDeviceManager};
use crate::midi::event::MidiEvent;
use crate::midi::manager::MidiConnectionManager;
use crate::synth::oscillator::WaveformType;
use eframe::egui;
use std::collections::{HashSet, VecDeque};

pub struct DawApp {
    command_tx: CommandProducer,
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
    // CPU monitoring
    cpu_monitor: CpuMonitor,
    last_cpu_load: CpuLoad,
    // Notification system
    notification_rx: NotificationConsumer,
    notification_queue: VecDeque<Notification>,
    max_notifications: usize,
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

        Self {
            command_tx,
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
            cpu_monitor,
            last_cpu_load: CpuLoad::Low,
            notification_rx,
            notification_queue: VecDeque::new(),
            max_notifications: 10,
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
    fn get_latest_notification(&self) -> Option<&Notification> {
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
            let cmd = Command::Midi(MidiEvent::NoteOn {
                note,
                velocity: 100,
            });
            let _ = ringbuf::traits::Producer::try_push(&mut self.command_tx, cmd);
        }
    }

    fn send_note_off(&mut self, note: u8) {
        if self.active_notes.remove(&note) {
            let cmd = Command::Midi(MidiEvent::NoteOff { note });
            let _ = ringbuf::traits::Producer::try_push(&mut self.command_tx, cmd);
        }
    }

    fn draw_keyboard(&mut self, ui: &mut egui::Ui) {
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

        // Handle pressed and release keys
        let ctx = ui.ctx();

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

        // Display the visual keyboard
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

        // Update notifications from ringbuffer
        self.update_notifications();

        // Check CPU load and notify if high
        self.check_cpu_load();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("MyMusic DAW - MVP");
            ui.separator();

            ui.add_space(10.0);

            // === Device Selection Section ===
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

            ui.add_space(10.0);
            ui.separator();

            // Volume control (connected to atomic parameter)
            ui.horizontal(|ui| {
                ui.label("Volume:");
                if ui.add(egui::Slider::new(&mut self.volume_ui, 0.0..=1.0)).changed() {
                    // Update atomic volume when slider changes
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

                // Send command if waveform changed
                if previous_waveform != self.selected_waveform {
                    let cmd = Command::SetWaveform(self.selected_waveform);
                    let _ = ringbuf::traits::Producer::try_push(&mut self.command_tx, cmd);
                }
            });

            ui.add_space(10.0);
            ui.separator();

            // CPU Monitor
            ui.horizontal(|ui| {
                let cpu_percentage = self.cpu_monitor.get_cpu_percentage();
                let load_level = self.cpu_monitor.get_load_level();

                ui.label("CPU:");

                // Color based on load level
                let (cpu_color, status_text) = match load_level {
                    crate::audio::cpu_monitor::CpuLoad::Low => (egui::Color32::GREEN, "●"),
                    crate::audio::cpu_monitor::CpuLoad::Medium => (egui::Color32::from_rgb(255, 165, 0), "●"), // Orange
                    crate::audio::cpu_monitor::CpuLoad::High => (egui::Color32::RED, "●"),
                };

                ui.colored_label(cpu_color, status_text);
                ui.label(format!("{:.1}%", cpu_percentage));

                // Show warning if CPU is high
                if matches!(load_level, crate::audio::cpu_monitor::CpuLoad::High) {
                    ui.colored_label(egui::Color32::RED, "⚠ High CPU load!");
                }
            });

            ui.add_space(10.0);

            // Virtual Keyboard
            self.draw_keyboard(ui);

            ui.add_space(20.0);
            ui.separator();
            ui.label("Info : Play with your computer keyboard or an external MIDI Keyboard");

            // Status bar at the bottom
            ui.add_space(10.0);
            self.draw_status_bar(ui);
        });
    }
}
