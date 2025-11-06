// Audio Export - Offline rendering to WAV/FLAC files
//
// This module provides offline audio rendering capabilities, allowing
// the user to export their project to audio files. Unlike the real-time
// audio callback, this processes audio as fast as possible without time
// constraints.

use crate::audio::dsp_utils::{OnePoleSmoother, flush_denormals_to_zero, soft_clip};
use crate::midi::event::{MidiEvent, MidiEventTimed};
use crate::sequencer::metronome::{Metronome, MetronomeScheduler};
use crate::sequencer::{Pattern, SequencerPlayer, Tempo, TimeSignature};
use crate::synth::voice_manager::VoiceManager;
use hound::{WavSpec, WavWriter};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

/// Audio export format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// WAV format (uncompressed)
    Wav,
    /// FLAC format (lossless compression)
    Flac,
}

impl ExportFormat {
    /// Get file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Wav => "wav",
            ExportFormat::Flac => "flac",
        }
    }
}

/// Audio export settings
#[derive(Debug, Clone)]
pub struct ExportSettings {
    /// Output file path
    pub output_path: String,
    /// Export format (WAV or FLAC)
    pub format: ExportFormat,
    /// Sample rate (Hz)
    pub sample_rate: u32,
    /// Bit depth (16 or 24)
    pub bit_depth: u16,
    /// Number of channels (1=mono, 2=stereo)
    pub channels: u16,
    /// Include metronome in export
    pub include_metronome: bool,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            output_path: "export.wav".to_string(),
            format: ExportFormat::Wav,
            sample_rate: 44100,
            bit_depth: 16,
            channels: 2,
            include_metronome: false,
        }
    }
}

/// Progress callback for export (reports 0.0 to 1.0).
/// The callback should update a shared state (e.g., Arc<Mutex<f32>>) or send progress via a channel to the UI.
pub type ProgressCallback = Box<dyn FnMut(f32) + Send>;

/// Audio exporter - renders project to audio file
pub struct AudioExporter {
    settings: ExportSettings,
}

impl AudioExporter {
    /// Create a new audio exporter
    pub fn new(settings: ExportSettings) -> Self {
        Self { settings }
    }

    /// Export a pattern to audio file
    ///
    /// # Arguments
    /// * `pattern` - The pattern to render
    /// * `tempo` - Tempo for playback
    /// * `time_signature` - Time signature
    /// * `duration_seconds` - Total duration to render (None = auto-detect from pattern)
    /// * `progress_callback` - Optional callback for progress updates
    ///
    /// # Returns
    /// Result with success message or error
    pub fn export(
        &self,
        pattern: &Pattern,
        tempo: &Tempo,
        time_signature: &TimeSignature,
        duration_seconds: Option<f64>,
        mut progress_callback: Option<ProgressCallback>,
    ) -> Result<String, String> {
        // Calculate total duration
        let sample_rate_f64 = self.settings.sample_rate as f64;
        let total_duration = duration_seconds.unwrap_or_else(|| {
            // Auto-detect from pattern length (in samples)
            let pattern_length_samples =
                pattern.length_samples(sample_rate_f64, tempo, time_signature);
            pattern_length_samples as f64 / sample_rate_f64
        });

        if total_duration <= 0.0 {
            return Err("Invalid duration: must be > 0".to_string());
        }

        let total_samples = (total_duration * sample_rate_f64) as u64;

        println!(
            "Exporting audio: {:.2}s ({} samples) at {} Hz",
            total_duration, total_samples, self.settings.sample_rate
        );

        // Export based on format
        match self.settings.format {
            ExportFormat::Wav => self.export_wav(
                pattern,
                tempo,
                time_signature,
                total_samples,
                progress_callback.as_mut(),
            ),
            ExportFormat::Flac => self.export_flac(
                pattern,
                tempo,
                time_signature,
                total_samples,
                progress_callback.as_mut(),
            ),
        }
    }

    /// Export to WAV format
    fn export_wav(
        &self,
        pattern: &Pattern,
        tempo: &Tempo,
        time_signature: &TimeSignature,
        total_samples: u64,
        mut progress_callback: Option<&mut ProgressCallback>,
    ) -> Result<String, String> {
        // Create WAV spec
        let spec = WavSpec {
            channels: self.settings.channels,
            sample_rate: self.settings.sample_rate,
            bits_per_sample: self.settings.bit_depth,
            sample_format: hound::SampleFormat::Int,
        };

        // Create WAV writer
        let path = Path::new(&self.settings.output_path);
        let writer = WavWriter::create(path, spec)
            .map_err(|e| format!("Failed to create WAV file: {}", e))?;

        // Render audio
        self.render_audio(
            writer,
            pattern,
            tempo,
            time_signature,
            total_samples,
            progress_callback,
        )?;

        Ok(format!(
            "Successfully exported to {}",
            self.settings.output_path
        ))
    }

    /// Export to FLAC format
    fn export_flac(
        &self,
        pattern: &Pattern,
        tempo: &Tempo,
        time_signature: &TimeSignature,
        total_samples: u64,
        mut progress_callback: Option<&mut ProgressCallback>,
    ) -> Result<String, String> {
        // FLAC export using hound (which supports FLAC via feature flag)
        // For now, we'll just export as WAV and recommend using external tools for FLAC
        // TODO: Add proper FLAC support with claxon or similar

        // For now, change extension to .wav and export as WAV
        let mut settings = self.settings.clone();
        settings.output_path = settings.output_path.replace(".flac", ".wav");

        println!("Note: FLAC export not yet implemented, exporting as WAV instead");

        let exporter = AudioExporter::new(settings);
        exporter.export_wav(
            pattern,
            tempo,
            time_signature,
            total_samples,
            progress_callback,
        )
    }

    /// Render audio to a WAV writer
    fn render_audio(
        &self,
        mut writer: WavWriter<BufWriter<File>>,
        pattern: &Pattern,
        tempo: &Tempo,
        time_signature: &TimeSignature,
        total_samples: u64,
        mut progress_callback: Option<&mut ProgressCallback>,
    ) -> Result<(), String> {
        // Create a new voice manager for offline rendering
        // TODO Phase 4+: Copy settings from active voice manager
        let mut voice_manager = VoiceManager::new(self.settings.sample_rate as f32);

        // Create sequencer player
        let mut sequencer_player = SequencerPlayer::new(self.settings.sample_rate as f64);

        // Create volume smoother (using default 50% volume)
        let mut volume_smoother = OnePoleSmoother::new(0.5, 10.0, self.settings.sample_rate as f32);

        // Create metronome (if enabled)
        let mut metronome = if self.settings.include_metronome {
            Some(Metronome::new(self.settings.sample_rate as f32))
        } else {
            None
        };
        let mut metronome_scheduler = if self.settings.include_metronome {
            Some(MetronomeScheduler::new())
        } else {
            None
        };

        // Buffer size for rendering (process in chunks)
        const BUFFER_SIZE: usize = 512;
        let mut current_position: u64 = 0;

        // Progress tracking
        let mut samples_processed: u64 = 0;
        let progress_update_interval = self.settings.sample_rate as u64; // Update every 1 second

        println!("Starting audio rendering...");

        // Main rendering loop
        while current_position < total_samples {
            // Calculate how many samples to render in this iteration
            let samples_to_render = BUFFER_SIZE.min((total_samples - current_position) as usize);

            // Get MIDI events from sequencer player
            let midi_events = sequencer_player.process(
                pattern,
                current_position,
                true, // is_playing
                tempo,
                time_signature,
                samples_to_render,
            );

            // Process MIDI events
            for timed_event in midi_events {
                self.process_midi_event(timed_event, &mut voice_manager);
            }

            // Generate and write audio samples
            for i in 0..samples_to_render {
                // Generate synth sample (stereo)
                let (mut left, mut right) = voice_manager.next_sample();

                // Add metronome if enabled
                if let (Some(ref mut scheduler), Some(ref mut metro)) =
                    (metronome_scheduler.as_mut(), metronome.as_mut())
                {
                    // Check if a click should occur in this buffer
                    let sample_position = current_position + i as u64;
                    if let Some((offset, click_type)) = scheduler.check_for_click(
                        sample_position,
                        1,
                        self.settings.sample_rate as f64,
                        tempo,
                        time_signature,
                    ) {
                        if offset == 0 {
                            metro.trigger_click(click_type);
                        }
                    }

                    // Process metronome sample
                    let click_sample = metro.process_sample();
                    left += click_sample;
                    right += click_sample;
                }

                // Apply DSP to both channels
                left = flush_denormals_to_zero(left);
                right = flush_denormals_to_zero(right);

                let volume = volume_smoother.process(0.5); // Fixed 50% volume for export
                left *= volume;
                right *= volume;

                left = soft_clip(left);
                right = soft_clip(right);

                // Convert to i16 and write
                if self.settings.channels == 2 {
                    // Stereo: write both channels
                    let left_i16 = (left * i16::MAX as f32) as i16;
                    let right_i16 = (right * i16::MAX as f32) as i16;
                    writer
                        .write_sample(left_i16)
                        .map_err(|e| format!("Failed to write sample: {}", e))?;
                    writer
                        .write_sample(right_i16)
                        .map_err(|e| format!("Failed to write sample: {}", e))?;
                } else {
                    // Mono: mix down to mono
                    let mono = (left + right) * 0.5;
                    let mono_i16 = (mono * i16::MAX as f32) as i16;
                    writer
                        .write_sample(mono_i16)
                        .map_err(|e| format!("Failed to write sample: {}", e))?;
                }

                current_position += 1;
                samples_processed += 1;

                // Update progress callback
                if samples_processed.is_multiple_of(progress_update_interval) {
                    if let Some(ref mut callback) = progress_callback {
                        let progress = current_position as f32 / total_samples as f32;
                        callback(progress);
                    }
                }
            }
        }

        // Finalize WAV file
        writer
            .finalize()
            .map_err(|e| format!("Failed to finalize WAV file: {}", e))?;

        println!("Audio rendering complete!");

        // Call progress callback one last time with 1.0
        if let Some(ref mut callback) = progress_callback {
            callback(1.0);
        }

        Ok(())
    }

    /// Process a MIDI event (helper function)
    fn process_midi_event(&self, timed_event: MidiEventTimed, voice_manager: &mut VoiceManager) {
        // Process event immediately (samples_from_now is handled by sequencer)
        match timed_event.event {
            MidiEvent::NoteOn { note, velocity } => {
                voice_manager.note_on(note, velocity);
            }
            MidiEvent::NoteOff { note } => {
                voice_manager.note_off(note);
            }
            MidiEvent::ChannelAftertouch { value } => {
                voice_manager.set_aftertouch(value);
            }
            _ => {} // Ignore other events for now
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequencer::{Note, Position};
    use tempfile::tempdir;

    #[test]
    fn test_export_settings_default() {
        let settings = ExportSettings::default();
        assert_eq!(settings.sample_rate, 44100);
        assert_eq!(settings.bit_depth, 16);
        assert_eq!(settings.channels, 2);
        assert_eq!(settings.format, ExportFormat::Wav);
    }

    #[test]
    fn test_export_format_extension() {
        assert_eq!(ExportFormat::Wav.extension(), "wav");
        assert_eq!(ExportFormat::Flac.extension(), "flac");
    }

    #[test]
    fn test_export_empty_pattern() {
        let dir = tempdir().unwrap();
        let output_path = dir.path().join("test.wav");

        let settings = ExportSettings {
            output_path: output_path.to_str().unwrap().to_string(),
            format: ExportFormat::Wav,
            sample_rate: 44100,
            bit_depth: 16,
            channels: 2,
            include_metronome: false,
        };

        let exporter = AudioExporter::new(settings);
        let pattern = Pattern::new_default(1, "Test".to_string());
        let tempo = Tempo::new(120.0);
        let time_signature = TimeSignature::four_four();

        // Export 1 second of silence
        let result = exporter.export(&pattern, &tempo, &time_signature, Some(1.0), None);

        assert!(result.is_ok(), "Export should succeed: {:?}", result);
        assert!(output_path.exists(), "Output file should exist");
    }

    #[test]
    fn test_export_with_notes() {
        let dir = tempdir().unwrap();
        let output_path = dir.path().join("test_notes.wav");

        let settings = ExportSettings {
            output_path: output_path.to_str().unwrap().to_string(),
            format: ExportFormat::Wav,
            sample_rate: 44100,
            bit_depth: 16,
            channels: 2,
            include_metronome: false,
        };

        let exporter = AudioExporter::new(settings);
        let mut pattern = Pattern::new_default(1, "Test".to_string());

        // Add a note (middle C for 0.5 seconds)
        let note = Note::new(
            1,
            60,
            Position::zero(),
            22050, // 0.5s at 44.1kHz
            100,
        );
        pattern.add_note(note);

        let tempo = Tempo::new(120.0);
        let time_signature = TimeSignature::four_four();

        // Export 1 second (note will play for first 0.5s)
        let result = exporter.export(&pattern, &tempo, &time_signature, Some(1.0), None);

        assert!(result.is_ok(), "Export should succeed: {:?}", result);
        assert!(output_path.exists(), "Output file should exist");

        // Check file size is reasonable (should be > 0)
        let metadata = std::fs::metadata(&output_path).unwrap();
        assert!(metadata.len() > 1000, "File should contain audio data");
    }
}
