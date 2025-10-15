// Modulation Matrix (MVP) - lock-free, preallocated, callback-safe
//
// This module provides a small, fixed-size modulation matrix that can be
// evaluated inside the audio callback without allocations or blocking.
// Sources: LFO(0), Velocity, Aftertouch
// Destinations: OscillatorPitch(0), Amplitude

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModSource {
    Lfo(usize),
    Velocity,
    Aftertouch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModDestination {
    /// Pitch of oscillator index (0 for now)
    OscillatorPitch(usize),
    /// Output amplitude (multiplier)
    Amplitude,
}

#[derive(Debug, Clone, Copy)]
pub struct ModRouting {
    pub source: ModSource,
    pub destination: ModDestination,
    /// Amount in [-1.0, 1.0]. Interpretation depends on destination:
    /// - Pitch: amount in semitones (multiplied by source value [-1..1])
    /// - Amplitude: amount as multiplier delta (added to 1.0, result clamped >= 0)
    pub amount: f32,
    pub enabled: bool,
}

impl ModRouting {
    pub const fn disabled() -> Self {
        Self {
            source: ModSource::Velocity,
            destination: ModDestination::Amplitude,
            amount: 0.0,
            enabled: false,
        }
    }
}

pub const MAX_ROUTINGS: usize = 8;

#[derive(Debug, Clone, Copy)]
pub struct ModulationMatrix {
    routings: [ModRouting; MAX_ROUTINGS],
}

impl ModulationMatrix {
    pub fn new_empty() -> Self {
        Self { routings: [ModRouting::disabled(); MAX_ROUTINGS] }
    }

    pub fn set_routing(&mut self, index: usize, routing: ModRouting) {
        if index < MAX_ROUTINGS {
            self.routings[index] = routing;
        }
    }

    pub fn clear_routing(&mut self, index: usize) {
        if index < MAX_ROUTINGS {
            self.routings[index] = ModRouting::disabled();
        }
    }

    pub fn routings(&self) -> &[ModRouting; MAX_ROUTINGS] {
        &self.routings
    }

    /// Quick check if any pitch routing exists (for optional legacy LFO behavior switching)
    pub fn has_pitch_routing(&self) -> bool {
        self.routings.iter().any(|r| r.enabled && matches!(r.destination, ModDestination::OscillatorPitch(_)))
    }

    /// Apply the matrix for a single voice sample
    ///
    /// - `velocity`: 0..1
    /// - `aftertouch`: 0..1 (channel pressure)
    /// - `lfo_values`: current LFO outputs; for MVP, [lfo0]
    /// Returns deltas to apply: pitch in semitones, amplitude multiplier (>=0)
    pub fn apply(&self, velocity: f32, aftertouch: f32, lfo_values: &[f32; 1]) -> (f32, f32) {
        let mut pitch_semitones = 0.0f32;
        let mut amp_mult = 1.0f32;

        // Evaluate all enabled routings
        for r in &self.routings {
            if !r.enabled { continue; }

            // Compute source value in [-1, 1] (or [0,1] mapped to [-1,1] where relevant)
            let src = match r.source {
                ModSource::Lfo(0) => lfo_values[0].clamp(-1.0, 1.0),
                ModSource::Lfo(_) => 0.0, // not used yet
                ModSource::Velocity => (velocity * 2.0 - 1.0).clamp(-1.0, 1.0),
                ModSource::Aftertouch => (aftertouch * 2.0 - 1.0).clamp(-1.0, 1.0),
            };

            match r.destination {
                ModDestination::OscillatorPitch(_idx) => {
                    // Semitone delta = amount * src
                    pitch_semitones += r.amount * src;
                }
                ModDestination::Amplitude => {
                    // Amplitude multiplier = 1.0 + amount * src
                    amp_mult += r.amount * src;
                }
            }
        }

        // Clamp amplitude multiplier to a sane range (non-negative, modest upper bound)
        let amp_mult = amp_mult.clamp(0.0, 2.0);
        (pitch_semitones, amp_mult)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_matrix() {
        let m = ModulationMatrix::new_empty();
        let (p, a) = m.apply(0.8, 0.2, &[0.0]);
        assert_eq!(p, 0.0);
        assert!((a - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_lfo_to_pitch() {
        let mut m = ModulationMatrix::new_empty();
        m.set_routing(0, ModRouting { source: ModSource::Lfo(0), destination: ModDestination::OscillatorPitch(0), amount: 2.0, enabled: true });
        // LFO value +1 → +2 semitones
        let (p, _a) = m.apply(0.5, 0.5, &[1.0]);
        assert!((p - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_velocity_to_amp() {
        let mut m = ModulationMatrix::new_empty();
        m.set_routing(0, ModRouting { source: ModSource::Velocity, destination: ModDestination::Amplitude, amount: 0.5, enabled: true });
        // velocity 1.0 → src = +1.0 → amp = 1 + 0.5*1 = 1.5
        let (_p, a) = m.apply(1.0, 0.0, &[0.0]);
        assert!((a - 1.5).abs() < 1e-6);
    }
}

