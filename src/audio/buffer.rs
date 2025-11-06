// Gestion des buffers audio

/// Audio buffer for storing audio samples
pub struct AudioBuffer {
    data: Vec<f32>,
}

impl AudioBuffer {
    /// Create a new audio buffer with the given size
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0.0; size],
        }
    }

    /// Clear the buffer (set all samples to 0.0)
    pub fn clear(&mut self) {
        self.data.fill(0.0);
    }

    /// Get immutable reference to buffer data
    pub fn data(&self) -> &[f32] {
        &self.data
    }

    /// Get mutable reference to buffer data
    pub fn data_mut(&mut self) -> &mut [f32] {
        &mut self.data
    }

    /// Get the size of the buffer
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Resize the buffer
    pub fn resize(&mut self, new_size: usize) {
        self.data.resize(new_size, 0.0);
    }
}
