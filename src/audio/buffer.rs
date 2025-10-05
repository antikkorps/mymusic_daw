// Gestion des buffers audio

pub struct AudioBuffer {
    size: usize,
}

impl AudioBuffer {
    pub fn new(size: usize) -> Self {
        Self { size }
    }
}
