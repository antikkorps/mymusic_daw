// Status des connexions périphériques

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceStatus {
    Disconnected = 0,
    Connecting = 1,
    Connected = 2,
    Error = 3,
}

impl From<u8> for DeviceStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => DeviceStatus::Disconnected,
            1 => DeviceStatus::Connecting,
            2 => DeviceStatus::Connected,
            3 => DeviceStatus::Error,
            _ => DeviceStatus::Disconnected,
        }
    }
}

/// Atomic wrapper pour partager le status entre threads
#[derive(Clone)]
pub struct AtomicDeviceStatus {
    inner: Arc<AtomicU8>,
}

impl AtomicDeviceStatus {
    pub fn new(status: DeviceStatus) -> Self {
        Self {
            inner: Arc::new(AtomicU8::new(status as u8)),
        }
    }

    pub fn get(&self) -> DeviceStatus {
        DeviceStatus::from(self.inner.load(Ordering::Relaxed))
    }

    pub fn set(&self, status: DeviceStatus) {
        self.inner.store(status as u8, Ordering::Relaxed);
    }
}

impl Default for AtomicDeviceStatus {
    fn default() -> Self {
        Self::new(DeviceStatus::Disconnected)
    }
}
