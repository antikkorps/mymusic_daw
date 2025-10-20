// MIDI Connection Manager - Gestion de la reconnexion automatique

use crate::connection::reconnect::ReconnectionStrategy;
use crate::connection::status::{AtomicDeviceStatus, DeviceStatus};
use crate::messaging::channels::{CommandProducer, NotificationProducer};
use crate::messaging::command::Command;
use crate::messaging::notification::{Notification, NotificationCategory};
use crate::midi::event::{MidiEvent, MidiEventTimed};
use midir::{MidiInput as MidirInput, MidiInputConnection};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

type MidiConnection = Arc<Mutex<Option<MidiInputConnection<()>>>>;

pub struct MidiConnectionManager {
    connection: MidiConnection,
    status: AtomicDeviceStatus,
    target_device: Arc<Mutex<Option<String>>>,
    command_tx: Arc<Mutex<CommandProducer>>,
    notification_tx: Arc<Mutex<NotificationProducer>>,
    _monitor_thread: Option<thread::JoinHandle<()>>,
}

impl MidiConnectionManager {
    pub fn new(
        command_tx: CommandProducer,
        notification_tx: Arc<Mutex<NotificationProducer>>,
    ) -> Self {
        let connection = Arc::new(Mutex::new(None));
        let status = AtomicDeviceStatus::new(DeviceStatus::Disconnected);
        let target_device = Arc::new(Mutex::new(None));
        let command_tx = Arc::new(Mutex::new(command_tx));

        // Check if MIDI is available (WSL-friendly)
        let midi_available = Self::is_midi_available();
        if !midi_available {
            println!("⚠ MIDI not available - running without MIDI support");
            return Self {
                connection,
                status,
                target_device,
                command_tx,
                notification_tx,
                _monitor_thread: None,
            };
        }

        // Créer une instance et lancer le monitoring
        let mut manager = Self {
            connection: connection.clone(),
            status: status.clone(),
            target_device: target_device.clone(),
            command_tx: command_tx.clone(),
            notification_tx: notification_tx.clone(),
            _monitor_thread: None,
        };

        // Tenter la connexion initiale au premier device disponible
        manager.try_connect_default();

        // Lancer le thread de monitoring
        let monitor_thread = Self::spawn_monitor_thread(
            connection,
            status.clone(),
            target_device,
            command_tx,
            notification_tx,
        );

        manager._monitor_thread = Some(monitor_thread);
        manager
    }

    /// Check if MIDI subsystem is available (WSL-friendly)
    fn is_midi_available() -> bool {
        // Try to initialize MIDI input to check availability
        match MidirInput::new("MIDI Availability Check") {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Tente de se connecter au premier device MIDI disponible
    fn try_connect_default(&mut self) {
        let midi_in = match MidirInput::new("MyMusic DAW MIDI Input") {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Failed to initialize MIDI: {}", e);
                self.status.set(DeviceStatus::Error);
                self.send_notification(Notification::error(
                    NotificationCategory::Midi,
                    format!("Failed to initialize MIDI: {}", e),
                ));
                return;
            }
        };

        let ports = midi_in.ports();
        if ports.is_empty() {
            println!("No MIDI devices found");
            self.status.set(DeviceStatus::Disconnected);
            self.send_notification(Notification::warning(
                NotificationCategory::Midi,
                "No MIDI devices found".to_string(),
            ));
            return;
        }

        let port = &ports[0];
        let port_name = midi_in
            .port_name(port)
            .unwrap_or_else(|_| "Unknown".to_string());

        // Stocker le nom du device cible
        if let Ok(mut target) = self.target_device.lock() {
            *target = Some(port_name.clone());
        }

        // Tenter la connexion
        self.try_connect_to_device(&port_name);
    }

    /// Tente de se connecter à un device MIDI spécifique
    pub fn try_connect_to_device(&self, device_name: &str) -> bool {
        self.status.set(DeviceStatus::Connecting);

        let midi_in = match MidirInput::new("MyMusic DAW MIDI Input") {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Failed to initialize MIDI: {}", e);
                self.status.set(DeviceStatus::Error);
                self.send_notification(Notification::error(
                    NotificationCategory::Midi,
                    format!("Failed to initialize MIDI: {}", e),
                ));
                return false;
            }
        };

        // Chercher le port par nom
        let ports = midi_in.ports();
        let port = ports.iter().find(|p| {
            midi_in
                .port_name(p)
                .map(|name| name == device_name)
                .unwrap_or(false)
        });

        let port = match port {
            Some(p) => p,
            None => {
                eprintln!("MIDI device '{}' not found", device_name);
                self.status.set(DeviceStatus::Disconnected);
                self.send_notification(Notification::error(
                    NotificationCategory::Midi,
                    format!("MIDI device '{}' not found", device_name),
                ));
                return false;
            }
        };

        // Cloner l'Arc pour le callback
        let command_tx_clone: Arc<Mutex<CommandProducer>> = Arc::clone(&self.command_tx);

        // Créer la connexion avec callback
        let connection = midi_in.connect(
            port,
            "mymusic-daw-input",
            move |_timestamp, message, _| {
                if let Some(midi_event) = MidiEvent::from_bytes(message) {
                    // Create timed MIDI event
                    // TODO: Calculate precise samples_from_now based on _timestamp
                    let timed_event = MidiEventTimed {
                        event: midi_event,
                        samples_from_now: 0,
                    };
                    let cmd = Command::Midi(timed_event);
                    // Lock et push (non-bloquant grâce à try_lock)
                    if let Ok(mut tx) = command_tx_clone.try_lock() {
                        let _ = ringbuf::traits::Producer::try_push(&mut *tx, cmd);
                    }
                }
            },
            (),
        );

        match connection {
            Ok(conn) => {
                if let Ok(mut c) = self.connection.lock() {
                    *c = Some(conn);
                }
                self.status.set(DeviceStatus::Connected);
                println!("✓ MIDI connected: {}", device_name);
                self.send_notification(Notification::info(
                    NotificationCategory::Midi,
                    format!("MIDI connected: {}", device_name),
                ));
                true
            }
            Err(e) => {
                eprintln!("Failed to connect to MIDI device: {}", e);
                self.status.set(DeviceStatus::Error);
                self.send_notification(Notification::error(
                    NotificationCategory::Midi,
                    format!("Failed to connect to MIDI: {}", e),
                ));
                false
            }
        }
    }

    /// Thread de monitoring qui vérifie l'état de la connexion et tente de se reconnecter
    fn spawn_monitor_thread(
        connection: MidiConnection,
        status: AtomicDeviceStatus,
        target_device: Arc<Mutex<Option<String>>>,
        command_tx: Arc<Mutex<CommandProducer>>,
        notification_tx: Arc<Mutex<NotificationProducer>>,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let mut reconnect_strategy = ReconnectionStrategy::new();
            let mut consecutive_failures = 0;

            // Helper pour envoyer des notifications depuis le thread
            let send_notification = |notif: Notification| {
                if let Ok(mut tx) = notification_tx.try_lock() {
                    let _ = ringbuf::traits::Producer::try_push(&mut *tx, notif);
                }
            };

            loop {
                thread::sleep(Duration::from_secs(2)); // Polling toutes les 2 secondes

                let current_status = status.get();

                match current_status {
                    DeviceStatus::Connected => {
                        // Connexion OK, reset la stratégie
                        reconnect_strategy.reset();

                        // Vérifier si la connexion est toujours active
                        // (le MidiInputConnection ne nous donne pas facilement cette info,
                        // donc on fait confiance au status pour le moment)
                    }
                    DeviceStatus::Disconnected | DeviceStatus::Error => {
                        // Tenter de se reconnecter
                        if !reconnect_strategy.should_retry() {
                            eprintln!(
                                "MIDI: Max reconnection attempts reached, trying default device"
                            );
                            send_notification(Notification::warning(
                                NotificationCategory::Midi,
                                "Max reconnection attempts reached".to_string(),
                            ));

                            // Fallback : essayer le premier device disponible
                            let midi_in = match MidirInput::new("MyMusic DAW MIDI Fallback") {
                                Ok(m) => m,
                                Err(_) => {
                                    consecutive_failures += 1;
                                    // If MIDI has failed many times, wait longer (WSL-friendly)
                                    let wait_time = if consecutive_failures > 5 {
                                        Duration::from_secs(30)
                                    } else {
                                        Duration::from_secs(5)
                                    };
                                    thread::sleep(wait_time);
                                    reconnect_strategy.reset();
                                    continue;
                                }
                            };

                            let ports = midi_in.ports();
                            if !ports.is_empty() {
                                let port = &ports[0];
                                if let Ok(device_name) = midi_in.port_name(port) {
                                    // Changer le device cible vers le premier disponible
                                    if let Ok(mut target) = target_device.lock() {
                                        *target = Some(device_name);
                                    }
                                    println!("MIDI: Falling back to default device");
                                }
                            }

                            thread::sleep(Duration::from_secs(5));
                            reconnect_strategy.reset(); // Reset après fallback
                            continue;
                        }

                        let delay = reconnect_strategy.next_delay();
                        if let Some(d) = delay {
                            println!(
                                "MIDI: Reconnection attempt {} in {:?}",
                                reconnect_strategy.current_attempt(),
                                d
                            );
                            thread::sleep(d);
                        }

                        // Obtenir le nom du device cible
                        let target = target_device.lock().ok().and_then(|t| t.clone());

                        if let Some(device_name) = target {
                            status.set(DeviceStatus::Connecting);

                            // Tenter de se reconnecter
                            let midi_in = match MidirInput::new("MyMusic DAW MIDI Reconnect") {
                                Ok(m) => m,
                                Err(_) => continue,
                            };

                            let ports = midi_in.ports();
                            let port = ports.iter().find(|p| {
                                midi_in
                                    .port_name(p)
                                    .map(|name| name == device_name)
                                    .unwrap_or(false)
                            });

                            if let Some(port) = port {
                                // Cloner l'Arc pour le callback
                                let cmd_tx_clone: Arc<Mutex<CommandProducer>> = Arc::clone(&command_tx);

                                // Tenter de se connecter
                                let new_connection = midi_in.connect(
                                    port,
                                    "mymusic-daw-reconnect",
                                    move |_timestamp, message, _| {
                                        if let Some(midi_event) = MidiEvent::from_bytes(message) {
                                            // Create timed MIDI event
                                            // TODO: Calculate precise samples_from_now based on _timestamp
                                            let timed_event = MidiEventTimed {
                                                event: midi_event,
                                                samples_from_now: 0,
                                            };
                                            let cmd = Command::Midi(timed_event);
                                            if let Ok(mut tx) = cmd_tx_clone.try_lock() {
                                                let _ = ringbuf::traits::Producer::try_push(
                                                    &mut *tx, cmd,
                                                );
                                            }
                                        }
                                    },
                                    (),
                                );

                                match new_connection {
                                    Ok(conn) => {
                                        if let Ok(mut c) = connection.lock() {
                                            *c = Some(conn);
                                        }
                                        status.set(DeviceStatus::Connected);
                                        println!("✓ MIDI reconnected: {}", device_name);
                                        send_notification(Notification::info(
                                            NotificationCategory::Midi,
                                            format!("MIDI reconnected: {}", device_name),
                                        ));
                                        reconnect_strategy.reset();
                                    }
                                    Err(e) => {
                                        eprintln!("MIDI reconnection failed: {}", e);
                                        status.set(DeviceStatus::Error);
                                        send_notification(Notification::error(
                                            NotificationCategory::Midi,
                                            "MIDI reconnection failed".to_string(),
                                        ));
                                    }
                                }
                            } else {
                                // Device pas encore disponible, réessayer plus tard
                                status.set(DeviceStatus::Disconnected);
                            }
                        }
                    }
                    DeviceStatus::Connecting => {
                        // En cours de connexion, attendre
                    }
                }
            }
        })
    }

    /// Change le device MIDI cible
    pub fn set_target_device(&self, device_name: String) {
        if let Ok(mut target) = self.target_device.lock() {
            *target = Some(device_name.clone());
        }

        // Fermer la connexion actuelle
        if let Ok(mut conn) = self.connection.lock() {
            *conn = None;
        }

        // Tenter de se connecter au nouveau device
        self.try_connect_to_device(&device_name);
    }

    /// Retourne le status actuel de la connexion
    pub fn status(&self) -> DeviceStatus {
        self.status.get()
    }

    /// Retourne le device cible actuel
    pub fn target_device(&self) -> Option<String> {
        self.target_device.lock().ok().and_then(|t| t.clone())
    }

    /// Helper pour envoyer une notification
    fn send_notification(&self, notification: Notification) {
        if let Ok(mut tx) = self.notification_tx.try_lock() {
            let _ = ringbuf::traits::Producer::try_push(&mut *tx, notification);
        }
    }
}
