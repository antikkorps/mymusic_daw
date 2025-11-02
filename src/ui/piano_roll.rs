// Piano Roll UI - MIDI note editor
// Phase 4: Sequencer - MVP implementation

use crate::sequencer::{Note, NoteId, Pattern, Position, Tempo, TimeSignature, generate_note_id};
use eframe::egui;
use egui::{Color32, Pos2, Rect, Response, Sense, Ui, Vec2};
use std::collections::HashSet;

/// Tool mode for piano roll interaction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PianoRollTool {
    /// Draw notes (click + drag)
    Draw,
    /// Select and move notes
    Select,
    /// Erase notes
    Erase,
}

/// Piano roll editor state
pub struct PianoRollEditor {
    /// Current tool mode
    tool: PianoRollTool,

    /// Selected note IDs
    selected_notes: HashSet<NoteId>,

    /// Vertical zoom (pixels per MIDI note)
    pixels_per_note: f32,

    /// Horizontal zoom (pixels per beat)
    pixels_per_beat: f32,

    /// Visible MIDI note range (bottom to top)
    visible_note_start: u8, // Lowest visible note
    visible_note_count: u8, // Number of notes visible

    /// Interaction state
    is_dragging: bool,
    drag_start_pos: Option<Pos2>,
    drag_note_id: Option<NoteId>,

    /// Snap to grid
    snap_enabled: bool,
    snap_subdivision: u16, // 1, 2, 4, 8, 16 (whole, half, quarter, eighth, sixteenth)
}

impl Default for PianoRollEditor {
    fn default() -> Self {
        Self {
            tool: PianoRollTool::Draw,
            selected_notes: HashSet::new(),
            pixels_per_note: 16.0,
            pixels_per_beat: 64.0,
            visible_note_start: 36, // C2
            visible_note_count: 48, // 4 octaves
            is_dragging: false,
            drag_start_pos: None,
            drag_note_id: None,
            snap_enabled: true,
            snap_subdivision: 4, // Quarter notes by default
        }
    }
}

impl PianoRollEditor {
    /// Show the piano roll UI
    ///
    /// Returns true if the pattern was modified (and needs to be sent to audio thread)
    pub fn show(
        &mut self,
        ui: &mut Ui,
        pattern: &mut Pattern,
        tempo: &Tempo,
        time_signature: &TimeSignature,
        sample_rate: f64,
        current_position_samples: u64,
    ) -> bool {
        let mut pattern_changed = false;
        // Toolbar
        self.show_toolbar(ui);

        ui.separator();

        // Main piano roll area (scrollable)
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Calculate dimensions
                let total_height = self.visible_note_count as f32 * self.pixels_per_note;
                let total_width = pattern.length_bars as f32
                    * time_signature.beats_per_bar() as f32
                    * self.pixels_per_beat;

                // Reserve space for drawing
                let (response, painter) = ui.allocate_painter(
                    Vec2::new(total_width, total_height),
                    Sense::click_and_drag(),
                );

                let rect = response.rect;

                // Draw background grid
                self.draw_grid(
                    &painter,
                    rect,
                    pattern.length_bars,
                    time_signature,
                    tempo,
                    sample_rate,
                );

                // Draw piano keyboard on the left
                self.draw_piano_keyboard(&painter, rect);

                // Draw notes
                self.draw_notes(&painter, rect, pattern, tempo, time_signature, sample_rate);

                // Draw playback cursor
                self.draw_playback_cursor(
                    &painter,
                    rect,
                    pattern,
                    tempo,
                    time_signature,
                    sample_rate,
                    current_position_samples,
                );

                // Handle interactions
                let changed = self.handle_interactions(
                    &response,
                    rect,
                    pattern,
                    tempo,
                    time_signature,
                    sample_rate,
                    ui,
                );

                if changed {
                    pattern_changed = true;
                }
            });

        pattern_changed
    }

    /// Show toolbar with tool selection and controls
    fn show_toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Tool:");

            ui.selectable_value(&mut self.tool, PianoRollTool::Draw, "✏ Draw");
            ui.selectable_value(&mut self.tool, PianoRollTool::Select, "↖ Select");
            ui.selectable_value(&mut self.tool, PianoRollTool::Erase, "⌫ Erase");

            ui.separator();

            ui.label("Snap:");
            ui.checkbox(&mut self.snap_enabled, "");

            if self.snap_enabled {
                egui::ComboBox::from_id_salt("snap_subdivision")
                    .selected_text(format!("1/{}", self.snap_subdivision))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.snap_subdivision, 1, "1/1 (Whole)");
                        ui.selectable_value(&mut self.snap_subdivision, 2, "1/2 (Half)");
                        ui.selectable_value(&mut self.snap_subdivision, 4, "1/4 (Quarter)");
                        ui.selectable_value(&mut self.snap_subdivision, 8, "1/8 (Eighth)");
                        ui.selectable_value(&mut self.snap_subdivision, 16, "1/16 (Sixteenth)");
                    });
            }

            ui.separator();

            // Zoom controls
            ui.label("Zoom:");
            if ui.button("-").clicked() {
                self.pixels_per_beat = (self.pixels_per_beat * 0.8).max(32.0);
            }
            if ui.button("+").clicked() {
                self.pixels_per_beat = (self.pixels_per_beat * 1.2).min(256.0);
            }
        });
    }

    /// Draw the background grid (bars, beats, subdivisions)
    fn draw_grid(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        length_bars: u32,
        time_signature: &TimeSignature,
        _tempo: &Tempo,
        _sample_rate: f64,
    ) {
        // Background
        painter.rect_filled(rect, 0.0, Color32::from_gray(30));

        let beats_per_bar = time_signature.numerator as f32;

        // Draw vertical lines for beats and bars
        for bar in 0..length_bars {
            for beat in 0..time_signature.numerator {
                let beat_index = bar as f32 * beats_per_bar + beat as f32;
                let x = rect.left() + beat_index * self.pixels_per_beat;

                // Bar lines (thick)
                if beat == 0 {
                    painter.line_segment(
                        [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                        (2.0, Color32::from_gray(80)),
                    );
                } else {
                    // Beat lines (thin)
                    painter.line_segment(
                        [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                        (1.0, Color32::from_gray(50)),
                    );
                }
            }
        }

        // Draw horizontal lines for MIDI notes
        for note_offset in 0..=self.visible_note_count {
            let note = self.visible_note_start + note_offset;
            let y = rect.bottom() - note_offset as f32 * self.pixels_per_note;

            // Highlight C notes
            let is_c_note = note.is_multiple_of(12);
            let color = if is_c_note {
                Color32::from_gray(70)
            } else {
                Color32::from_gray(40)
            };

            painter.line_segment(
                [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                (1.0, color),
            );
        }
    }

    /// Draw piano keyboard on the left side
    fn draw_piano_keyboard(&self, painter: &egui::Painter, rect: Rect) {
        let keyboard_width = 60.0;

        for note_offset in 0..self.visible_note_count {
            let note = self.visible_note_start + note_offset;
            let y_top = rect.bottom() - (note_offset + 1) as f32 * self.pixels_per_note;
            let y_bottom = rect.bottom() - note_offset as f32 * self.pixels_per_note;

            // Determine if this is a black or white key
            let is_black_key = matches!(note % 12, 1 | 3 | 6 | 8 | 10);

            let key_color = if is_black_key {
                Color32::from_gray(60)
            } else {
                Color32::from_gray(200)
            };

            let key_rect = Rect::from_min_max(
                Pos2::new(rect.left(), y_top),
                Pos2::new(rect.left() + keyboard_width, y_bottom),
            );

            painter.rect_filled(key_rect, 0.0, key_color);
            painter.rect_stroke(key_rect, 0.0, (1.0, Color32::from_gray(100)));

            // Draw note name for C notes
            if note.is_multiple_of(12) {
                let note_name = Self::get_note_name(note);
                painter.text(
                    key_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    note_name,
                    egui::FontId::proportional(10.0),
                    Color32::BLACK,
                );
            }
        }
    }

    /// Draw notes in the pattern
    fn draw_notes(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        pattern: &Pattern,
        tempo: &Tempo,
        _time_signature: &TimeSignature,
        sample_rate: f64,
    ) {
        for note in pattern.notes() {
            // Skip notes outside visible range
            if note.pitch < self.visible_note_start
                || note.pitch >= self.visible_note_start + self.visible_note_count
            {
                continue;
            }

            // Convert start position to beats
            let start_beats = self.samples_to_beats(note.start.samples, sample_rate, tempo);

            // Convert duration to beats
            let duration_beats = self.samples_to_beats(note.duration_samples, sample_rate, tempo);

            // Calculate screen position
            let x_start = rect.left() + start_beats * self.pixels_per_beat;
            let x_end = x_start + duration_beats * self.pixels_per_beat;

            let note_offset = note.pitch - self.visible_note_start;
            let y_bottom = rect.bottom() - note_offset as f32 * self.pixels_per_note;
            let y_top = y_bottom - self.pixels_per_note;

            let note_rect =
                Rect::from_min_max(Pos2::new(x_start, y_top), Pos2::new(x_end, y_bottom));

            // Color based on velocity (darker = quieter)
            let velocity_normalized = note.velocity as f32 / 127.0;
            let base_color = Color32::from_rgb(100, 150, 255);
            let note_color = Color32::from_rgb(
                (base_color.r() as f32 * velocity_normalized) as u8,
                (base_color.g() as f32 * velocity_normalized) as u8,
                (base_color.b() as f32 * velocity_normalized) as u8,
            );

            // Highlight selected notes
            let is_selected = self.selected_notes.contains(&note.id);
            let final_color = if is_selected {
                Color32::from_rgb(255, 200, 100)
            } else {
                note_color
            };

            painter.rect_filled(note_rect, 2.0, final_color);
            painter.rect_stroke(note_rect, 2.0, (1.0, Color32::from_gray(150)));
        }
    }

    /// Draw the playback cursor showing current position
    #[allow(clippy::too_many_arguments)]
    fn draw_playback_cursor(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        pattern: &Pattern,
        tempo: &Tempo,
        time_signature: &TimeSignature,
        sample_rate: f64,
        current_position_samples: u64,
    ) {
        // Calculate pattern length in samples
        let pattern_length_samples = pattern.length_samples(sample_rate, tempo, time_signature);

        // If pattern is empty, don't draw cursor
        if pattern_length_samples == 0 {
            return;
        }

        // Normalize position within pattern (for looping)
        let normalized_position = current_position_samples % pattern_length_samples;

        // Convert position to beats
        let position_beats = self.samples_to_beats(normalized_position, sample_rate, tempo);

        // Calculate screen X position
        let x_pos = rect.left() + position_beats * self.pixels_per_beat;

        // Draw a vertical line for the playback cursor
        painter.line_segment(
            [
                Pos2::new(x_pos, rect.top()),
                Pos2::new(x_pos, rect.bottom()),
            ],
            egui::Stroke::new(2.0, Color32::from_rgb(255, 100, 100)), // Red cursor
        );
    }

    /// Handle user interactions (click, drag, etc.)
    ///
    /// Returns true if the pattern was modified
    #[allow(clippy::too_many_arguments)]
    fn handle_interactions(
        &mut self,
        response: &Response,
        rect: Rect,
        pattern: &mut Pattern,
        tempo: &Tempo,
        time_signature: &TimeSignature,
        sample_rate: f64,
        ui: &mut Ui,
    ) -> bool {
        let mut pattern_changed = false;
        // Handle drag start (primary button pressed)
        if response.drag_started()
            && let Some(pos) = response.interact_pointer_pos()
            && self.tool == PianoRollTool::Select
        {
            // Find note at click position to start dragging
            let pitch = self.screen_y_to_pitch(pos.y, rect);
            let beats = self.screen_x_to_beats(pos.x, rect);
            let samples = self.beats_to_samples(beats, sample_rate, tempo);

            for note in pattern.notes() {
                if note.pitch == pitch && note.contains_sample(samples) {
                    self.is_dragging = true;
                    self.drag_start_pos = Some(pos);
                    self.drag_note_id = Some(note.id);
                    break;
                }
            }
        }

        // Handle dragging
        if response.dragged()
            && self.is_dragging
            && let (Some(note_id), Some(current_pos)) =
                (self.drag_note_id, response.interact_pointer_pos())
            && let Some(note) = pattern.get_note_mut(note_id)
        {
            // Calculate new position
            let new_pitch = self.screen_y_to_pitch(current_pos.y, rect);
            let new_beats = self.screen_x_to_beats(current_pos.x, rect);

            // Snap to grid if enabled
            let snapped_beats = if self.snap_enabled {
                self.snap_to_grid(new_beats, time_signature)
            } else {
                new_beats
            };

            // Update note position
            let new_start_samples =
                self.beats_to_samples(snapped_beats.max(0.0), sample_rate, tempo);
            let new_position =
                Position::from_samples(new_start_samples, sample_rate, tempo, time_signature);

            note.pitch = new_pitch.clamp(0, 127);
            note.start = new_position;
        }

        // Handle drag end
        if response.drag_stopped() {
            if self.is_dragging {
                pattern_changed = true; // Pattern was modified by dragging
            }
            self.is_dragging = false;
            self.drag_start_pos = None;
            self.drag_note_id = None;
        }

        // Handle single click (not drag)
        if response.clicked()
            && !self.is_dragging
            && let Some(pos) = response.interact_pointer_pos()
        {
            match self.tool {
                PianoRollTool::Draw => {
                    self.add_note_at_position(
                        pos,
                        rect,
                        pattern,
                        tempo,
                        time_signature,
                        sample_rate,
                    );
                    pattern_changed = true; // Note added
                }
                PianoRollTool::Select => {
                    self.select_note_at_position(
                        pos,
                        rect,
                        pattern,
                        tempo,
                        time_signature,
                        sample_rate,
                    );
                }
                PianoRollTool::Erase => {
                    self.erase_note_at_position(
                        pos,
                        rect,
                        pattern,
                        tempo,
                        time_signature,
                        sample_rate,
                    );
                    pattern_changed = true; // Note erased
                }
            }
        }

        // Handle keyboard shortcuts
        ui.input(|input| {
            // Delete key removes selected notes
            if input.key_pressed(egui::Key::Delete) || input.key_pressed(egui::Key::Backspace) {
                let had_selection = !self.selected_notes.is_empty();
                self.delete_selected_notes(pattern);
                if had_selection {
                    pattern_changed = true; // Notes deleted
                }
            }

            // Ctrl+A selects all
            if input.modifiers.command && input.key_pressed(egui::Key::A) {
                self.select_all_notes(pattern);
            }
        });

        pattern_changed
    }

    /// Add a note at the clicked position
    fn add_note_at_position(
        &mut self,
        pos: Pos2,
        rect: Rect,
        pattern: &mut Pattern,
        tempo: &Tempo,
        time_signature: &TimeSignature,
        sample_rate: f64,
    ) {
        // Convert screen position to MIDI note and time
        let pitch = self.screen_y_to_pitch(pos.y, rect);
        let start_beats = self.screen_x_to_beats(pos.x, rect);

        // Snap to grid if enabled
        let snapped_beats = if self.snap_enabled {
            self.snap_to_grid(start_beats, time_signature)
        } else {
            start_beats
        };

        // Default duration: one beat
        let duration_beats = 1.0;

        // Convert to samples
        let start_samples = self.beats_to_samples(snapped_beats, sample_rate, tempo);
        let duration_samples = self.beats_to_samples(duration_beats, sample_rate, tempo);

        // Create position
        let start_position =
            Position::from_samples(start_samples, sample_rate, tempo, time_signature);

        // Create note
        let note = Note::new(
            generate_note_id(),
            pitch,
            start_position,
            duration_samples,
            100, // Default velocity
        );

        pattern.add_note(note);
    }

    /// Select note at position
    fn select_note_at_position(
        &mut self,
        pos: Pos2,
        rect: Rect,
        pattern: &Pattern,
        tempo: &Tempo,
        _time_signature: &TimeSignature,
        sample_rate: f64,
    ) {
        let pitch = self.screen_y_to_pitch(pos.y, rect);
        let beats = self.screen_x_to_beats(pos.x, rect);
        let samples = self.beats_to_samples(beats, sample_rate, tempo);

        // Find note at this position
        for note in pattern.notes() {
            if note.pitch == pitch && note.contains_sample(samples) {
                // Toggle selection
                if self.selected_notes.contains(&note.id) {
                    self.selected_notes.remove(&note.id);
                } else {
                    self.selected_notes.clear();
                    self.selected_notes.insert(note.id);
                }
                break;
            }
        }
    }

    /// Erase note at position
    fn erase_note_at_position(
        &mut self,
        pos: Pos2,
        rect: Rect,
        pattern: &mut Pattern,
        tempo: &Tempo,
        _time_signature: &TimeSignature,
        sample_rate: f64,
    ) {
        let pitch = self.screen_y_to_pitch(pos.y, rect);
        let beats = self.screen_x_to_beats(pos.x, rect);
        let samples = self.beats_to_samples(beats, sample_rate, tempo);

        // Find and remove note at this position
        let mut note_to_remove = None;
        for note in pattern.notes() {
            if note.pitch == pitch && note.contains_sample(samples) {
                note_to_remove = Some(note.id);
                break;
            }
        }

        if let Some(note_id) = note_to_remove {
            pattern.remove_note(note_id);
            self.selected_notes.remove(&note_id);
        }
    }

    /// Delete all selected notes
    fn delete_selected_notes(&mut self, pattern: &mut Pattern) {
        let notes_to_delete: Vec<NoteId> = self.selected_notes.iter().copied().collect();

        for note_id in notes_to_delete {
            pattern.remove_note(note_id);
        }

        self.selected_notes.clear();
    }

    /// Select all notes in pattern
    fn select_all_notes(&mut self, pattern: &Pattern) {
        self.selected_notes.clear();
        for note in pattern.notes() {
            self.selected_notes.insert(note.id);
        }
    }

    // Helper conversions

    fn screen_y_to_pitch(&self, y: f32, rect: Rect) -> u8 {
        let relative_y = rect.bottom() - y;
        let note_offset = (relative_y / self.pixels_per_note).floor() as i32;
        (self.visible_note_start as i32 + note_offset).clamp(0, 127) as u8
    }

    fn screen_x_to_beats(&self, x: f32, rect: Rect) -> f32 {
        let relative_x = x - rect.left();
        relative_x / self.pixels_per_beat
    }

    fn samples_to_beats(&self, samples: u64, sample_rate: f64, tempo: &Tempo) -> f32 {
        let seconds = samples as f64 / sample_rate;
        let beats = seconds / tempo.beat_duration_seconds();
        beats as f32
    }

    fn beats_to_samples(&self, beats: f32, sample_rate: f64, tempo: &Tempo) -> u64 {
        let seconds = beats as f64 * tempo.beat_duration_seconds();
        (seconds * sample_rate) as u64
    }

    fn snap_to_grid(&self, beats: f32, _time_signature: &TimeSignature) -> f32 {
        let subdivision_beats = 1.0 / self.snap_subdivision as f32;
        (beats / subdivision_beats).round() * subdivision_beats
    }

    fn get_note_name(midi_note: u8) -> String {
        const NOTE_NAMES: [&str; 12] = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
        let octave = (midi_note / 12) as i32 - 1;
        let note_index = (midi_note % 12) as usize;
        format!("{}{}", NOTE_NAMES[note_index], octave)
    }
}
