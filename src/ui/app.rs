// Main UI App UI

use crate::messaging::channels::CommandProducer;
use crate::messaging::command::Command;
use crate::midi::event::MidiEvent;
use eframe::egui;
use std::collections::HashSet;

pub struct DawApp {
    command_tx: CommandProducer,
    volume: f32,
    active_notes: HashSet<u8>,
}

impl DawApp {
    pub fn new(command_tx: CommandProducer) -> Self {
        Self {
            command_tx,
            volume: 0.5,
            active_notes: HashSet::new(),
        }
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
}

impl eframe::App for DawApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Ask for a refresh to capture keyboard events
        ctx.request_repaint();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("MyMusic DAW - MVP");
            ui.separator();

            ui.add_space(10.0);

            // Volume control
            ui.horizontal(|ui| {
                ui.label("Volume:");
                ui.add(egui::Slider::new(&mut self.volume, 0.0..=1.0));
            });

            ui.add_space(20.0);

            // Virtual Keyboard
            self.draw_keyboard(ui);

            ui.add_space(20.0);
            ui.separator();
            ui.label("Info : Play with your computer keyboard or an external MIDI Keyboard");
        });
    }
}
