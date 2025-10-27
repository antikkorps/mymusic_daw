// Atomic parameters - Lock-free communication UI ↔ Audio thread
// Uses atomic operations to share parameters between threads without locks

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

/// Thread-safe f32 parameter using atomic operations
/// Converts f32 to u32 bits for atomic storage
#[derive(Clone)]
pub struct AtomicF32 {
    inner: Arc<AtomicU32>,
}

impl AtomicF32 {
    pub fn new(value: f32) -> Self {
        Self {
            inner: Arc::new(AtomicU32::new(value.to_bits())),
        }
    }

    /// Set the value (called from UI thread)
    pub fn set(&self, value: f32) {
        self.inner.store(value.to_bits(), Ordering::Relaxed);
    }

    /// Get the value (called from audio thread)
    pub fn get(&self) -> f32 {
        f32::from_bits(self.inner.load(Ordering::Relaxed))
    }
}

impl Default for AtomicF32 {
    fn default() -> Self {
        Self::new(0.0)
    }
}
