// Gestion des buffers audio

pub struct AudioBuffer {
    _size: usize,
}

impl AudioBuffer {
    pub fn new(size: usize) -> Self {
        Self { _size: size }
    }
}
