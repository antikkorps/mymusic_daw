// Polyphony modes - Controls how multiple notes are handled
//
// - Poly: Multiple notes can play simultaneously (default behavior)
// - Mono: Only one note at a time, new notes cut off old ones
// - Legato: Monophonic with legato (no envelope retrigger when sliding between notes)

/// Polyphony mode for the synthesizer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolyMode {
    /// Polyphonic mode - multiple notes can play simultaneously
    Poly,
    /// Monophonic mode - only one note at a time, retriggering envelope
    Mono,
    /// Legato mode - monophonic without envelope retrigger when sliding between notes
    Legato,
}

impl Default for PolyMode {
    fn default() -> Self {
        Self::Poly
    }
}

impl PolyMode {
    /// Check if this mode allows multiple simultaneous notes
    pub fn is_polyphonic(self) -> bool {
        matches!(self, PolyMode::Poly)
    }

    /// Check if this mode retriggers the envelope on new notes
    pub fn should_retrigger_envelope(self) -> bool {
        // Legato does NOT retrigger envelope when a note is already playing
        !matches!(self, PolyMode::Legato)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_poly() {
        assert_eq!(PolyMode::default(), PolyMode::Poly);
    }

    #[test]
    fn test_is_polyphonic() {
        assert!(PolyMode::Poly.is_polyphonic());
        assert!(!PolyMode::Mono.is_polyphonic());
        assert!(!PolyMode::Legato.is_polyphonic());
    }

    #[test]
    fn test_should_retrigger_envelope() {
        assert!(PolyMode::Poly.should_retrigger_envelope());
        assert!(PolyMode::Mono.should_retrigger_envelope());
        assert!(!PolyMode::Legato.should_retrigger_envelope());
    }
}
