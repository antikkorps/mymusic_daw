# MyMusic DAW - Tauri Backend

Backend Tauri qui expose le moteur audio Rust au frontend React via IPC.

## 🏗️ Architecture

```
┌─────────────────────────────────────────┐
│        Tauri Application                │
│                                         │
│  ┌─────────────┐      ┌──────────────┐ │
│  │   lib.rs    │──────│   main.rs    │ │
│  │  (Commands) │      │   (Init)     │ │
│  └─────────────┘      └──────────────┘ │
│         │                     │         │
│         │                     ▼         │
│         │            ┌──────────────┐  │
│         │            │  DawState    │  │
│         │            │  (Managed)   │  │
│         │            └──────────────┘  │
│         │                     │         │
└─────────┼─────────────────────┼─────────┘
          │                     │
          ▼                     ▼
┌─────────────────────────────────────────┐
│      MyMusic DAW Engine                 │
│  (from parent crate)                    │
│                                         │
│  • AudioEngine                          │
│  • CommandProducer                      │
│  • AtomicF32 (volume)                   │
│  • MidiConnectionManager                │
│  • CpuMonitor                           │
└─────────────────────────────────────────┘
```

## 📁 Fichiers

### `src/lib.rs`

Expose les commandes Tauri pour contrôler le moteur audio :

```rust
#[tauri::command]
pub fn set_volume(volume: f32, state: State<DawState>) -> Result<(), String>

#[tauri::command]
pub fn play_note(note: u8, velocity: u8, state: State<DawState>) -> Result<(), String>

#[tauri::command]
pub fn stop_note(note: u8, state: State<DawState>) -> Result<(), String>

#[tauri::command]
pub fn get_volume(state: State<DawState>) -> Result<f32, String>

#[tauri::command]
pub fn get_engine_status() -> Result<serde_json::Value, String>
```

### `src/main.rs`

Point d'entrée de l'application Tauri :

1. Initialise le moteur audio
2. Crée les channels de communication
3. Démarre le stream audio
4. Configure l'état partagé Tauri
5. Lance l'application avec les commandes enregistrées

### `Cargo.toml`

Dépendances :
- `tauri = "2"` - Framework Tauri
- `serde` et `serde_json` - Sérialisation
- `mymusic_daw = { path = ".." }` - Moteur audio

### `tauri.conf.json`

Configuration de l'application :
- Nom, version, identifier
- Configuration de build (devUrl, frontendDist)
- Configuration de fenêtre (taille, titre)
- Bundle settings

## 🔄 Flux de données

### Frontend → Backend (Commandes)

```
React Component
    │
    ▼
invoke('play_note', { note: 60, velocity: 100 })
    │
    ▼
Tauri IPC
    │
    ▼
play_note() in lib.rs
    │
    ▼
Command::Midi(MidiEventTimed)
    │
    ▼
CommandProducer (ringbuffer)
    │
    ▼
Audio Thread
    │
    ▼
CPAL Callback
```

### Backend → Frontend (État)

```
AtomicF32 (volume)
    │
    ▼
get_volume() in lib.rs
    │
    ▼
Tauri IPC
    │
    ▼
invoke<number>('get_volume')
    │
    ▼
React State
```

## 🎵 État partagé (DawState)

```rust
pub struct DawState {
    /// Command producer to send commands to audio thread
    command_tx: Arc<Mutex<CommandProducer>>,

    /// Volume control (atomic for thread-safe access)
    volume_atomic: Arc<AtomicF32>,
}
```

L'état est géré par Tauri avec `.manage(daw_state)` et accessible via `State<DawState>` dans les commandes.

## 🚀 Initialisation

Séquence de démarrage dans `main()` :

1. **Créer les channels** : `create_channels()`
2. **Créer le volume atomique** : `AtomicF32::new(0.5)`
3. **Créer le MIDI manager** : `MidiConnectionManager::new()`
4. **Créer le CPU monitor** : `CpuMonitor::new()`
5. **Créer le moteur audio** : `AudioEngine::new()`
6. **Démarrer le stream** : `audio_engine.start()`
7. **Créer le DawState** : `DawState::new()`
8. **Lancer Tauri** : `tauri::Builder::default()`

## 🔒 Thread Safety

### Atomic Operations

Le volume utilise `AtomicF32` pour un accès lock-free depuis le thread audio :

```rust
volume_atomic.set(0.5);  // Écriture depuis UI
let vol = volume_atomic.get();  // Lecture depuis audio thread
```

### Ringbuffer Lock-Free

Les commandes MIDI utilisent un ringbuffer SPSC (Single Producer Single Consumer) :

```rust
tx.try_push(command)?;  // Non-bloquant, retourne Err si plein
```

### Mutex (Tauri State uniquement)

Le `CommandProducer` est dans un `Mutex` car :
- Accès depuis plusieurs commandes Tauri (multi-threaded)
- Jamais accédé depuis le thread audio (pas de contention RT)

```rust
if let Ok(mut tx) = state.command_tx.lock() {
    tx.try_push(command)?;
}
```

## 📊 Logging

Le backend utilise `println!` et `eprintln!` pour le logging :

```rust
println!("🎵 Initializing MyMusic DAW...");
println!("✅ Audio engine started successfully");
eprintln!("❌ Failed to start audio engine: {}", e);
```

En mode debug, activez les logs détaillés :

```bash
RUST_LOG=debug cargo tauri dev
```

## 🧪 Testing

### Tester les commandes

Vous pouvez tester les commandes Tauri depuis le DevTools (F12) :

```javascript
// Dans la console DevTools
const { invoke } = window.__TAURI__;

// Test volume
await invoke('set_volume', { volume: 0.7 });
const vol = await invoke('get_volume');
console.log('Volume:', vol);

// Test note
await invoke('play_note', { note: 60, velocity: 100 });
await new Promise(r => setTimeout(r, 500));
await invoke('stop_note', { note: 60 });
```

### Tester le moteur sans Tauri

Le moteur audio peut être testé indépendamment :

```bash
cd ..  # Retour à la racine
cargo run  # Lance la version egui
```

## ⚠️ Limitations actuelles

### Real-time Safety

- ✅ Volume : Atomic, RT-safe
- ✅ MIDI : Ringbuffer lock-free, RT-safe
- ⚠️ Audio stream : Géré par référence `std::mem::forget(_stream)`
  - Le stream doit vivre aussi longtemps que l'app
  - Actuellement oublié (`forget`) pour éviter le drop
  - TODO: Stocker dans managed state Tauri

### Notifications Audio → UI

Actuellement non implémenté. Les notifications (CPU load, erreurs audio) vont dans un ringbuffer mais ne sont pas exposées à Tauri.

**TODO** :
- Ajouter des commandes pour récupérer les notifications
- Ou utiliser Tauri events pour push vers le frontend

### Extensions futures

Commands à ajouter :
- `get_active_notes()` - Liste des notes en cours
- `set_waveform(waveform)` - Changer la forme d'onde
- `set_adsr(attack, decay, sustain, release)` - Contrôles ADSR
- `load_plugin(path)` - Charger un plugin CLAP
- `set_tempo(bpm)` - Définir le tempo
- etc.

## 🔧 Développement

### Ajouter une nouvelle commande

1. **Définir la fonction dans `lib.rs`** :

```rust
#[tauri::command]
pub fn my_command(param: u32, state: State<DawState>) -> Result<String, String> {
    // Logique
    Ok("Success".to_string())
}
```

2. **Enregistrer dans `main.rs`** :

```rust
.invoke_handler(tauri::generate_handler![
    // ... autres commandes
    lib::my_command,
])
```

3. **Utiliser dans React** :

```typescript
await invoke('my_command', { param: 42 });
```

### Debugging

Utilisez `dbg!()` pour debug :

```rust
dbg!(volume);
dbg!(&state.volume_atomic.get());
```

### Profiling

Pour profiler le backend :

```bash
cargo install cargo-flamegraph
cargo flamegraph --bin mymusic-daw-tauri
```

## 📚 Ressources

- [Tauri Command System](https://tauri.app/v1/guides/features/command/)
- [Tauri State Management](https://tauri.app/v1/guides/features/state-management/)
- [MyMusic DAW Engine](../README.md)
- [CPAL Documentation](https://docs.rs/cpal/)

## 🎯 Roadmap Backend

- [ ] Implémenter notifications Audio → UI
- [ ] Ajouter commandes ADSR/LFO/Filters
- [ ] Exposer le plugin scanner CLAP
- [ ] Commandes pour le séquenceur
- [ ] Streaming de spectrum/waveform data
- [ ] Gestion de projets (save/load)
