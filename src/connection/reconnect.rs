// Logique de reconnexion avec backoff exponentiel

use std::time::Duration;

pub struct ReconnectionStrategy {
    max_attempts: u32,
    base_delay_ms: u64,
    max_delay_ms: u64,
    current_attempt: u32,
}

impl ReconnectionStrategy {
    pub fn new() -> Self {
        Self {
            max_attempts: 10, // 10 tentatives max
            base_delay_ms: 1000, // 1 seconde de base
            max_delay_ms: 30000, // 30 secondes max
            current_attempt: 0,
        }
    }

    /// Calcule le délai pour la prochaine tentative (backoff exponentiel)
    pub fn next_delay(&mut self) -> Option<Duration> {
        if self.current_attempt >= self.max_attempts {
            return None; // Plus de tentatives
        }

        // Backoff exponentiel: base * 2^attempt
        let delay_ms = self.base_delay_ms * 2u64.pow(self.current_attempt);
        let delay_ms = delay_ms.min(self.max_delay_ms); // Cap au maximum

        self.current_attempt += 1;

        Some(Duration::from_millis(delay_ms))
    }

    /// Réinitialise le compteur de tentatives (après succès)
    pub fn reset(&mut self) {
        self.current_attempt = 0;
    }

    /// Indique si on doit encore tenter de se reconnecter
    pub fn should_retry(&self) -> bool {
        self.current_attempt < self.max_attempts
    }

    pub fn current_attempt(&self) -> u32 {
        self.current_attempt
    }
}

impl Default for ReconnectionStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff() {
        let mut strategy = ReconnectionStrategy::new();

        // Premier délai: 1s
        assert_eq!(strategy.next_delay(), Some(Duration::from_millis(1000)));

        // Deuxième: 2s
        assert_eq!(strategy.next_delay(), Some(Duration::from_millis(2000)));

        // Troisième: 4s
        assert_eq!(strategy.next_delay(), Some(Duration::from_millis(4000)));

        // Quatrième: 8s
        assert_eq!(strategy.next_delay(), Some(Duration::from_millis(8000)));
    }

    #[test]
    fn test_reset() {
        let mut strategy = ReconnectionStrategy::new();

        strategy.next_delay();
        strategy.next_delay();
        assert_eq!(strategy.current_attempt(), 2);

        strategy.reset();
        assert_eq!(strategy.current_attempt(), 0);
        assert!(strategy.should_retry());
    }

    #[test]
    fn test_max_attempts() {
        let mut strategy = ReconnectionStrategy::new();
        strategy.max_attempts = 3;

        assert!(strategy.next_delay().is_some());
        assert!(strategy.next_delay().is_some());
        assert!(strategy.next_delay().is_some());
        assert!(strategy.next_delay().is_none()); // Plus de tentatives
        assert!(!strategy.should_retry());
    }
}
