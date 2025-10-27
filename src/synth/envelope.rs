// ADSR Envelope implementation
//
// Classic Attack-Decay-Sustain-Release envelope generator
// Used to shape the amplitude of voices over time

/// ADSR Envelope parameters
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdsrParams {
    /// Attack time in seconds (0.001 to 5.0)
    pub attack: f32,
    /// Decay time in seconds (0.001 to 5.0)
    pub decay: f32,
    /// Sustain level (0.0 to 1.0)
    pub sustain: f32,
    /// Release time in seconds (0.001 to 5.0)
    pub release: f32,
}

impl AdsrParams {
    /// Create ADSR parameters with validation
    pub fn new(attack: f32, decay: f32, sustain: f32, release: f32) -> Self {
        Self {
            attack: attack.clamp(0.001, 5.0),
            decay: decay.clamp(0.001, 5.0),
            sustain: sustain.clamp(0.0, 1.0),
            release: release.clamp(0.001, 5.0),
        }
    }
}

impl Default for AdsrParams {
    fn default() -> Self {
        Self {
            attack: 0.01, // 10ms attack
            decay: 0.1,   // 100ms decay
            sustain: 0.7, // 70% sustain level
            release: 0.2, // 200ms release
        }
    }
}

/// State of the ADSR envelope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnvelopeState {
    /// No note is playing
    Idle,
    /// Attack phase (rising to 1.0)
    Attack,
    /// Decay phase (falling to sustain level)
    Decay,
    /// Sustain phase (holding at sustain level)
    Sustain,
    /// Release phase (falling to 0.0)
    Release,
}

/// ADSR Envelope Generator
///
/// Generates an amplitude envelope with Attack, Decay, Sustain, Release phases.
/// Sample rate must be set before use.
pub struct AdsrEnvelope {
    params: AdsrParams,
    state: EnvelopeState,
    current_value: f32,
    sample_rate: f32,

    // Internal counters (in samples)
    attack_samples: f32,
    decay_samples: f32,
    release_samples: f32,
    current_sample: f32,
}

impl AdsrEnvelope {
    /// Create a new ADSR envelope
    pub fn new(params: AdsrParams, sample_rate: f32) -> Self {
        let mut envelope = Self {
            params,
            state: EnvelopeState::Idle,
            current_value: 0.0,
            sample_rate,
            attack_samples: 0.0,
            decay_samples: 0.0,
            release_samples: 0.0,
            current_sample: 0.0,
        };
        envelope.update_sample_counts();
        envelope
    }

    /// Update internal sample counts when parameters change
    fn update_sample_counts(&mut self) {
        self.attack_samples = self.params.attack * self.sample_rate;
        self.decay_samples = self.params.decay * self.sample_rate;
        self.release_samples = self.params.release * self.sample_rate;
    }

    /// Set new ADSR parameters
    pub fn set_params(&mut self, params: AdsrParams) {
        self.params = params;
        self.update_sample_counts();
    }

    /// Get current parameters
    pub fn params(&self) -> AdsrParams {
        self.params
    }

    /// Trigger note on (start attack phase)
    pub fn note_on(&mut self) {
        self.state = EnvelopeState::Attack;
        self.current_sample = 0.0;
        // If retriggering during release, start from current value for smooth transition
        // Otherwise start from 0
        if !matches!(self.state, EnvelopeState::Release) {
            self.current_value = 0.0;
        }
    }

    /// Trigger note off (start release phase)
    pub fn note_off(&mut self) {
        // Only enter release if we're not already idle
        if !matches!(self.state, EnvelopeState::Idle) {
            self.state = EnvelopeState::Release;
            self.current_sample = 0.0;
        }
    }

    /// Process one sample and return the envelope value
    ///
    /// Returns a value between 0.0 and 1.0 that should be multiplied with the audio signal
    pub fn process(&mut self) -> f32 {
        match self.state {
            EnvelopeState::Idle => {
                self.current_value = 0.0;
            }

            EnvelopeState::Attack => {
                if self.attack_samples > 0.0 {
                    // Linear attack from current_value to 1.0
                    let progress = self.current_sample / self.attack_samples;
                    self.current_value = progress.min(1.0);

                    self.current_sample += 1.0;

                    if self.current_sample >= self.attack_samples {
                        self.state = EnvelopeState::Decay;
                        self.current_sample = 0.0;
                        self.current_value = 1.0;
                    }
                } else {
                    // Instant attack
                    self.current_value = 1.0;
                    self.state = EnvelopeState::Decay;
                    self.current_sample = 0.0;
                }
            }

            EnvelopeState::Decay => {
                if self.decay_samples > 0.0 {
                    // Linear decay from 1.0 to sustain level
                    let progress = self.current_sample / self.decay_samples;
                    self.current_value = 1.0 - progress * (1.0 - self.params.sustain);
                    self.current_value = self.current_value.max(self.params.sustain);

                    self.current_sample += 1.0;

                    if self.current_sample >= self.decay_samples {
                        self.state = EnvelopeState::Sustain;
                        self.current_value = self.params.sustain;
                    }
                } else {
                    // Instant decay
                    self.current_value = self.params.sustain;
                    self.state = EnvelopeState::Sustain;
                }
            }

            EnvelopeState::Sustain => {
                // Hold at sustain level
                self.current_value = self.params.sustain;
            }

            EnvelopeState::Release => {
                if self.release_samples > 0.0 {
                    // Linear release from current value to 0.0
                    let start_value = if self.current_sample == 0.0 {
                        self.current_value
                    } else {
                        // Continue from where we are
                        self.current_value
                    };

                    let progress = self.current_sample / self.release_samples;
                    self.current_value = start_value * (1.0 - progress);
                    self.current_value = self.current_value.max(0.0);

                    self.current_sample += 1.0;

                    if self.current_sample >= self.release_samples {
                        self.state = EnvelopeState::Idle;
                        self.current_value = 0.0;
                    }
                } else {
                    // Instant release
                    self.current_value = 0.0;
                    self.state = EnvelopeState::Idle;
                }
            }
        }

        self.current_value
    }

    /// Check if the envelope is currently active (not idle)
    pub fn is_active(&self) -> bool {
        !matches!(self.state, EnvelopeState::Idle)
    }

    /// Get the current envelope value without processing
    pub fn current_value(&self) -> f32 {
        self.current_value
    }

    /// Reset the envelope to idle state
    pub fn reset(&mut self) {
        self.state = EnvelopeState::Idle;
        self.current_value = 0.0;
        self.current_sample = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SAMPLE_RATE: f32 = 48000.0;

    #[test]
    fn test_adsr_params_default() {
        let params = AdsrParams::default();
        assert_eq!(params.attack, 0.01);
        assert_eq!(params.decay, 0.1);
        assert_eq!(params.sustain, 0.7);
        assert_eq!(params.release, 0.2);
    }

    #[test]
    fn test_adsr_params_clamping() {
        let params = AdsrParams::new(-1.0, 10.0, 1.5, 0.0001);
        assert!(params.attack >= 0.001);
        assert!(params.decay <= 5.0);
        assert!(params.sustain <= 1.0);
        assert!(params.release >= 0.001);
    }

    #[test]
    fn test_envelope_starts_idle() {
        let params = AdsrParams::default();
        let envelope = AdsrEnvelope::new(params, TEST_SAMPLE_RATE);
        assert_eq!(envelope.state, EnvelopeState::Idle);
        assert_eq!(envelope.current_value(), 0.0);
    }

    #[test]
    fn test_attack_phase() {
        let params = AdsrParams::new(0.01, 0.1, 0.7, 0.2);
        let mut envelope = AdsrEnvelope::new(params, TEST_SAMPLE_RATE);

        envelope.note_on();
        assert_eq!(envelope.state, EnvelopeState::Attack);

        // Process through attack (should reach 1.0)
        let attack_samples = (0.01 * TEST_SAMPLE_RATE) as usize;
        for _ in 0..attack_samples {
            envelope.process();
        }

        // Should be in decay phase now
        assert_eq!(envelope.state, EnvelopeState::Decay);
        assert!((envelope.current_value() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_decay_to_sustain() {
        let params = AdsrParams::new(0.001, 0.01, 0.5, 0.1);
        let mut envelope = AdsrEnvelope::new(params, TEST_SAMPLE_RATE);

        envelope.note_on();

        // Skip through attack
        let attack_samples = (0.001 * TEST_SAMPLE_RATE) as usize;
        for _ in 0..attack_samples + 10 {
            envelope.process();
        }

        // Should be in decay
        assert_eq!(envelope.state, EnvelopeState::Decay);

        // Process through decay
        let decay_samples = (0.01 * TEST_SAMPLE_RATE) as usize;
        for _ in 0..decay_samples + 100 {
            envelope.process();
        }

        // Should be in sustain at 0.5 level
        assert_eq!(envelope.state, EnvelopeState::Sustain);
        assert!((envelope.current_value() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_sustain_holds() {
        let params = AdsrParams::new(0.001, 0.001, 0.6, 0.1);
        let mut envelope = AdsrEnvelope::new(params, TEST_SAMPLE_RATE);

        envelope.note_on();

        // Skip to sustain
        for _ in 0..1000 {
            envelope.process();
        }

        assert_eq!(envelope.state, EnvelopeState::Sustain);
        let sustain_value = envelope.current_value();

        // Process many samples, should stay at sustain
        for _ in 0..10000 {
            envelope.process();
        }

        assert_eq!(envelope.state, EnvelopeState::Sustain);
        assert_eq!(envelope.current_value(), sustain_value);
    }

    #[test]
    fn test_release_to_idle() {
        let params = AdsrParams::new(0.001, 0.001, 0.5, 0.01);
        let mut envelope = AdsrEnvelope::new(params, TEST_SAMPLE_RATE);

        envelope.note_on();

        // Skip to sustain
        for _ in 0..1000 {
            envelope.process();
        }

        envelope.note_off();
        assert_eq!(envelope.state, EnvelopeState::Release);

        // Process through release
        let release_samples = (0.01 * TEST_SAMPLE_RATE) as usize;
        for _ in 0..release_samples + 100 {
            envelope.process();
        }

        // Should be idle at 0.0
        assert_eq!(envelope.state, EnvelopeState::Idle);
        assert_eq!(envelope.current_value(), 0.0);
        assert!(!envelope.is_active());
    }

    #[test]
    fn test_note_off_during_attack() {
        let params = AdsrParams::new(0.1, 0.1, 0.5, 0.05);
        let mut envelope = AdsrEnvelope::new(params, TEST_SAMPLE_RATE);

        envelope.note_on();

        // Process a bit of attack
        for _ in 0..100 {
            envelope.process();
        }

        assert_eq!(envelope.state, EnvelopeState::Attack);
        envelope.note_off();

        // Should go to release
        assert_eq!(envelope.state, EnvelopeState::Release);
    }

    #[test]
    fn test_retriggering() {
        let params = AdsrParams::new(0.01, 0.01, 0.5, 0.05);
        let mut envelope = AdsrEnvelope::new(params, TEST_SAMPLE_RATE);

        envelope.note_on();
        for _ in 0..500 {
            envelope.process();
        }

        // Retrigger during sustain
        envelope.note_on();
        assert_eq!(envelope.state, EnvelopeState::Attack);
    }

    #[test]
    fn test_is_active() {
        let params = AdsrParams::default();
        let mut envelope = AdsrEnvelope::new(params, TEST_SAMPLE_RATE);

        assert!(!envelope.is_active());

        envelope.note_on();
        assert!(envelope.is_active());

        envelope.note_off();
        assert!(envelope.is_active()); // Still releasing

        // Process until idle
        for _ in 0..100000 {
            envelope.process();
        }
        assert!(!envelope.is_active());
    }
}
