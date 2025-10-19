// Effect - Generic effect architecture for DSP processing
//
// This module defines a generic Effect trait and EffectChain for chaining
// multiple audio effects in series. Each effect processes a mono sample
// and can be bypassed individually.
//
// Architecture:
// - Effect trait: Common interface for all effects
// - EffectChain: Pre-allocated chain of effects with bypass support
// - FilterEffect: Wrapper around StateVariableFilter to implement Effect trait
//
// Real-time constraints:
// - No allocations during audio processing
// - Lock-free processing (effects are owned by the chain)
// - Sample-accurate bypass (no clicks)
//
// # Command Pattern Integration (TODO for Delay/Reverb)
//
// When adding new effects (Delay, Reverb), create corresponding commands:
//
// ```rust,ignore
// pub struct AddDelayCommand {
//     delay_params: DelayParams,
//     voice_index: Option<usize>, // None = all voices
// }
//
// pub struct SetReverbCommand {
//     new_params: ReverbParams,
//     old_params: Option<ReverbParams>,
// }
//
// impl UndoableCommand for AddDelayCommand {
//     fn execute(&mut self, state: &mut DawState) -> CommandResult<()> {
//         // Add delay to effect chain
//         // Send Command::AddEffect(delay) to audio thread
//     }
//     // ...
// }
// ```
//
// Note: Filter already has SetFilterCommand in src/command/commands.rs

use super::delay::{Delay, DelayParams};
use super::filter::{FilterParams, StateVariableFilter};
use super::reverb::{Reverb, ReverbParams};

/// Generic effect trait
///
/// All audio effects (filter, delay, reverb, etc.) implement this trait
/// to provide a consistent interface for processing and control.
///
/// # Real-time Safety
/// Implementations must be real-time safe:
/// - No allocations in `process()`
/// - No blocking operations
/// - Deterministic execution time
pub trait Effect: Send {
    /// Process a single mono sample through the effect
    ///
    /// # Arguments
    /// * `input` - Input sample
    ///
    /// # Returns
    /// Processed output sample
    fn process(&mut self, input: f32) -> f32;

    /// Reset effect internal state (delay lines, filter states, etc.)
    ///
    /// Called when:
    /// - Voice is triggered (note on)
    /// - Effect is bypassed then re-enabled
    /// - User manually resets the effect
    fn reset(&mut self);

    /// Check if effect is enabled (bypassed if false)
    fn is_enabled(&self) -> bool;

    /// Enable or disable (bypass) the effect
    ///
    /// When disabled, the effect should pass audio through unchanged.
    fn set_enabled(&mut self, enabled: bool);

    /// Get effect latency in samples
    ///
    /// Used for latency compensation in future phases.
    /// Default: 0 samples (no latency)
    fn latency_samples(&self) -> usize {
        0
    }

    /// Get effect name for UI display
    fn name(&self) -> &str;
}

/// Effect chain - pre-allocated chain of effects
///
/// Processes audio through multiple effects in series.
/// Each effect can be individually bypassed without changing the order.
///
/// # Example
/// ```
/// use mymusic_daw::synth::effect::{EffectChain, FilterEffect};
/// use mymusic_daw::synth::filter::{FilterParams, StateVariableFilter};
///
/// let filter_params = FilterParams::default();
/// let filter = StateVariableFilter::new(filter_params, 44100.0);
/// let filter_effect = FilterEffect::new(filter);
///
/// let mut chain = EffectChain::new();
/// chain.add_effect(Box::new(filter_effect));
///
/// // Process audio
/// let output = chain.process(0.5);
/// ```
pub struct EffectChain {
    /// Pre-allocated effects (max capacity determined at creation)
    effects: Vec<Box<dyn Effect>>,
}

impl EffectChain {
    /// Create an empty effect chain
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
        }
    }

    /// Create an effect chain with pre-allocated capacity
    ///
    /// # Arguments
    /// * `capacity` - Maximum number of effects (pre-allocates memory)
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            effects: Vec::with_capacity(capacity),
        }
    }

    /// Add an effect to the end of the chain
    ///
    /// # Arguments
    /// * `effect` - Boxed effect to add
    ///
    /// # Note
    /// This may allocate if capacity is exceeded. For RT-safe usage,
    /// use `with_capacity()` to pre-allocate sufficient space.
    pub fn add_effect(&mut self, effect: Box<dyn Effect>) {
        self.effects.push(effect);
    }

    /// Remove an effect by index
    ///
    /// # Arguments
    /// * `index` - Index of effect to remove
    ///
    /// # Returns
    /// The removed effect, or None if index is out of bounds
    pub fn remove_effect(&mut self, index: usize) -> Option<Box<dyn Effect>> {
        if index < self.effects.len() {
            Some(self.effects.remove(index))
        } else {
            None
        }
    }

    /// Get number of effects in the chain
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    /// Check if chain is empty
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// Get mutable reference to effect by index
    ///
    /// Used for parameter changes from UI/commands.
    pub fn get_effect_mut(&mut self, index: usize) -> Option<&mut Box<dyn Effect>> {
        self.effects.get_mut(index)
    }

    /// Process a sample through all effects in the chain
    ///
    /// Effects are processed in order. Disabled effects are bypassed.
    ///
    /// # Arguments
    /// * `input` - Input sample
    ///
    /// # Returns
    /// Output sample after all effects
    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let mut sample = input;
        for effect in &mut self.effects {
            if effect.is_enabled() {
                sample = effect.process(sample);
            }
            // If disabled, sample passes through unchanged
        }
        sample
    }

    /// Reset all effects in the chain
    ///
    /// Clears delay lines, filter states, etc. for all effects.
    pub fn reset(&mut self) {
        for effect in &mut self.effects {
            effect.reset();
        }
    }

    /// Get total latency of the chain in samples
    ///
    /// Sums latency of all enabled effects.
    pub fn total_latency_samples(&self) -> usize {
        self.effects
            .iter()
            .filter(|e| e.is_enabled())
            .map(|e| e.latency_samples())
            .sum()
    }

    /// Clear all effects from the chain
    pub fn clear(&mut self) {
        self.effects.clear();
    }
}

impl Default for EffectChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapper around StateVariableFilter to implement Effect trait
///
/// This allows the existing filter to be used in the generic effect chain.
///
/// # Note
/// This wrapper uses the standard `process()` method. For modulated cutoff
/// (via modulation matrix), the filter should be accessed directly or
/// we need a different approach.
pub struct FilterEffect {
    filter: StateVariableFilter,
}

impl FilterEffect {
    /// Create a new filter effect
    pub fn new(filter: StateVariableFilter) -> Self {
        Self { filter }
    }

    /// Get filter parameters
    pub fn params(&self) -> FilterParams {
        self.filter.params()
    }

    /// Set filter parameters
    pub fn set_params(&mut self, params: FilterParams) {
        self.filter.set_params(params);
    }

    /// Get mutable reference to underlying filter
    ///
    /// Used when modulated processing is needed (e.g., LFO → cutoff)
    pub fn filter_mut(&mut self) -> &mut StateVariableFilter {
        &mut self.filter
    }

    /// Get reference to underlying filter
    pub fn filter(&self) -> &StateVariableFilter {
        &self.filter
    }
}

impl Effect for FilterEffect {
    fn process(&mut self, input: f32) -> f32 {
        self.filter.process(input)
    }

    fn reset(&mut self) {
        self.filter.reset();
    }

    fn is_enabled(&self) -> bool {
        self.filter.params().enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        let mut params = self.filter.params();
        params.enabled = enabled;
        self.filter.set_params(params);
    }

    fn name(&self) -> &str {
        "Filter"
    }
}

/// Wrapper around Delay to implement Effect trait
///
/// This allows the delay effect to be used in the generic effect chain.
pub struct DelayEffect {
    delay: Delay,
}

impl DelayEffect {
    /// Create a new delay effect
    ///
    /// # Arguments
    /// * `delay` - Delay instance
    pub fn new(delay: Delay) -> Self {
        Self { delay }
    }

    /// Create a new delay effect with parameters
    ///
    /// # Arguments
    /// * `params` - Delay parameters
    /// * `sample_rate` - Sample rate in Hz
    /// * `max_time_ms` - Maximum delay time in milliseconds
    pub fn with_params(params: DelayParams, sample_rate: f32, max_time_ms: f32) -> Self {
        Self {
            delay: Delay::new(params, sample_rate, max_time_ms),
        }
    }

    /// Get delay parameters
    pub fn params(&self) -> DelayParams {
        self.delay.params()
    }

    /// Set delay parameters
    pub fn set_params(&mut self, params: DelayParams) {
        self.delay.set_params(params);
    }

    /// Get mutable reference to underlying delay
    pub fn delay_mut(&mut self) -> &mut Delay {
        &mut self.delay
    }

    /// Get reference to underlying delay
    pub fn delay(&self) -> &Delay {
        &self.delay
    }
}

impl Effect for DelayEffect {
    fn process(&mut self, input: f32) -> f32 {
        self.delay.process(input)
    }

    fn reset(&mut self) {
        self.delay.reset();
    }

    fn is_enabled(&self) -> bool {
        self.delay.params().enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        let mut params = self.delay.params();
        params.enabled = enabled;
        self.delay.set_params(params);
    }

    fn latency_samples(&self) -> usize {
        self.delay.latency_samples()
    }

    fn name(&self) -> &str {
        "Delay"
    }
}

/// Wrapper around Reverb to implement Effect trait
///
/// This allows the reverb effect to be used in the generic effect chain.
pub struct ReverbEffect {
    reverb: Reverb,
}

impl ReverbEffect {
    /// Create a new reverb effect
    ///
    /// # Arguments
    /// * `reverb` - Reverb instance
    pub fn new(reverb: Reverb) -> Self {
        Self { reverb }
    }

    /// Create a new reverb effect with parameters
    ///
    /// # Arguments
    /// * `params` - Reverb parameters
    /// * `sample_rate` - Sample rate in Hz
    pub fn with_params(params: ReverbParams, sample_rate: f32) -> Self {
        Self {
            reverb: Reverb::new(params, sample_rate),
        }
    }

    /// Get reverb parameters
    pub fn params(&self) -> ReverbParams {
        self.reverb.params()
    }

    /// Set reverb parameters
    pub fn set_params(&mut self, params: ReverbParams) {
        self.reverb.set_params(params);
    }

    /// Get mutable reference to underlying reverb
    pub fn reverb_mut(&mut self) -> &mut Reverb {
        &mut self.reverb
    }

    /// Get reference to underlying reverb
    pub fn reverb(&self) -> &Reverb {
        &self.reverb
    }
}

impl Effect for ReverbEffect {
    fn process(&mut self, input: f32) -> f32 {
        self.reverb.process(input)
    }

    fn reset(&mut self) {
        self.reverb.reset();
    }

    fn is_enabled(&self) -> bool {
        self.reverb.params().enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        let mut params = self.reverb.params();
        params.enabled = enabled;
        self.reverb.set_params(params);
    }

    fn latency_samples(&self) -> usize {
        0 // Reverb has negligible latency
    }

    fn name(&self) -> &str {
        "Reverb"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::filter::{FilterParams, FilterType};

    #[test]
    fn test_effect_chain_creation() {
        let chain = EffectChain::new();
        assert_eq!(chain.len(), 0);
        assert!(chain.is_empty());
    }

    #[test]
    fn test_effect_chain_with_capacity() {
        let chain = EffectChain::with_capacity(4);
        assert_eq!(chain.len(), 0);
        assert!(chain.is_empty());
        // Capacity is pre-allocated but not visible in API
    }

    #[test]
    fn test_add_effect() {
        let mut chain = EffectChain::new();

        let filter_params = FilterParams::default();
        let filter = StateVariableFilter::new(filter_params, 44100.0);
        let filter_effect = FilterEffect::new(filter);

        chain.add_effect(Box::new(filter_effect));

        assert_eq!(chain.len(), 1);
        assert!(!chain.is_empty());
    }

    #[test]
    fn test_remove_effect() {
        let mut chain = EffectChain::new();

        let filter_params = FilterParams::default();
        let filter = StateVariableFilter::new(filter_params, 44100.0);
        let filter_effect = FilterEffect::new(filter);

        chain.add_effect(Box::new(filter_effect));
        assert_eq!(chain.len(), 1);

        let removed = chain.remove_effect(0);
        assert!(removed.is_some());
        assert_eq!(chain.len(), 0);
    }

    #[test]
    fn test_remove_effect_out_of_bounds() {
        let mut chain = EffectChain::new();
        let removed = chain.remove_effect(0);
        assert!(removed.is_none());
    }

    #[test]
    fn test_process_empty_chain() {
        let mut chain = EffectChain::new();

        let input = 0.5;
        let output = chain.process(input);

        // Empty chain should pass audio through unchanged
        assert_eq!(output, input);
    }

    #[test]
    fn test_process_with_filter() {
        let mut chain = EffectChain::new();

        let filter_params = FilterParams {
            cutoff: 1000.0,
            resonance: 0.707,
            filter_type: FilterType::LowPass,
            enabled: true,
        };

        let filter = StateVariableFilter::new(filter_params, 44100.0);
        let filter_effect = FilterEffect::new(filter);

        chain.add_effect(Box::new(filter_effect));

        // Process many samples to settle filter
        let mut last_output = 0.0;
        for _ in 0..1000 {
            last_output = chain.process(0.5);
        }

        // Filter should be processing (output may differ from input)
        assert!(last_output.is_finite());
    }

    #[test]
    fn test_bypass_effect() {
        let mut chain = EffectChain::new();

        let mut filter_params = FilterParams {
            cutoff: 100.0, // Very low cutoff to clearly affect signal
            resonance: 0.707,
            filter_type: FilterType::LowPass,
            enabled: false, // Bypassed
        };

        let filter = StateVariableFilter::new(filter_params, 44100.0);
        let filter_effect = FilterEffect::new(filter);

        chain.add_effect(Box::new(filter_effect));

        let input = 0.5;
        let output = chain.process(input);

        // Bypassed filter should pass audio through unchanged
        assert_eq!(output, input);

        // Enable filter
        if let Some(effect) = chain.get_effect_mut(0) {
            effect.set_enabled(true);
        }

        // Now filter should process
        let output_enabled = chain.process(input);
        // Output may differ from input when filter is active
        assert!(output_enabled.is_finite());
    }

    #[test]
    fn test_chain_multiple_effects() {
        let mut chain = EffectChain::with_capacity(2);

        // Add first filter (low-pass)
        let filter1_params = FilterParams {
            cutoff: 2000.0,
            resonance: 1.0,
            filter_type: FilterType::LowPass,
            enabled: true,
        };
        let filter1 = StateVariableFilter::new(filter1_params, 44100.0);
        chain.add_effect(Box::new(FilterEffect::new(filter1)));

        // Add second filter (high-pass)
        let filter2_params = FilterParams {
            cutoff: 500.0,
            resonance: 1.0,
            filter_type: FilterType::HighPass,
            enabled: true,
        };
        let filter2 = StateVariableFilter::new(filter2_params, 44100.0);
        chain.add_effect(Box::new(FilterEffect::new(filter2)));

        assert_eq!(chain.len(), 2);

        // Process samples
        for _ in 0..1000 {
            let output = chain.process(0.5);
            assert!(output.is_finite());
        }
    }

    #[test]
    fn test_reset_chain() {
        let mut chain = EffectChain::new();

        let filter_params = FilterParams::default();
        let filter = StateVariableFilter::new(filter_params, 44100.0);
        let filter_effect = FilterEffect::new(filter);

        chain.add_effect(Box::new(filter_effect));

        // Process some samples to build up state
        for _ in 0..100 {
            chain.process(1.0);
        }

        // Reset should clear state
        chain.reset();

        // Processing after reset should work
        let output = chain.process(0.5);
        assert!(output.is_finite());
    }

    #[test]
    fn test_latency_calculation() {
        let chain = EffectChain::new();

        // Empty chain has zero latency
        assert_eq!(chain.total_latency_samples(), 0);

        // TODO: Add test with effects that have latency (delay, reverb)
        // For now, filter has 0 latency
    }

    #[test]
    fn test_clear_chain() {
        let mut chain = EffectChain::new();

        let filter_params = FilterParams::default();
        let filter = StateVariableFilter::new(filter_params, 44100.0);
        chain.add_effect(Box::new(FilterEffect::new(filter)));

        assert_eq!(chain.len(), 1);

        chain.clear();

        assert_eq!(chain.len(), 0);
        assert!(chain.is_empty());
    }

    #[test]
    fn test_filter_effect_wrapper() {
        let filter_params = FilterParams {
            cutoff: 1000.0,
            resonance: 2.0,
            filter_type: FilterType::LowPass,
            enabled: true,
        };

        let filter = StateVariableFilter::new(filter_params, 44100.0);
        let mut filter_effect = FilterEffect::new(filter);

        // Test Effect trait methods
        assert_eq!(filter_effect.name(), "Filter");
        assert!(filter_effect.is_enabled());
        assert_eq!(filter_effect.latency_samples(), 0);

        // Test processing
        let output = filter_effect.process(0.5);
        assert!(output.is_finite());

        // Test disable
        filter_effect.set_enabled(false);
        assert!(!filter_effect.is_enabled());

        let output_bypassed = filter_effect.process(0.5);
        assert_eq!(output_bypassed, 0.5); // Bypassed

        // Test reset
        filter_effect.set_enabled(true);
        for _ in 0..100 {
            filter_effect.process(1.0);
        }
        filter_effect.reset();
        let output_after_reset = filter_effect.process(0.5);
        assert!(output_after_reset.is_finite());
    }

    #[test]
    fn test_filter_effect_params() {
        let filter_params = FilterParams::default();
        let filter = StateVariableFilter::new(filter_params, 44100.0);
        let mut filter_effect = FilterEffect::new(filter);

        // Get params
        let params = filter_effect.params();
        assert_eq!(params.cutoff, 1000.0);

        // Set params
        let new_params = FilterParams {
            cutoff: 2000.0,
            resonance: 5.0,
            filter_type: FilterType::HighPass,
            enabled: true,
        };
        filter_effect.set_params(new_params);

        let updated_params = filter_effect.params();
        assert_eq!(updated_params.cutoff, 2000.0);
        assert_eq!(updated_params.resonance, 5.0);
        assert_eq!(updated_params.filter_type, FilterType::HighPass);
    }

    #[test]
    fn test_get_effect_mut() {
        let mut chain = EffectChain::new();

        let filter_params = FilterParams::default();
        let filter = StateVariableFilter::new(filter_params, 44100.0);
        chain.add_effect(Box::new(FilterEffect::new(filter)));

        // Get mutable reference and modify
        if let Some(effect) = chain.get_effect_mut(0) {
            effect.set_enabled(false);
        }

        // Verify change
        let input = 0.5;
        let output = chain.process(input);
        assert_eq!(output, input); // Bypassed
    }
}
