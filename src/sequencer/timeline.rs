// Timeline - Musical time representation
// Handles conversion between samples, beats, bars, and real time

use std::fmt;

/// Time signature (numerator/denominator)
/// Example: 4/4 time = TimeSignature { numerator: 4, denominator: 4 }
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TimeSignature {
    pub numerator: u8,   // Beats per bar (typically 3, 4, 5, 6, 7)
    pub denominator: u8, // Note value (4 = quarter note, 8 = eighth note)
}

impl TimeSignature {
    /// Creates a new time signature
    pub fn new(numerator: u8, denominator: u8) -> Self {
        assert!(numerator > 0, "Time signature numerator must be > 0");
        assert!(
            denominator.is_power_of_two(),
            "Time signature denominator must be power of 2"
        );
        Self {
            numerator,
            denominator,
        }
    }

    /// Common 4/4 time signature
    pub fn four_four() -> Self {
        Self::new(4, 4)
    }

    /// Common 3/4 time signature (waltz)
    pub fn three_four() -> Self {
        Self::new(3, 4)
    }

    /// Common 6/8 time signature
    pub fn six_eight() -> Self {
        Self::new(6, 8)
    }

    /// Number of beats per bar
    pub fn beats_per_bar(&self) -> f64 {
        self.numerator as f64
    }

    /// Beat duration relative to quarter note
    /// Example: 4/4 = 1.0, 6/8 = 0.5 (eighth notes)
    pub fn beat_duration_multiplier(&self) -> f64 {
        4.0 / self.denominator as f64
    }
}

impl Default for TimeSignature {
    fn default() -> Self {
        Self::four_four()
    }
}

impl fmt::Display for TimeSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

/// Tempo in BPM (Beats Per Minute)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tempo {
    bpm: f64,
}

impl Tempo {
    /// Creates a new tempo
    /// BPM must be in range [20.0, 999.0]
    pub fn new(bpm: f64) -> Self {
        assert!(
            (20.0..=999.0).contains(&bpm),
            "BPM must be between 20 and 999"
        );
        Self { bpm }
    }

    /// Get BPM value
    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    /// Set BPM value
    pub fn set_bpm(&mut self, bpm: f64) {
        assert!(
            (20.0..=999.0).contains(&bpm),
            "BPM must be between 20 and 999"
        );
        self.bpm = bpm;
    }

    /// Duration of one beat in seconds
    pub fn beat_duration_seconds(&self) -> f64 {
        60.0 / self.bpm
    }

    /// Duration of one beat in samples at given sample rate
    pub fn beat_duration_samples(&self, sample_rate: f64) -> f64 {
        self.beat_duration_seconds() * sample_rate
    }

    /// Duration of one bar in seconds at given time signature
    pub fn bar_duration_seconds(&self, time_signature: &TimeSignature) -> f64 {
        self.beat_duration_seconds() * time_signature.beats_per_bar()
    }

    /// Duration of one bar in samples at given sample rate and time signature
    pub fn bar_duration_samples(&self, sample_rate: f64, time_signature: &TimeSignature) -> f64 {
        self.bar_duration_seconds(time_signature) * sample_rate
    }
}

impl Default for Tempo {
    fn default() -> Self {
        Self::new(120.0)
    }
}

impl fmt::Display for Tempo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1} BPM", self.bpm)
    }
}

/// Musical time representation
/// Represents a position in the timeline using bars, beats, and ticks
/// Tick = subdivision of a beat (typically 480 or 960 ticks per quarter note)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MusicalTime {
    pub bar: u32,  // Bar number (1-based)
    pub beat: u8,  // Beat within bar (1-based)
    pub tick: u16, // Tick within beat (0-based)
}

impl MusicalTime {
    /// Ticks per quarter note (PPQN - Pulses Per Quarter Note)
    /// Standard MIDI resolution
    pub const TICKS_PER_QUARTER: u16 = 480;

    /// Creates a new musical time position
    pub fn new(bar: u32, beat: u8, tick: u16) -> Self {
        Self { bar, beat, tick }
    }

    /// Zero position (bar 1, beat 1, tick 0)
    pub fn zero() -> Self {
        Self::new(1, 1, 0)
    }

    /// Convert to total ticks from start
    /// Useful for arithmetic operations
    pub fn to_total_ticks(&self, time_signature: &TimeSignature) -> u64 {
        let ticks_per_beat = Self::TICKS_PER_QUARTER;
        let beats_per_bar = time_signature.numerator as u64;
        let ticks_per_bar = beats_per_bar * ticks_per_beat as u64;

        // Convert to 0-based for calculation
        let bar_0 = (self.bar - 1) as u64;
        let beat_0 = (self.beat - 1) as u64;

        bar_0 * ticks_per_bar + beat_0 * ticks_per_beat as u64 + self.tick as u64
    }

    /// Create from total ticks
    pub fn from_total_ticks(total_ticks: u64, time_signature: &TimeSignature) -> Self {
        let ticks_per_beat = Self::TICKS_PER_QUARTER as u64;
        let beats_per_bar = time_signature.numerator as u64;
        let ticks_per_bar = beats_per_bar * ticks_per_beat;

        let bar = (total_ticks / ticks_per_bar) + 1; // 1-based
        let remaining_after_bars = total_ticks % ticks_per_bar;
        let beat = (remaining_after_bars / ticks_per_beat) + 1; // 1-based
        let tick = remaining_after_bars % ticks_per_beat;

        Self::new(bar as u32, beat as u8, tick as u16)
    }

    /// Quantize to nearest beat
    pub fn quantize_to_beat(&self, time_signature: &TimeSignature) -> Self {
        let total_ticks = self.to_total_ticks(time_signature);
        let ticks_per_beat = Self::TICKS_PER_QUARTER as u64;

        // Round to nearest beat
        let quantized_ticks =
            ((total_ticks + ticks_per_beat / 2) / ticks_per_beat) * ticks_per_beat;

        Self::from_total_ticks(quantized_ticks, time_signature)
    }

    /// Quantize to nearest subdivision of a beat
    /// Example: subdivision = 4 for sixteenth notes
    pub fn quantize_to_subdivision(
        &self,
        time_signature: &TimeSignature,
        subdivision: u16,
    ) -> Self {
        let total_ticks = self.to_total_ticks(time_signature);
        let ticks_per_subdivision = (Self::TICKS_PER_QUARTER / subdivision) as u64;

        // Round to nearest subdivision
        let quantized_ticks = ((total_ticks + ticks_per_subdivision / 2) / ticks_per_subdivision)
            * ticks_per_subdivision;

        Self::from_total_ticks(quantized_ticks, time_signature)
    }
}

impl Default for MusicalTime {
    fn default() -> Self {
        Self::zero()
    }
}

impl fmt::Display for MusicalTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{:02}:{:03}", self.bar, self.beat, self.tick)
    }
}

/// Position in the timeline
/// Can represent time in multiple formats simultaneously
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub samples: u64,         // Absolute position in samples
    pub musical: MusicalTime, // Musical position (bars:beats:ticks)
}

impl Position {
    /// Create a new position
    pub fn new(samples: u64, musical: MusicalTime) -> Self {
        Self { samples, musical }
    }

    /// Zero position
    pub fn zero() -> Self {
        Self {
            samples: 0,
            musical: MusicalTime::zero(),
        }
    }

    /// Create from samples
    pub fn from_samples(
        samples: u64,
        sample_rate: f64,
        tempo: &Tempo,
        time_signature: &TimeSignature,
    ) -> Self {
        // Convert samples to seconds
        let seconds = samples as f64 / sample_rate;

        // Convert seconds to beats
        let beats = seconds / tempo.beat_duration_seconds();

        // Convert beats to ticks
        let total_ticks = (beats * MusicalTime::TICKS_PER_QUARTER as f64) as u64;

        // Create musical time from ticks
        let musical = MusicalTime::from_total_ticks(total_ticks, time_signature);

        Self { samples, musical }
    }

    /// Create from musical time
    pub fn from_musical(
        musical: MusicalTime,
        sample_rate: f64,
        tempo: &Tempo,
        time_signature: &TimeSignature,
    ) -> Self {
        // Convert to total ticks
        let total_ticks = musical.to_total_ticks(time_signature);

        // Convert ticks to beats
        let beats = total_ticks as f64 / MusicalTime::TICKS_PER_QUARTER as f64;

        // Convert beats to seconds
        let seconds = beats * tempo.beat_duration_seconds();

        // Convert seconds to samples
        let samples = (seconds * sample_rate) as u64;

        Self { samples, musical }
    }

    /// Add samples to position
    pub fn add_samples(
        &self,
        delta_samples: u64,
        sample_rate: f64,
        tempo: &Tempo,
        time_signature: &TimeSignature,
    ) -> Self {
        Self::from_samples(
            self.samples + delta_samples,
            sample_rate,
            tempo,
            time_signature,
        )
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::zero()
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.musical, self.samples)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_signature() {
        let ts = TimeSignature::four_four();
        assert_eq!(ts.numerator, 4);
        assert_eq!(ts.denominator, 4);
        assert_eq!(ts.beats_per_bar(), 4.0);
        assert_eq!(ts.to_string(), "4/4");
    }

    #[test]
    fn test_tempo() {
        let tempo = Tempo::new(120.0);
        assert_eq!(tempo.bpm(), 120.0);
        assert_eq!(tempo.beat_duration_seconds(), 0.5);

        // At 120 BPM, one beat = 0.5s
        // At 48000 Hz, one beat = 24000 samples
        assert_eq!(tempo.beat_duration_samples(48000.0), 24000.0);
    }

    #[test]
    fn test_musical_time_conversion() {
        let ts = TimeSignature::four_four();

        // Bar 1, beat 1, tick 0 = 0 total ticks
        let time1 = MusicalTime::new(1, 1, 0);
        assert_eq!(time1.to_total_ticks(&ts), 0);

        // Bar 1, beat 2, tick 0 = 480 ticks (one quarter note)
        let time2 = MusicalTime::new(1, 2, 0);
        assert_eq!(time2.to_total_ticks(&ts), 480);

        // Bar 2, beat 1, tick 0 = 1920 ticks (4 beats)
        let time3 = MusicalTime::new(2, 1, 0);
        assert_eq!(time3.to_total_ticks(&ts), 1920);

        // Round trip
        let total = 1000u64;
        let converted = MusicalTime::from_total_ticks(total, &ts);
        assert_eq!(converted.to_total_ticks(&ts), total);
    }

    #[test]
    fn test_musical_time_quantization() {
        let ts = TimeSignature::four_four();

        // Bar 1, beat 1, tick 240 (halfway through beat)
        // Should quantize to bar 1, beat 1, tick 480 (start of beat 2)
        let time = MusicalTime::new(1, 1, 240);
        let quantized = time.quantize_to_beat(&ts);
        assert_eq!(quantized, MusicalTime::new(1, 2, 0));

        // Test sixteenth note quantization (4 subdivisions per beat)
        let time2 = MusicalTime::new(1, 1, 100);
        let quantized2 = time2.quantize_to_subdivision(&ts, 4);
        // 480 / 4 = 120 ticks per sixteenth
        // 100 rounds to 120
        assert_eq!(quantized2.tick, 120);
    }

    #[test]
    fn test_position_conversion() {
        let sample_rate = 48000.0;
        let tempo = Tempo::new(120.0);
        let ts = TimeSignature::four_four();

        // At 120 BPM, one beat = 0.5s = 24000 samples
        let pos1 = Position::from_samples(24000, sample_rate, &tempo, &ts);
        assert_eq!(pos1.samples, 24000);
        assert_eq!(pos1.musical.bar, 1);
        assert_eq!(pos1.musical.beat, 2); // Second beat

        // Round trip: create from musical time
        let musical = MusicalTime::new(1, 3, 0); // Third beat
        let pos2 = Position::from_musical(musical, sample_rate, &tempo, &ts);
        assert_eq!(pos2.samples, 48000); // 2 beats * 24000 samples/beat
        assert_eq!(pos2.musical, musical);
    }

    #[test]
    fn test_position_arithmetic() {
        let sample_rate = 48000.0;
        let tempo = Tempo::new(120.0);
        let ts = TimeSignature::four_four();

        let pos = Position::zero();
        let new_pos = pos.add_samples(24000, sample_rate, &tempo, &ts);

        assert_eq!(new_pos.samples, 24000);
        assert_eq!(new_pos.musical.bar, 1);
        assert_eq!(new_pos.musical.beat, 2);
    }

    #[test]
    fn test_different_time_signatures() {
        let ts_34 = TimeSignature::three_four();
        let ts_68 = TimeSignature::six_eight();

        // 3/4: 3 beats per bar
        assert_eq!(ts_34.beats_per_bar(), 3.0);

        // 6/8: 6 beats per bar (but beat = eighth note)
        assert_eq!(ts_68.beats_per_bar(), 6.0);

        // Bar 2 in 3/4 time
        let time_34 = MusicalTime::new(2, 1, 0);
        // Should be 3 * 480 = 1440 ticks from start
        assert_eq!(time_34.to_total_ticks(&ts_34), 1440);

        // Bar 2 in 6/8 time
        let time_68 = MusicalTime::new(2, 1, 0);
        // Should be 6 * 480 = 2880 ticks from start
        assert_eq!(time_68.to_total_ticks(&ts_68), 2880);
    }
}
