// Buffer Pool - Zero-allocation audio buffer management
//
// This module provides pre-allocated buffer pools to avoid allocations
// in the audio processing callback (real-time safe).

/// Buffer pool for f32 audio samples
///
/// Pre-allocates buffers to avoid allocations in process() callbacks.
/// Buffers are reused across process() calls for better cache locality.
pub struct AudioBufferPool {
    /// Input channel buffers
    input_buffers: Vec<Vec<f32>>,
    /// Output channel buffers
    output_buffers: Vec<Vec<f32>>,
    /// Channel pointer arrays (for CLAP FFI)
    input_ptrs: Vec<*mut f32>,
    output_ptrs: Vec<*mut f32>,
    /// Maximum buffer size
    max_buffer_size: usize,
    /// Number of input channels
    input_channels: usize,
    /// Number of output channels
    output_channels: usize,
}

impl AudioBufferPool {
    /// Create a new buffer pool
    ///
    /// # Arguments
    /// * `input_channels` - Number of input channels (typically 0-2)
    /// * `output_channels` - Number of output channels (typically 2 for stereo)
    /// * `max_buffer_size` - Maximum buffer size (e.g., 8192 samples)
    pub fn new(input_channels: usize, output_channels: usize, max_buffer_size: usize) -> Self {
        // Pre-allocate input buffers
        let mut input_buffers = Vec::with_capacity(input_channels);
        for _ in 0..input_channels {
            input_buffers.push(vec![0.0f32; max_buffer_size]);
        }

        // Pre-allocate output buffers
        let mut output_buffers = Vec::with_capacity(output_channels);
        for _ in 0..output_channels {
            output_buffers.push(vec![0.0f32; max_buffer_size]);
        }

        Self {
            input_buffers,
            output_buffers,
            input_ptrs: Vec::with_capacity(input_channels),
            output_ptrs: Vec::with_capacity(output_channels),
            max_buffer_size,
            input_channels,
            output_channels,
        }
    }

    /// Prepare buffers for a process() call
    ///
    /// Returns (input_ptrs, output_ptrs) ready for FFI
    ///
    /// # Safety
    /// Returned pointers are valid only until next call to prepare() or drop
    pub fn prepare(&mut self, buffer_size: usize) -> (&[*mut f32], &mut [*mut f32]) {
        assert!(
            buffer_size <= self.max_buffer_size,
            "Buffer size {} exceeds max {}",
            buffer_size,
            self.max_buffer_size
        );

        // Clear input ptrs
        self.input_ptrs.clear();

        // Get input buffer pointers
        for buffer in &mut self.input_buffers {
            self.input_ptrs.push(buffer.as_mut_ptr());
        }

        // Clear output ptrs
        self.output_ptrs.clear();

        // Get output buffer pointers
        for buffer in &mut self.output_buffers {
            // Zero output buffers
            for sample in &mut buffer[..buffer_size] {
                *sample = 0.0;
            }
            self.output_ptrs.push(buffer.as_mut_ptr());
        }

        (&self.input_ptrs, &mut self.output_ptrs)
    }

    /// Copy input data into pool buffers
    pub fn copy_input(&mut self, channel: usize, data: &[f32]) {
        if channel < self.input_channels {
            let len = data.len().min(self.max_buffer_size);
            self.input_buffers[channel][..len].copy_from_slice(&data[..len]);
        }
    }

    /// Copy output data from pool buffers
    pub fn copy_output(&self, channel: usize, data: &mut [f32]) {
        if channel < self.output_channels {
            let len = data.len().min(self.max_buffer_size);
            data[..len].copy_from_slice(&self.output_buffers[channel][..len]);
        }
    }

    /// Get output buffer slice
    pub fn output_buffer(&self, channel: usize, size: usize) -> &[f32] {
        if channel < self.output_channels {
            &self.output_buffers[channel][..size.min(self.max_buffer_size)]
        } else {
            &[]
        }
    }

    /// Get mutable input buffer slice
    pub fn input_buffer_mut(&mut self, channel: usize, size: usize) -> &mut [f32] {
        if channel < self.input_channels {
            &mut self.input_buffers[channel][..size.min(self.max_buffer_size)]
        } else {
            &mut []
        }
    }

    /// Resize pool (reallocates buffers)
    ///
    /// Should only be called when audio is not processing
    pub fn resize(
        &mut self,
        input_channels: usize,
        output_channels: usize,
        max_buffer_size: usize,
    ) {
        *self = Self::new(input_channels, output_channels, max_buffer_size);
    }

    /// Get max buffer size
    pub fn max_buffer_size(&self) -> usize {
        self.max_buffer_size
    }

    /// Get input channel count
    pub fn input_channels(&self) -> usize {
        self.input_channels
    }

    /// Get output channel count
    pub fn output_channels(&self) -> usize {
        self.output_channels
    }
}

// Safety: AudioBufferPool can be sent between threads
// (but should not be used concurrently without synchronization)
unsafe impl Send for AudioBufferPool {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_pool_creation() {
        let pool = AudioBufferPool::new(2, 2, 1024);
        assert_eq!(pool.input_channels(), 2);
        assert_eq!(pool.output_channels(), 2);
        assert_eq!(pool.max_buffer_size(), 1024);
    }

    #[test]
    fn test_buffer_pool_prepare() {
        let mut pool = AudioBufferPool::new(2, 2, 1024);
        let (input_ptrs, output_ptrs) = pool.prepare(512);

        assert_eq!(input_ptrs.len(), 2);
        assert_eq!(output_ptrs.len(), 2);
    }

    #[test]
    fn test_buffer_pool_copy() {
        let mut pool = AudioBufferPool::new(1, 1, 1024);

        // Copy input
        let input_data = vec![1.0f32; 512];
        pool.copy_input(0, &input_data);

        // Verify input was copied
        let input_buffer = pool.input_buffer_mut(0, 512);
        assert_eq!(input_buffer[0], 1.0);

        // Prepare and check output (should be zero)
        pool.prepare(512);
        let output_buffer = pool.output_buffer(0, 512);
        assert_eq!(output_buffer[0], 0.0);
    }

    #[test]
    #[should_panic]
    fn test_buffer_pool_size_limit() {
        let mut pool = AudioBufferPool::new(2, 2, 1024);
        // This should panic (exceeds max_buffer_size)
        pool.prepare(2048);
    }

    #[test]
    fn test_buffer_pool_resize() {
        let mut pool = AudioBufferPool::new(2, 2, 1024);
        pool.resize(4, 4, 2048);

        assert_eq!(pool.input_channels(), 4);
        assert_eq!(pool.output_channels(), 4);
        assert_eq!(pool.max_buffer_size(), 2048);
    }
}
