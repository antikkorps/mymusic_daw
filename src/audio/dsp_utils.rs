// Utilitaires DSP - Hygiène audio et smoothing
//
// Ce module contient les fonctions essentielles pour maintenir
// une qualité audio optimale dans le callback temps-réel.

/// Flush denormals to zero (anti-dénormaux)
///
/// Les nombres dénormaux (très proches de 0) peuvent causer des ralentissements CPU
/// importants sur certains processeurs. Cette fonction force les très petites valeurs
/// à zéro pour éviter ce problème.
///
/// Seuil: 1e-15 (largement sous le bruit numérique à 32-bit float)
#[inline]
pub fn flush_denormals_to_zero(x: f32) -> f32 {
    if x.abs() < 1e-15 {
        0.0
    } else {
        x
    }
}

/// Soft clipping avec tanh (saturation douce)
///
/// Limite doucement la sortie audio dans [-1, 1] sans créer de distorsion dure.
/// Utilise tanh qui donne une courbe de saturation naturelle et musicale.
///
/// - Entrée < -1 ou > 1 : saturation douce asymptotique
/// - Entrée proche de 0 : quasi-linéaire (pas de coloration)
#[inline]
pub fn soft_clip(x: f32) -> f32 {
    // tanh(x) compresse naturellement vers [-1, 1]
    // On peut ajuster le gain d'entrée pour contrôler la "dureté"
    x.tanh()
}

/// Hard clipping (alternative simple)
///
/// Clamp strict dans [-1, 1]. Plus simple mais peut créer des harmoniques
/// indésirables sur les signaux forts. Utilisez soft_clip() de préférence.
#[inline]
#[allow(dead_code)]
pub fn hard_clip(x: f32) -> f32 {
    x.clamp(-1.0, 1.0)
}

/// Smoother 1-pole (filtre passe-bas du 1er ordre)
///
/// Smooth les changements brusques de paramètres pour éviter les clics/pops.
/// Implémentation ultra-simple et efficace pour le temps-réel.
///
/// Formule: y[n] = y[n-1] + α * (x[n] - y[n-1])
/// où α contrôle la vitesse de convergence.
pub struct OnePoleSmoother {
    current: f32,
    coefficient: f32,
}

impl OnePoleSmoother {
    /// Crée un nouveau smoother
    ///
    /// # Arguments
    /// * `initial_value` - Valeur de départ
    /// * `time_constant_ms` - Temps pour atteindre ~63% de la cible (en millisecondes)
    /// * `sample_rate` - Sample rate en Hz
    ///
    /// # Exemple
    /// ```
    /// use mymusic_daw::audio::dsp_utils::OnePoleSmoother;
    /// // Smoothing de 10ms à 44.1kHz
    /// let smoother = OnePoleSmoother::new(0.5, 10.0, 44100.0);
    /// ```
    pub fn new(initial_value: f32, time_constant_ms: f32, sample_rate: f32) -> Self {
        // Calcul du coefficient: α = 1 - e^(-1/(τ * sr))
        // Approximation: α ≈ 1 / (τ * sr) pour de petites valeurs
        let time_constant_samples = time_constant_ms * 0.001 * sample_rate;
        let coefficient = 1.0 / time_constant_samples;

        Self {
            current: initial_value,
            coefficient: coefficient.min(1.0), // Clamp pour éviter instabilité
        }
    }

    /// Process un nouveau sample (next target value)
    #[inline]
    pub fn process(&mut self, target: f32) -> f32 {
        self.current += self.coefficient * (target - self.current);

        // Anti-denormals sur le state interne
        self.current = flush_denormals_to_zero(self.current);

        self.current
    }

    /// Reset à une nouvelle valeur (sans smoothing)
    #[inline]
    pub fn reset(&mut self, value: f32) {
        self.current = value;
    }

    /// Obtenir la valeur courante sans la modifier
    #[inline]
    pub fn get(&self) -> f32 {
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flush_denormals() {
        assert_eq!(flush_denormals_to_zero(1e-20), 0.0);
        assert_eq!(flush_denormals_to_zero(0.1), 0.1);
        assert_eq!(flush_denormals_to_zero(-0.1), -0.1);
    }

    #[test]
    fn test_soft_clip() {
        // Dans la plage normale
        assert!((soft_clip(0.0) - 0.0).abs() < 0.001);
        assert!((soft_clip(0.5) - 0.462).abs() < 0.01);

        // Saturation : tanh converge vers ±1.0 asymptotiquement
        assert!(soft_clip(10.0) <= 1.0);
        assert!(soft_clip(10.0) > 0.99); // Très proche de 1
        assert!(soft_clip(-10.0) >= -1.0);
        assert!(soft_clip(-10.0) < -0.99);
    }

    #[test]
    fn test_smoother_convergence() {
        let mut smoother = OnePoleSmoother::new(0.0, 10.0, 44100.0);

        // Converge vers la cible (10ms à 44.1kHz = 441 samples pour 63% de convergence)
        // On simule 100ms (4410 samples) pour atteindre ~99.99% de convergence
        let mut final_value = 0.0;
        for _ in 0..4410 {
            final_value = smoother.process(1.0);
        }

        // Doit être très proche de 1.0
        assert!((final_value - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_smoother_no_overshoot() {
        let mut smoother = OnePoleSmoother::new(0.0, 5.0, 44100.0);

        // Ne doit jamais dépasser la cible
        for _ in 0..100 {
            let value = smoother.process(1.0);
            assert!(value <= 1.0);
            assert!(value >= 0.0);
        }
    }
}
