// Metronome - Click track generator for musical timing
// Generates sample-accurate metronome clicks on beats

use super::timeline::{Tempo, TimeSignature};
use std::f32::consts::PI;

/// Metronome click type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickType {
    /// Click on first beat of bar (accent/downbeat)
    Accent,
    /// Click on other beats
    Regular,
}

/// Metronome click sound generator
/// Pre-generates short click samples for low CPU overhead
#[derive(Debug, Clone)]
pub struct MetronomeSound {
    accent_samples: Vec<f32>,
    regular_samples: Vec<f32>,
}

impl MetronomeSound {
    /// Duration of click in samples
    const CLICK_DURATION_MS: f32 = 10.0;

    /// Create new metronome sound generator
    pub fn new(sample_rate: f32) -> Self {
        let click_samples = ((Self::CLICK_DURATION_MS / 1000.0) * sample_rate) as usize;

        Self {
            accent_samples: Self::generate_click(sample_rate, click_samples, 1200.0, 0.6),
            regular_samples: Self::generate_click(sample_rate, click_samples, 800.0, 0.4),
        }
    }

    /// Generate a short click sound using sine wave with envelope
    /// Higher frequency and amplitude for accent clicks
    fn generate_click(
        sample_rate: f32,
        num_samples: usize,
        frequency: f32,
        amplitude: f32,
    ) -> Vec<f32> {
        let mut samples = Vec::with_capacity(num_samples);
        let phase_increment = 2.0 * PI * frequency / sample_rate;

        for i in 0..num_samples {
            // Exponential decay envelope
            let t = i as f32 / num_samples as f32;
            let envelope = (-t * 8.0).exp(); // Fast decay

            // Sine wave oscillator
            let phase = i as f32 * phase_increment;
            let sample = phase.sin() * envelope * amplitude;

            samples.push(sample);
        }

        samples
    }

    /// Get click samples for given type
    pub fn get_click(&self, click_type: ClickType) -> &[f32] {
        match click_type {
            ClickType::Accent => &self.accent_samples,
            ClickType::Regular => &self.regular_samples,
        }
    }

    /// Get duration of click in samples
    pub fn click_duration(&self) -> usize {
        self.accent_samples.len()
    }
}

/// Active click playback state
#[derive(Debug, Clone)]
struct ClickPlayback {
    click_type: ClickType,
    position: usize, // Current position in click buffer
}

/// Metronome state for playback
/// Tracks when to generate clicks and maintains playback state
#[derive(Debug, Clone)]
pub struct Metronome {
    sound: MetronomeSound,
    enabled: bool,
    volume: f32,

    // Playback state
    current_click: Option<ClickPlayback>,
}

impl Metronome {
    /// Create new metronome
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sound: MetronomeSound::new(sample_rate),
            enabled: true,
            volume: 0.5,
            current_click: None,
        }
    }

    /// Enable/disable metronome
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.current_click = None;
        }
    }

    /// Check if metronome is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set metronome volume (0.0 to 1.0)
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    /// Get metronome volume
    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// Trigger a metronome click
    /// Call this when a beat occurs
    pub fn trigger_click(&mut self, click_type: ClickType) {
        if !self.enabled {
            return;
        }

        self.current_click = Some(ClickPlayback {
            click_type,
            position: 0,
        });
    }

    /// Process one sample of metronome output
    /// Returns the click sample (0.0 if no click active)
    pub fn process_sample(&mut self) -> f32 {
        if let Some(ref mut playback) = self.current_click {
            let click_samples = self.sound.get_click(playback.click_type);

            if playback.position < click_samples.len() {
                let sample = click_samples[playback.position] * self.volume;
                playback.position += 1;
                return sample;
            } else {
                // Click finished
                self.current_click = None;
            }
        }

        0.0
    }

    /// Process a buffer of metronome output
    /// This is more efficient than processing sample-by-sample
    pub fn process_buffer(&mut self, output: &mut [f32]) {
        for sample in output.iter_mut() {
            *sample = self.process_sample();
        }
    }

    /// Reset metronome state
    pub fn reset(&mut self) {
        self.current_click = None;
    }
}

/// Metronome scheduler
/// Determines when clicks should occur based on musical time
#[derive(Debug, Clone)]
pub struct MetronomeScheduler {
    last_beat: u64, // Last beat number that triggered a click
}

impl MetronomeScheduler {
    /// Create new scheduler
    pub fn new() -> Self {
        Self { last_beat: 0 }
    }

    /// Check if a click should occur in the current buffer
    /// Returns (sample_offset, ClickType) if a click should happen
    pub fn check_for_click(
        &mut self,
        buffer_start_samples: u64,
        buffer_size: usize,
        sample_rate: f64,
        tempo: &Tempo,
        time_signature: &TimeSignature,
    ) -> Option<(usize, ClickType)> {
        let buffer_end_samples = buffer_start_samples + buffer_size as u64;

        // Calculate beat duration in samples
        let beat_duration_samples = tempo.beat_duration_samples(sample_rate);

        // Calculate which beat we're on at buffer start and end
        let beat_start = (buffer_start_samples as f64 / beat_duration_samples) as u64;
        let beat_end = (buffer_end_samples as f64 / beat_duration_samples) as u64;

        // Check if we crossed a beat boundary
        if beat_end > beat_start && beat_end > self.last_beat {
            // A new beat occurred
            let beat_number = beat_end;
            self.last_beat = beat_number;

            // Calculate exact sample offset within buffer where beat occurs
            let beat_sample_position = (beat_number as f64 * beat_duration_samples) as u64;
            let offset = (beat_sample_position.saturating_sub(buffer_start_samples)) as usize;

            // Determine if it's an accent (first beat of bar)
            let beat_in_bar = (beat_number - 1) % time_signature.numerator as u64;
            let click_type = if beat_in_bar == 0 {
                ClickType::Accent
            } else {
                ClickType::Regular
            };

            return Some((offset, click_type));
        }

        None
    }

    /// Reset scheduler (e.g., when transport stops or position changes)
    pub fn reset(&mut self) {
        self.last_beat = 0;
    }

    /// Set current beat (when seeking in timeline)
    pub fn set_current_beat(&mut self, beat: u64) {
        self.last_beat = beat;
    }
}

impl Default for MetronomeScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metronome_sound_generation() {
        let sound = MetronomeSound::new(48000.0);

        let accent = sound.get_click(ClickType::Accent);
        let regular = sound.get_click(ClickType::Regular);

        // Both should have samples
        assert!(!accent.is_empty());
        assert!(!regular.is_empty());

        // Same duration
        assert_eq!(accent.len(), regular.len());

        // Expected duration: 10ms at 48kHz = 480 samples
        assert_eq!(accent.len(), 480);

        // Accent should be louder (higher peak amplitude)
        let accent_peak = accent.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        let regular_peak = regular.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(accent_peak > regular_peak);
    }

    #[test]
    fn test_metronome_click_playback() {
        let mut metronome = Metronome::new(48000.0);

        // Initially silent
        assert_eq!(metronome.process_sample(), 0.0);

        // Trigger click
        metronome.trigger_click(ClickType::Accent);

        // Should produce non-zero samples
        let mut non_zero_count = 0;
        for _ in 0..500 {
            let sample = metronome.process_sample();
            if sample.abs() > 0.0001 {
                non_zero_count += 1;
            }
        }

        assert!(non_zero_count > 400); // Most samples should be non-zero during click

        // After click finishes, should be silent again
        assert_eq!(metronome.process_sample(), 0.0);
    }

    #[test]
    fn test_metronome_volume_control() {
        let mut metronome = Metronome::new(48000.0);

        // Test at half volume - get peak sample
        metronome.set_volume(0.5);
        assert_eq!(metronome.volume(), 0.5);
        metronome.trigger_click(ClickType::Accent);
        let mut peak_half = 0.0f32;
        for _ in 0..500 {
            let sample = metronome.process_sample().abs();
            peak_half = peak_half.max(sample);
        }

        // Reset and test at full volume
        metronome.reset();
        metronome.set_volume(1.0);
        metronome.trigger_click(ClickType::Accent);
        let mut peak_full = 0.0f32;
        for _ in 0..500 {
            let sample = metronome.process_sample().abs();
            peak_full = peak_full.max(sample);
        }

        // Full volume peak should be approximately 2x half volume peak
        // Allow some tolerance for floating point math
        assert!(peak_full > peak_half * 1.8);
        assert!(peak_full < peak_half * 2.2);
    }

    #[test]
    fn test_metronome_enable_disable() {
        let mut metronome = Metronome::new(48000.0);

        assert!(metronome.is_enabled());

        metronome.set_enabled(false);
        assert!(!metronome.is_enabled());

        // Triggering click while disabled should do nothing
        metronome.trigger_click(ClickType::Accent);
        assert_eq!(metronome.process_sample(), 0.0);
    }

    #[test]
    fn test_scheduler_basic() {
        let mut scheduler = MetronomeScheduler::new();
        let tempo = Tempo::new(120.0);
        let ts = TimeSignature::four_four();
        let sample_rate = 48000.0;

        // At 120 BPM, one beat = 0.5s = 24000 samples
        let _beat_duration = 24000u64;

        // First buffer (0-512 samples): no click yet
        let result = scheduler.check_for_click(0, 512, sample_rate, &tempo, &ts);
        assert!(result.is_none());

        // Buffer crossing first beat (23500-24012 samples)
        let result = scheduler.check_for_click(23500, 512, sample_rate, &tempo, &ts);
        assert!(result.is_some());

        let (offset, click_type) = result.unwrap();
        assert_eq!(click_type, ClickType::Accent); // First beat of bar
        assert!(offset < 512); // Within buffer

        // Second beat
        let result = scheduler.check_for_click(47500, 512, sample_rate, &tempo, &ts);
        assert!(result.is_some());
        let (_, click_type) = result.unwrap();
        assert_eq!(click_type, ClickType::Regular); // Not first beat
    }

    #[test]
    fn test_scheduler_accent_pattern() {
        let mut scheduler = MetronomeScheduler::new();
        let tempo = Tempo::new(120.0);
        let ts = TimeSignature::four_four();
        let sample_rate = 48000.0;

        let beat_duration = 24000u64;

        // Collect click types for 8 beats
        let mut click_types = Vec::new();
        for beat_num in 1..=8 {
            let buffer_start = beat_duration * beat_num - 512;
            if let Some((_, click_type)) =
                scheduler.check_for_click(buffer_start, 512, sample_rate, &tempo, &ts)
            {
                click_types.push(click_type);
            }
        }

        // In 4/4 time: Accent, Regular, Regular, Regular, Accent, Regular, Regular, Regular
        assert_eq!(click_types.len(), 8);
        assert_eq!(click_types[0], ClickType::Accent); // Beat 1
        assert_eq!(click_types[1], ClickType::Regular); // Beat 2
        assert_eq!(click_types[2], ClickType::Regular); // Beat 3
        assert_eq!(click_types[3], ClickType::Regular); // Beat 4
        assert_eq!(click_types[4], ClickType::Accent); // Beat 5 (bar 2)
        assert_eq!(click_types[5], ClickType::Regular); // Beat 6
    }

    #[test]
    fn test_scheduler_different_time_signature() {
        let mut scheduler = MetronomeScheduler::new();
        let tempo = Tempo::new(120.0);
        let ts = TimeSignature::three_four(); // 3/4 time
        let sample_rate = 48000.0;

        let beat_duration = 24000u64;

        // Collect click types for 6 beats (2 bars of 3/4)
        let mut click_types = Vec::new();
        for beat_num in 1..=6 {
            let buffer_start = beat_duration * beat_num - 512;
            if let Some((_, click_type)) =
                scheduler.check_for_click(buffer_start, 512, sample_rate, &tempo, &ts)
            {
                click_types.push(click_type);
            }
        }

        // In 3/4 time: Accent, Regular, Regular, Accent, Regular, Regular
        assert_eq!(click_types[0], ClickType::Accent); // Beat 1
        assert_eq!(click_types[1], ClickType::Regular); // Beat 2
        assert_eq!(click_types[2], ClickType::Regular); // Beat 3
        assert_eq!(click_types[3], ClickType::Accent); // Beat 4 (bar 2)
        assert_eq!(click_types[4], ClickType::Regular); // Beat 5
        assert_eq!(click_types[5], ClickType::Regular); // Beat 6
    }

    #[test]
    fn test_scheduler_reset() {
        let mut scheduler = MetronomeScheduler::new();
        let tempo = Tempo::new(120.0);
        let ts = TimeSignature::four_four();
        let sample_rate = 48000.0;

        // Process first beat
        let _ = scheduler.check_for_click(23500, 512, sample_rate, &tempo, &ts);
        assert_eq!(scheduler.last_beat, 1);

        // Reset
        scheduler.reset();
        assert_eq!(scheduler.last_beat, 0);

        // Should trigger click again at same position
        let result = scheduler.check_for_click(23500, 512, sample_rate, &tempo, &ts);
        assert!(result.is_some());
    }

    #[test]
    fn test_metronome_buffer_processing() {
        let mut metronome = Metronome::new(48000.0);
        let mut buffer = vec![0.0f32; 512];

        metronome.trigger_click(ClickType::Accent);
        metronome.process_buffer(&mut buffer);

        // Should have non-zero samples at start
        let non_zero = buffer
            .iter()
            .take(480)
            .filter(|&&s| s.abs() > 0.0001)
            .count();
        assert!(non_zero > 400);

        // Rest should be silent
        let silent = buffer.iter().skip(480).all(|&s| s.abs() < 0.0001);
        assert!(silent);
    }
}
