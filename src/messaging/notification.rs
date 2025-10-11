// System de notifications pour la gestion d'erreurs UI

use std::time::{SystemTime, UNIX_EPOCH};

/// Niveau de sévérité d'une notification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
}

/// Catégorie de notification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationCategory {
    Midi,
    Audio,
    Cpu,
    Generic,
}

/// Notification avec timestamp et métadonnées
#[derive(Debug, Clone)]
pub struct Notification {
    pub level: NotificationLevel,
    pub category: NotificationCategory,
    pub message: String,
    pub timestamp: u64, // Unix timestamp en millisecondes
}

impl Notification {
    /// Crée une nouvelle notification avec le timestamp actuel
    pub fn new(level: NotificationLevel, category: NotificationCategory, message: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            level,
            category,
            message,
            timestamp,
        }
    }

    /// Helper pour créer une notification Info
    pub fn info(category: NotificationCategory, message: String) -> Self {
        Self::new(NotificationLevel::Info, category, message)
    }

    /// Helper pour créer une notification Warning
    pub fn warning(category: NotificationCategory, message: String) -> Self {
        Self::new(NotificationLevel::Warning, category, message)
    }

    /// Helper pour créer une notification Error
    pub fn error(category: NotificationCategory, message: String) -> Self {
        Self::new(NotificationLevel::Error, category, message)
    }

    /// Vérifie si la notification est plus récente que N millisecondes
    pub fn is_recent(&self, max_age_ms: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        now.saturating_sub(self.timestamp) < max_age_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_creation() {
        let notif = Notification::error(
            NotificationCategory::Midi,
            "Test error".to_string(),
        );

        assert_eq!(notif.level, NotificationLevel::Error);
        assert_eq!(notif.category, NotificationCategory::Midi);
        assert_eq!(notif.message, "Test error");
        assert!(notif.timestamp > 0);
    }

    #[test]
    fn test_notification_helpers() {
        let info = Notification::info(NotificationCategory::Audio, "Info".to_string());
        let warning = Notification::warning(NotificationCategory::Cpu, "Warning".to_string());
        let error = Notification::error(NotificationCategory::Generic, "Error".to_string());

        assert_eq!(info.level, NotificationLevel::Info);
        assert_eq!(warning.level, NotificationLevel::Warning);
        assert_eq!(error.level, NotificationLevel::Error);
    }

    #[test]
    fn test_notification_is_recent() {
        let notif = Notification::info(NotificationCategory::Generic, "Test".to_string());

        // Should be recent (within 1000ms)
        assert!(notif.is_recent(1000));

        // Should be recent (within 10s)
        assert!(notif.is_recent(10_000));
    }
}
