// MIDI Connection Manager - Gestion de la reconnexion automatique

use crate::connection::reconnect::ReconnectionStrategy;
use crate::connection::status::{AtomicDeviceStatus, DeviceStatus};
use crate::messaging::channels::CommandProducer;
use crate::messaging::command::Command;
use crate::midi::event::MidiEvent;
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
    _monitor_thread: Option<thread::JoinHandle<()>>,
}

impl MidiConnectionManager {
    pub fn new(command_tx: CommandProducer) -> Self {
        let connection = Arc::new(Mutex::new(None));
        let status = AtomicDeviceStatus::new(DeviceStatus::Disconnected);
        let target_device = Arc::new(Mutex::new(None));
        let command_tx = Arc::new(Mutex::new(command_tx));

        // Créer une instance et lancer le monitoring
        let mut manager = Self {
            connection: connection.clone(),
            status: status.clone(),
            target_device: target_device.clone(),
            command_tx: command_tx.clone(),
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
        );

        manager._monitor_thread = Some(monitor_thread);
        manager
    }

    /// Tente de se connecter au premier device MIDI disponible
    fn try_connect_default(&mut self) {
        let midi_in = match MidirInput::new("MyMusic DAW MIDI Input") {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Failed to initialize MIDI: {}", e);
                self.status.set(DeviceStatus::Error);
                return;
            }
        };

        let ports = midi_in.ports();
        if ports.is_empty() {
            println!("No MIDI devices found");
            self.status.set(DeviceStatus::Disconnected);
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
                return false;
            }
        };

        // Cloner l'Arc pour le callback
        let command_tx_clone = Arc::clone(&self.command_tx);

        // Créer la connexion avec callback
        let connection = midi_in.connect(
            port,
            "mymusic-daw-input",
            move |_timestamp, message, _| {
                if let Some(midi_event) = MidiEvent::from_bytes(message) {
                    let cmd = Command::Midi(midi_event);
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
                true
            }
            Err(e) => {
                eprintln!("Failed to connect to MIDI device: {}", e);
                self.status.set(DeviceStatus::Error);
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
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let mut reconnect_strategy = ReconnectionStrategy::new();

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
                            eprintln!("MIDI: Max reconnection attempts reached, trying default device");

                            // Fallback : essayer le premier device disponible
                            let midi_in = match MidirInput::new("MyMusic DAW MIDI Fallback") {
                                Ok(m) => m,
                                Err(_) => {
                                    thread::sleep(Duration::from_secs(30));
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
                                let cmd_tx_clone = Arc::clone(&command_tx);

                                // Tenter de se connecter
                                let new_connection = midi_in.connect(
                                    port,
                                    "mymusic-daw-reconnect",
                                    move |_timestamp, message, _| {
                                        if let Some(midi_event) = MidiEvent::from_bytes(message) {
                                            let cmd = Command::Midi(midi_event);
                                            if let Ok(mut tx) = cmd_tx_clone.try_lock() {
                                                let _ = ringbuf::traits::Producer::try_push(&mut *tx, cmd);
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
                                        reconnect_strategy.reset();
                                    }
                                    Err(e) => {
                                        eprintln!("MIDI reconnection failed: {}", e);
                                        status.set(DeviceStatus::Error);
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
}
