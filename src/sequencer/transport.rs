// Transport - Playback control and state management
// Controls play/stop/record state and playhead position

use super::timeline::{Position, Tempo, TimeSignature};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

/// Transport state (play/stop/record)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    Stopped,
    Playing,
    Recording,
    Paused,
}

impl TransportState {
    /// Check if transport is in a playing state (Playing or Recording)
    pub fn is_playing(&self) -> bool {
        matches!(self, TransportState::Playing | TransportState::Recording)
    }

    /// Check if transport is recording
    pub fn is_recording(&self) -> bool {
        matches!(self, TransportState::Recording)
    }

    /// Check if transport is stopped or paused
    pub fn is_stopped(&self) -> bool {
        matches!(self, TransportState::Stopped | TransportState::Paused)
    }
}

impl Default for TransportState {
    fn default() -> Self {
        TransportState::Stopped
    }
}

/// Shared transport state
/// Thread-safe via atomics for communication with audio thread
#[derive(Debug)]
pub struct SharedTransportState {
    playing: AtomicBool,
    recording: AtomicBool,
    paused: AtomicBool,
    position_samples: AtomicU64,
    loop_enabled: AtomicBool,
    loop_start_samples: AtomicU64,
    loop_end_samples: AtomicU64,
}

impl SharedTransportState {
    /// Create new shared transport state
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            playing: AtomicBool::new(false),
            recording: AtomicBool::new(false),
            paused: AtomicBool::new(false),
            position_samples: AtomicU64::new(0),
            loop_enabled: AtomicBool::new(false),
            loop_start_samples: AtomicU64::new(0),
            loop_end_samples: AtomicU64::new(0),
        })
    }

    /// Get current transport state
    pub fn state(&self) -> TransportState {
        if self.recording.load(Ordering::Relaxed) {
            TransportState::Recording
        } else if self.playing.load(Ordering::Relaxed) {
            TransportState::Playing
        } else if self.paused.load(Ordering::Relaxed) {
            TransportState::Paused
        } else {
            TransportState::Stopped
        }
    }

    /// Get current position in samples
    pub fn position_samples(&self) -> u64 {
        self.position_samples.load(Ordering::Relaxed)
    }

    /// Set position in samples
    pub fn set_position_samples(&self, samples: u64) {
        self.position_samples.store(samples, Ordering::Relaxed);
    }

    /// Advance position by given number of samples
    /// Returns new position (handles looping if enabled)
    pub fn advance_position(&self, delta_samples: u64) -> u64 {
        let current = self.position_samples.load(Ordering::Relaxed);
        let mut new_pos = current + delta_samples;

        // Handle looping
        if self.loop_enabled.load(Ordering::Relaxed) {
            let loop_start = self.loop_start_samples.load(Ordering::Relaxed);
            let loop_end = self.loop_end_samples.load(Ordering::Relaxed);

            if loop_end > loop_start && new_pos >= loop_end {
                // Loop back to start
                let loop_length = loop_end - loop_start;
                let overflow = new_pos - loop_end;
                new_pos = loop_start + (overflow % loop_length);
            }
        }

        self.position_samples.store(new_pos, Ordering::Relaxed);
        new_pos
    }

    /// Check if loop is enabled
    pub fn is_loop_enabled(&self) -> bool {
        self.loop_enabled.load(Ordering::Relaxed)
    }

    /// Get loop region (start, end) in samples
    pub fn loop_region(&self) -> (u64, u64) {
        (
            self.loop_start_samples.load(Ordering::Relaxed),
            self.loop_end_samples.load(Ordering::Relaxed),
        )
    }

    /// Set loop region
    pub fn set_loop_region(&self, start_samples: u64, end_samples: u64) {
        assert!(end_samples > start_samples, "Loop end must be after start");
        self.loop_start_samples.store(start_samples, Ordering::Relaxed);
        self.loop_end_samples.store(end_samples, Ordering::Relaxed);
    }

    /// Enable/disable looping
    pub fn set_loop_enabled(&self, enabled: bool) {
        self.loop_enabled.store(enabled, Ordering::Relaxed);
    }
}

impl Default for SharedTransportState {
    fn default() -> Self {
        Self {
            playing: AtomicBool::new(false),
            recording: AtomicBool::new(false),
            paused: AtomicBool::new(false),
            position_samples: AtomicU64::new(0),
            loop_enabled: AtomicBool::new(false),
            loop_start_samples: AtomicU64::new(0),
            loop_end_samples: AtomicU64::new(0),
        }
    }
}

/// Transport controller
/// Manages playback state and position updates
/// Owns the musical time context (tempo, time signature)
pub struct Transport {
    shared_state: Arc<SharedTransportState>,
    tempo: Tempo,
    time_signature: TimeSignature,
    sample_rate: f64,
}

impl Transport {
    /// Create new transport
    pub fn new(sample_rate: f64) -> Self {
        Self {
            shared_state: SharedTransportState::new(),
            tempo: Tempo::default(),
            time_signature: TimeSignature::default(),
            sample_rate,
        }
    }

    /// Create with existing shared state (for audio thread)
    pub fn with_shared_state(
        shared_state: Arc<SharedTransportState>,
        sample_rate: f64,
    ) -> Self {
        Self {
            shared_state,
            tempo: Tempo::default(),
            time_signature: TimeSignature::default(),
            sample_rate,
        }
    }

    /// Get shared state (for passing to audio thread)
    pub fn shared_state(&self) -> Arc<SharedTransportState> {
        Arc::clone(&self.shared_state)
    }

    /// Get current state
    pub fn state(&self) -> TransportState {
        self.shared_state.state()
    }

    /// Get current position
    pub fn position(&self) -> Position {
        let samples = self.shared_state.position_samples();
        Position::from_samples(samples, self.sample_rate, &self.tempo, &self.time_signature)
    }

    /// Set position
    pub fn set_position(&mut self, position: Position) {
        self.shared_state.set_position_samples(position.samples);
    }

    /// Set position from samples
    pub fn set_position_samples(&mut self, samples: u64) {
        self.shared_state.set_position_samples(samples);
    }

    /// Play
    pub fn play(&mut self) {
        self.shared_state.playing.store(true, Ordering::Relaxed);
        self.shared_state.recording.store(false, Ordering::Relaxed);
        self.shared_state.paused.store(false, Ordering::Relaxed);
    }

    /// Stop (reset position to 0)
    pub fn stop(&mut self) {
        self.shared_state.playing.store(false, Ordering::Relaxed);
        self.shared_state.recording.store(false, Ordering::Relaxed);
        self.shared_state.paused.store(false, Ordering::Relaxed);
        self.shared_state.set_position_samples(0);
    }

    /// Pause (keep current position)
    pub fn pause(&mut self) {
        self.shared_state.playing.store(false, Ordering::Relaxed);
        self.shared_state.recording.store(false, Ordering::Relaxed);
        self.shared_state.paused.store(true, Ordering::Relaxed);
    }

    /// Record
    pub fn record(&mut self) {
        self.shared_state.playing.store(true, Ordering::Relaxed);
        self.shared_state.recording.store(true, Ordering::Relaxed);
        self.shared_state.paused.store(false, Ordering::Relaxed);
    }

    /// Toggle play/pause
    pub fn toggle_play(&mut self) {
        if self.state().is_playing() {
            self.pause();
        } else {
            self.play();
        }
    }

    /// Get tempo
    pub fn tempo(&self) -> &Tempo {
        &self.tempo
    }

    /// Set tempo
    pub fn set_tempo(&mut self, tempo: Tempo) {
        self.tempo = tempo;
    }

    /// Get time signature
    pub fn time_signature(&self) -> &TimeSignature {
        &self.time_signature
    }

    /// Set time signature
    pub fn set_time_signature(&mut self, time_signature: TimeSignature) {
        self.time_signature = time_signature;
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    /// Set sample rate (called when audio device changes)
    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
    }

    /// Enable/disable loop
    pub fn set_loop_enabled(&mut self, enabled: bool) {
        self.shared_state.set_loop_enabled(enabled);
    }

    /// Check if loop is enabled
    pub fn is_loop_enabled(&self) -> bool {
        self.shared_state.is_loop_enabled()
    }

    /// Set loop region (musical time)
    pub fn set_loop_region(&mut self, start: Position, end: Position) {
        self.shared_state.set_loop_region(start.samples, end.samples);
    }

    /// Set loop region (samples)
    pub fn set_loop_region_samples(&mut self, start_samples: u64, end_samples: u64) {
        self.shared_state.set_loop_region(start_samples, end_samples);
    }

    /// Get loop region as positions
    pub fn loop_region(&self) -> (Position, Position) {
        let (start_samples, end_samples) = self.shared_state.loop_region();
        (
            Position::from_samples(start_samples, self.sample_rate, &self.tempo, &self.time_signature),
            Position::from_samples(end_samples, self.sample_rate, &self.tempo, &self.time_signature),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_state() {
        let state = TransportState::Playing;
        assert!(state.is_playing());
        assert!(!state.is_recording());
        assert!(!state.is_stopped());

        let state2 = TransportState::Recording;
        assert!(state2.is_playing());
        assert!(state2.is_recording());

        let state3 = TransportState::Stopped;
        assert!(!state3.is_playing());
        assert!(state3.is_stopped());
    }

    #[test]
    fn test_shared_transport_state() {
        let state = SharedTransportState::new();

        assert_eq!(state.state(), TransportState::Stopped);
        assert_eq!(state.position_samples(), 0);

        state.playing.store(true, Ordering::Relaxed);
        assert_eq!(state.state(), TransportState::Playing);

        state.recording.store(true, Ordering::Relaxed);
        assert_eq!(state.state(), TransportState::Recording);
    }

    #[test]
    fn test_position_advance() {
        let state = SharedTransportState::new();

        let new_pos = state.advance_position(1000);
        assert_eq!(new_pos, 1000);
        assert_eq!(state.position_samples(), 1000);

        let new_pos2 = state.advance_position(500);
        assert_eq!(new_pos2, 1500);
    }

    #[test]
    fn test_looping() {
        let state = SharedTransportState::new();

        // Set loop region: 0 to 48000 samples (1 second at 48kHz)
        state.set_loop_region(0, 48000);
        state.set_loop_enabled(true);

        // Advance to near loop end
        state.set_position_samples(47000);
        
        // Advance past loop end
        let new_pos = state.advance_position(2000);
        
        // Should wrap back to start + overflow
        // 47000 + 2000 = 49000
        // 49000 >= 48000, so overflow = 1000
        // new_pos = 0 + 1000 = 1000
        assert_eq!(new_pos, 1000);
    }

    #[test]
    fn test_transport_control() {
        let mut transport = Transport::new(48000.0);

        assert_eq!(transport.state(), TransportState::Stopped);

        transport.play();
        assert_eq!(transport.state(), TransportState::Playing);

        transport.pause();
        assert_eq!(transport.state(), TransportState::Paused);

        transport.record();
        assert_eq!(transport.state(), TransportState::Recording);
        assert!(transport.state().is_recording());

        transport.stop();
        assert_eq!(transport.state(), TransportState::Stopped);
        assert_eq!(transport.position().samples, 0);
    }

    #[test]
    fn test_tempo_time_signature() {
        let mut transport = Transport::new(48000.0);

        assert_eq!(transport.tempo().bpm(), 120.0);
        assert_eq!(transport.time_signature().numerator, 4);

        transport.set_tempo(Tempo::new(140.0));
        assert_eq!(transport.tempo().bpm(), 140.0);

        transport.set_time_signature(TimeSignature::three_four());
        assert_eq!(transport.time_signature().numerator, 3);
    }

    #[test]
    fn test_toggle_play() {
        let mut transport = Transport::new(48000.0);

        assert_eq!(transport.state(), TransportState::Stopped);

        transport.toggle_play();
        assert_eq!(transport.state(), TransportState::Playing);

        transport.toggle_play();
        assert_eq!(transport.state(), TransportState::Paused);

        transport.toggle_play();
        assert_eq!(transport.state(), TransportState::Playing);
    }
}
