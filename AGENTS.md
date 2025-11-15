# Architecture du DAW Rust - MyMusic DAW

## Vue d'ensemble

Ce DAW est construit autour d'une architecture modulaire qui respecte les contraintes temps-réel de l'audio tout en permettant une évolution future.

## Principes architecturaux fondamentaux

**IMPORTANT : Tous les commentaires de code, documentation inline et messages de commit doivent être en ANGLAIS.**

### 1. **Séparation stricte des threads**

- **Thread UI** (egui/eframe) : Interface utilisateur, interactions
- **Thread MIDI** (midir) : Réception des événements MIDI
- **Thread Audio** (cpal callback) : Génération audio temps-réel (SACRÉ)

### 2. **Règles du callback audio (Zone Sacrée)**

Le callback audio CPAL doit être **prévisible et ultra-rapide** :

- ❌ **INTERDICTIONS ABSOLUES** :

  - Allocations mémoire (`Vec::new`, `Box::new`, `String`)
  - I/O (`println!`, fichiers, réseau)
  - Mutex/locks bloquants
  - Appels système
  - Calculs non-bornés

- ✅ **AUTORISÉ** :
  - Lecture de données pré-allouées
  - Structures lock-free (ringbuffer, atomic)
  - Calculs mathématiques simples (DSP)
  - Accès à des buffers statiques

### 3. **Communication inter-threads**

Utiliser uniquement des structures **lock-free** :

- **ringbuf** : Pour les événements MIDI → Audio
- **std::sync::atomic** : Pour les paramètres simples (volume, fréquence)
- **triple-buffer** (optionnel) : Pour les états complexes

### 4. **Résilience et gestion d'erreurs**

Le DAW doit rester fonctionnel même en cas d'erreur :

- ✅ **Détection des erreurs** :
  - Déconnexion de périphériques (audio, MIDI)
  - Échec d'initialisation
  - Surcharge CPU (audio dropouts)

- ✅ **Récupération gracieuse** :
  - Reconnexion automatique si périphérique disponible
  - Fallback sur périphérique par défaut
  - Notification utilisateur via UI (non-bloquant)

- ✅ **Monitoring** :
  - Mesure de la charge CPU du callback audio
  - Détection des underruns/overruns
  - Métriques de latence

## Architecture modulaire

```
mymusic_daw/
├── src/
│   ├── main.rs              # Point d'entrée, initialisation
│   ├── audio/
│   │   ├── mod.rs           # Module audio principal
│   │   ├── engine.rs        # Moteur audio (callback CPAL)
│   │   ├── device.rs        # Gestion des devices CPAL (sélection, reconnexion)
│   │   ├── buffer.rs        # Gestion des buffers audio
│   │   └── monitor.rs       # Monitoring CPU et métriques (futur)
│   ├── synth/
│   │   ├── mod.rs           # Module synthèse
│   │   ├── oscillator.rs    # Oscillateurs (sine, square, saw, triangle)
│   │   ├── voice.rs         # Voix individuelle (note + oscillateur)
│   │   ├── voice_manager.rs # Gestion polyphonie
│   │   ├── envelope.rs      # Enveloppes ADSR (futur)
│   │   ├── filter.rs        # Filtres (futur)
│   │   └── modulation.rs    # Matrice de modulation (futur)
│   ├── midi/
│   │   ├── mod.rs           # Module MIDI
│   │   ├── input.rs         # Réception MIDI (midir)
│   │   ├── event.rs         # Types d'événements MIDI
│   │   └── clock.rs         # MIDI Clock sync (futur)
│   ├── ui/
│   │   ├── mod.rs           # Module UI
│   │   ├── app.rs           # Application egui principale
│   │   ├── keyboard.rs      # Clavier virtuel
│   │   ├── controls.rs      # Contrôles (sliders, boutons)
│   │   ├── device_picker.rs # Sélection périphériques (futur)
│   │   └── status_bar.rs    # Barre statut et notifications (futur)
│   ├── messaging/
│   │   ├── mod.rs           # Module communication
│   │   ├── command.rs       # Types de commandes (UI → Audio)
│   │   └── channels.rs      # Ringbuffers et canaux
│   └── tauri/               # Frontend Tauri (Phase 7)
│       ├── mod.rs           # Bridge Rust ↔ WebView
│       ├── commands.rs      # Tauri Commands
│       └── events.rs        # Event streaming vers frontend
```

## Flux de données

```
┌──────────────┐
│  MIDI Input  │ (midir)
│  (Thread)    │
└──────┬───────┘
       │ MidiEvent
       ▼
┌──────────────────┐
│   RingBuffer     │ (lock-free)
│   MIDI Events    │
└──────┬───────────┘
       │
       ▼
┌──────────────────────────────────────┐
│      CALLBACK AUDIO (CPAL)           │ ◄── ZONE SACRÉE
│  ┌────────────────────────────────┐  │
│  │ 1. Lire événements MIDI        │  │
│  │ 2. Mettre à jour Voice Manager │  │
│  │ 3. Générer samples (DSP)       │  │
│  │ 4. Écrire dans output buffer   │  │
│  └────────────────────────────────┘  │
└──────┬───────────────────────────────┘
       │ Audio Output
       ▼
┌──────────────┐
│  Speakers    │
└──────────────┘

┌──────────────┐
│   UI (egui)  │ (Thread séparé)
│   (eframe)   │
└──────┬───────┘
       │ Commands (atomics/ringbuffer)
       ▼
  [Audio Engine]
```

## Modules détaillés

### 1. **audio/engine.rs** - Moteur audio

Responsabilités :

- Initialiser le stream CPAL
- Callback audio (lecture events, génération audio)
- Gestion du sample rate
- Format audio (f32, stéréo)

### 2. **synth/oscillator.rs** - Oscillateurs

Types d'ondes :

- `Sine` : Oscillateur sinusoïdal (base)
- `Square` : Onde carrée
- `Saw` : Dent de scie
- `Triangle` : Triangle

Interface :

```rust
trait Oscillator {
    fn next_sample(&mut self) -> f32;
    fn set_frequency(&mut self, freq: f32);
    fn reset(&mut self);
}
```

### 3. **synth/voice.rs** - Voix

Une voix = une note jouée

- MIDI note number
- Oscillateur
- Enveloppe (futur)
- État (active/inactive)

### 4. **synth/voice_manager.rs** - Gestion polyphonie

- Pool de voix pré-allouées (ex: 16 voix)
- Attribution note → voix disponible
- Gestion note-on/note-off
- Voice stealing (si toutes occupées)

### 5. **midi/event.rs** - Événements MIDI

```rust
enum MidiEvent {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8 },
    ControlChange { controller: u8, value: u8 },
    PitchBend { value: i16 },
}
```

### 6. **messaging/channels.rs** - Communication

- `RingBuffer<MidiEvent>` : MIDI → Audio
- `AtomicF32` : Paramètres continus (volume, etc.)

### 7. **ui/app.rs** - Interface

- Clavier virtuel MIDI
- Affichage forme d'onde
- Contrôles (volume, sélection oscillateur)
- Visualisation des voix actives

### 8. **audio/device.rs** - Gestion des périphériques

Architecture pour la sélection et reconnexion :

```rust
struct DeviceManager {
    available_audio_devices: Vec<DeviceInfo>,
    available_midi_ports: Vec<MidiPortInfo>,
    current_audio_device: Option<DeviceHandle>,
    current_midi_port: Option<MidiConnection>,
}

impl DeviceManager {
    fn list_audio_devices() -> Vec<DeviceInfo>;
    fn select_audio_device(&mut self, id: DeviceId) -> Result<()>;
    fn try_reconnect_audio(&mut self) -> Result<()>;
    fn list_midi_ports() -> Vec<MidiPortInfo>;
    fn select_midi_port(&mut self, id: PortId) -> Result<()>;
}
```

**Principes** :

- Énumération périphériques à la demande (pas de polling continu)
- Reconnexion automatique avec timeout exponentiel
- Communication vers UI via événements (ringbuffer)

### 9. **audio/monitor.rs** - Monitoring performances

Mesure de la charge CPU du callback audio :

```rust
struct AudioMonitor {
    callback_duration: AtomicU64,  // en nanoseconds
    buffer_size: usize,
    sample_rate: f32,
}

impl AudioMonitor {
    fn record_callback_start(&self) -> Instant;
    fn record_callback_end(&self, start: Instant);
    fn get_cpu_usage_percent(&self) -> f32;  // Accessible depuis UI
}
```

**Formule** : `CPU% = (callback_time / available_time) * 100`
où `available_time = buffer_size / sample_rate`

Exemple : buffer 512 samples @ 48kHz → 10.6ms disponible

### 10. **synth/modulation.rs** - Matrice de modulation (Phase 2)

Architecture générique pour le routage de modulation :

```rust
enum ModSource {
    Lfo(usize),              // Index du LFO
    Envelope(usize),         // Index de l'envelope
    Velocity,
    Aftertouch,
    ModWheel,
}

enum ModDestination {
    OscillatorPitch(usize),
    FilterCutoff,
    Amplitude,
    Pan,
}

struct ModulationMatrix {
    routings: Vec<ModRouting>,  // Pré-alloué (ex: 32 slots)
}

struct ModRouting {
    source: ModSource,
    destination: ModDestination,
    amount: f32,              // -1.0 à 1.0
    enabled: bool,
}

impl ModulationMatrix {
    fn apply(&self, voice: &mut Voice, mod_values: &ModValues);
}
```

**Principes** :

- Matrice statique pré-allouée (pas d'allocations au runtime)
- Évaluation séquentielle dans le callback audio
- Sources et destinations extensibles via enum
- UI pour éditer routings (drag & drop futur)

### 11. **midi/clock.rs** - Synchronisation externe (Phase 4)

Support MIDI Clock pour sync avec matériel externe :

```rust
enum ClockMode {
    Internal,      // Horloge interne (BPM du DAW)
    MidiClockSlave,   // Suivre MIDI Clock externe
}

struct ClockManager {
    mode: ClockMode,
    bpm: f32,                    // BPM actuel
    tick_counter: u32,           // 24 ticks per quarter note
    last_tick_time: Instant,
}

impl ClockManager {
    fn process_midi_clock(&mut self);
    fn calculate_bpm(&self) -> f32;
    fn send_midi_clock(&self) -> MidiEvent;  // Master mode
}
```

**Protocole MIDI Clock** :

- `0xF8` : Clock tick (24 par noire)
- `0xFA` : Start
- `0xFB` : Continue
- `0xFC` : Stop

### 12. **tauri/** - Frontend moderne (Phase 7) ✅ (TERMINÉ)

Architecture de communication Rust ↔ WebView :

```rust
// tauri/commands.rs
#[tauri::command]
async fn set_volume(volume: f32, state: State<AudioEngine>) -> Result<()> {
    state.set_volume(volume);  // Via atomic
    Ok(())
}

#[tauri::command]
async fn get_audio_devices() -> Result<Vec<DeviceInfo>> {
    Ok(device_manager.list_devices())
}

// tauri/events.rs
struct EventStreamer {
    cpu_usage_emitter: Sender<f32>,
    active_notes_emitter: Sender<Vec<u8>>,
}

impl EventStreamer {
    fn start_streaming(&self, app_handle: AppHandle) {
        // Thread qui poll les métriques et émet vers WebView
        // Throttling : 30-60 FPS max pour ne pas surcharger
    }
}
```

**Architecture hybride** :

- **egui** (Phase 1-6) : Prototypage rapide, UI native
- **Tauri** (Phase 7) : UI moderne, distribution cross-platform
- **Cohabitation** : Possibilité de garder egui pour debug/dev mode

**Séparation des responsabilités** :

- **Backend Rust** : Audio engine reste identique
- **Tauri Layer** : API Commands + Event streaming
- **Frontend** : React/Vue/Svelte pour UI moderne
- **Communication** : Tauri IPC (JSON over WebSocket interne)

**✅ Implémentation complète** :
- **Bridge React ↔ Rust** : Commands Tauri + hooks React (`useDawEngine`, `useAudioEvents`)
- **Event system temps-réel** : AudioEventEmitter avec throttling/debouncing optimisé
- **Error handling robuste** : Global error boundary + toast notifications
- **Performance monitoring** : StatusBar avec métriques CPU/MIDI/Audio
- **Tests complets** : Rust unit tests + React hook tests + integration tests
- **Documentation système** : Guides utilisateur et API complète

## Point d'étape initial (MVP)

### Objectif : Synthétiseur monophonique simple

**Fonctionnalités** :

- ✅ 1 oscillateur sinus
- ✅ Input MIDI (clavier externe)
- ✅ Sortie audio
- ✅ UI basique (volume, forme d'onde)

**Ce qui est préparé pour l'évolution** :

- Architecture modulaire (facile d'ajouter oscillateurs)
- Voice system (passage à polyphonie trivial)
- Communication lock-free (scalable)

## Extensions futures (par ordre de priorité)

### Phase 2 : Enrichissement du son

- Enveloppe ADSR
- Multiple waveforms (square, saw, triangle)
- Polyphonie (4-16 voix)
- **Matrice de modulation** (architecture extensible)

### Phase 3 : Filtres et effets

- Low-pass filter (Moog-style)
- High-pass, band-pass
- Réverbération
- Delay

### Phase 4 : Séquenceur

- Piano roll
- Step sequencer
- Timeline
- **MIDI Clock sync** (Master/Slave)

### Phase 5 : Plugins et routing

- Architecture de plugins
- Routing audio flexible
- Mixeur multi-pistes

### Phase 6 : Production-ready

- **Gestion robuste des périphériques** (sélection, reconnexion)
- **Monitoring CPU** en temps réel
- Tests unitaires DSP
- Documentation complète

### Phase 7 : Frontend Tauri

- UI moderne cross-platform
- Bridge Rust ↔ WebView
- Distribution packaging

## Dépendances Cargo.toml

```toml
[dependencies]
cpal = "0.15"           # Audio I/O
midir = "0.9"           # MIDI input
eframe = "0.30"         # UI framework (inclut egui)
egui = "0.30"           # Immediate mode GUI
ringbuf = "0.4"         # Lock-free ringbuffer
atomic_float = "1.0"    # AtomicF32
```

## Bonnes pratiques

### Performance

- Pré-allouer toute la mémoire au démarrage
- Utiliser des buffers circulaires
- Éviter les branches dans le callback audio
- Utiliser SIMD pour le DSP (futur)

### Organisation du code

- Un module = une responsabilité
- Traits pour l'abstraction (Oscillator, Effect, etc.)
- Tests unitaires pour chaque module
- Documentation inline
- **IMPORTANT : Tous les commentaires de code doivent être en ANGLAIS**

### Points de vigilance

- **Mesure CPU dans le callback**: éviter `Instant::now()` à chaque buffer; échantillonner 1/N callbacks, accumuler des compteurs atomiques, et calculer/publier hors chemin critique quand possible.
- **Dispatch oscillateurs**: `Box<dyn Oscillator>` est flexible mais coûte du dispatch dynamique; préférer un `enum OscKind` + `match` (static dispatch) ou des voix spécialisées pour le cœur DSP.
- **Horodatage MIDI**: ajouter des timestamps relatifs en samples (`samples_from_now`) pour un scheduling sample-accurate et réduire le jitter.
- **Paramètres atomiques**: stocker les `f32` via `AtomicU32` (bits) et appliquer un smoothing 1‑pole côté audio pour éviter le zipper noise.
- **Gestion d’erreurs CPAL**: utiliser le callback d’erreur du stream, tenter un redémarrage avec backoff exponentiel borné, et fallback device si nécessaire.
- **Dénormaux et saturation**: empêcher les denormals (FTZ/DAZ ou offset minuscule) et choisir clamp ou soft clip (ex. tanh) pour [-1, 1].
- **Formats/buffers CPAL**: gérer `i16/u16` et interleaved vs non‑interleaved de manière explicite et sans allocations.
- **Priorités threads**: viser une priorité élevée (RT si possible) pour l’audio; jamais de logs/I/O/allocations dans le callback.

### Décisions d'implémentation recommandées

- `MidiEventTimed`:
  - `struct MidiEventTimed { event: MidiEvent, samples_from_now: u32 }`
  - Côté thread MIDI: convertir le timestamp en samples au sample rate courant et remplir `samples_from_now`.
  - Côté audio: consommer et déclencher l’événement à l’échantillon.
- Monitoring CPU échantillonné:
  - Mesurer 1/N callbacks et accumuler dans des `AtomicU64` (ns et compteurs), calculer `CPU% = callback_time / (buffer_size / sample_rate)`.
  - Publier vers l’UI via atomics/ringbuffer à 30–60 Hz max.
- Paramètres continus:
  - Représenter `f32` en `AtomicU32` via `to_bits/from_bits`; appliquer un filtre 1‑pole côté audio pour transitions douces.
- Anti‑dénormaux:
  - Utilitaire `fn sanitize(sample: f32) -> f32` qui force FTZ ou ajoute un très faible offset pour éviter la chute de perf.
- Oscillateurs:
  - Préférer `enum OscKind { Sine, Square, Saw, Triangle }` et un `match` dans la boucle DSP pour du static dispatch.
- CPAL formats:
  - Normaliser en interne vers `f32` et gérer conversion depuis/vers `i16/u16` et interleaved/non‑interleaved sans allocations dans le callback.

### Debugging

- Métriques de performance (temps callback)
- Logs en dehors du callback audio
- Mode "safe" pour désactiver optimisations pendant debug

## Principes de design et architecture

### DRY (Don't Repeat Yourself)

**Abstractions via Traits** :

```rust
// ❌ Mauvais : Code dupliqué pour chaque oscillateur
impl SineOsc {
    fn set_frequency(&mut self, freq: f32) { self.freq = freq; }
}
impl SawOsc {
    fn set_frequency(&mut self, freq: f32) { self.freq = freq; }
}

// ✅ Bon : Trait partagé
trait Oscillator {
    fn next_sample(&mut self) -> f32;
    fn set_frequency(&mut self, freq: f32);
    fn reset(&mut self);
}

// Implémentation unique pour le polymorphisme
struct Voice {
    oscillator: Box<dyn Oscillator>,  // N'importe quel oscillateur
}
```

**Réutilisation de composants DSP** :

```rust
// Module audio/dsp/common.rs
fn midi_to_frequency(note: u8) -> f32 {
    440.0 * 2f32.powf((note as f32 - 69.0) / 12.0)
}

fn db_to_linear(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

// Utilisé partout : oscillators, filters, effects, voice_manager, etc.
```

### SOLID Principles

**Single Responsibility Principle** :

```rust
// ❌ Mauvais : Classe fourre-tout
struct AudioEngine {
    fn process_midi(&mut self);
    fn render_audio(&mut self);
    fn manage_ui_state(&mut self);
    fn save_project(&self);
}

// ✅ Bon : Responsabilités séparées
struct MidiProcessor { /* ... */ }
struct AudioRenderer { /* ... */ }
struct ProjectManager { /* ... */ }
// UI reste dans son propre module
```

**Open/Closed Principle** :

```rust
// Extensible sans modifier le code existant
trait Effect: Send {
    fn process(&mut self, buffer: &mut [f32]);
    fn set_parameter(&mut self, param: &str, value: f32);
}

// Ajouter un nouvel effet n'impacte pas le moteur
struct EffectChain {
    effects: Vec<Box<dyn Effect>>,
}

impl EffectChain {
    fn add_effect(&mut self, effect: Box<dyn Effect>) {
        self.effects.push(effect);
    }
}
```

**Dependency Inversion** :

```rust
// ❌ Mauvais : Dépendance concrète
struct Voice {
    oscillator: SineOscillator,  // Couplage fort
}

// ✅ Bon : Dépendance abstraite
struct Voice {
    oscillator: Box<dyn Oscillator>,  // Injection de dépendance
}

// Permet de tester avec un mock
struct MockOscillator;
impl Oscillator for MockOscillator { /* ... */ }
```

### Composition over Inheritance

Rust favorise la composition (pas d'héritage de classes) :

```rust
// Composition de comportements via traits
struct Voice {
    oscillator: Box<dyn Oscillator>,
    envelope: ADSR,
    filter: Option<Box<dyn Filter>>,
}

// Chaque composant est indépendant et testable
impl Voice {
    fn process(&mut self) -> f32 {
        let sample = self.oscillator.next_sample();
        let amp = self.envelope.next_sample();
        let filtered = self.filter
            .as_mut()
            .map(|f| f.process(sample))
            .unwrap_or(sample);
        filtered * amp
    }
}
```

### Fail-Fast vs Fail-Safe

**Dans le code temps-réel (callback audio)** :

```rust
// ✅ Fail-safe : Ne jamais paniquer
fn audio_callback(data: &mut [f32]) {
    // Si erreur, silence plutôt que crash
    match midi_receiver.try_recv() {
        Ok(event) => process_event(event),
        Err(_) => { /* Silently continue */ }
    }

    // Clamp values au lieu de paniquer
    for sample in data.iter_mut() {
        *sample = sample.clamp(-1.0, 1.0);
    }
}
```

**Dans le code d'initialisation** :

```rust
// ✅ Fail-fast : Détecter les erreurs tôt
fn initialize_audio() -> Result<AudioEngine, AudioError> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or(AudioError::NoDevice)?;  // Échouer immédiatement

    let config = device.default_output_config()
        .map_err(AudioError::ConfigError)?;

    // Valider la configuration
    if config.sample_rate().0 < 44100 {
        return Err(AudioError::UnsupportedSampleRate);
    }

    Ok(AudioEngine::new(device, config))
}
```

### Séparation des préoccupations (Separation of Concerns)

**Layers d'architecture** :

```text
┌─────────────────────────────────────────┐
│  UI Layer (egui/Tauri)                  │  Présentation
│  - Affichage                            │
│  - Interactions utilisateur             │
└────────────────┬────────────────────────┘
                 │ Commands (atomics)
┌────────────────▼────────────────────────┐
│  Application Layer                      │  Logique métier
│  - Voice Manager                        │
│  - Effect Chain                         │
│  - Modulation Matrix                    │
└────────────────┬────────────────────────┘
                 │ DSP calls
┌────────────────▼────────────────────────┐
│  DSP Layer                              │  Traitement signal
│  - Oscillators                          │
│  - Filters                              │
│  - Envelopes                            │
└────────────────┬────────────────────────┘
                 │ Audio samples
┌────────────────▼────────────────────────┐
│  Hardware Layer (CPAL)                  │  I/O
│  - Audio device                         │
│  - MIDI input                           │
└─────────────────────────────────────────┘
```

**Pas de mélange des responsabilités** :

```rust
// ❌ Mauvais : DSP mélangé avec UI
fn render_oscillator(ctx: &Context) {
    let sample = (phase * TAU).sin();  // DSP dans l'UI !
    ui.label(format!("Sample: {}", sample));
}

// ✅ Bon : Séparation claire
// Dans synth/oscillator.rs
fn next_sample(&mut self) -> f32 {
    let sample = (self.phase * TAU).sin();
    self.phase += self.phase_increment;
    sample
}

// Dans ui/app.rs
fn render_oscillator_ui(&mut self, ctx: &Context) {
    // UI seulement, pas de DSP
    ui.label(format!("Active voices: {}", self.voice_count.load()));
}
```

### YAGNI (You Aren't Gonna Need It)

**Développement itératif** :

```rust
// Phase 1 : Simple, fonctionnel
struct Voice {
    oscillator: SineOscillator,
    note: u8,
    active: bool,
}

// Phase 2 : Ajouter complexité quand nécessaire
struct Voice {
    oscillator: Box<dyn Oscillator>,  // Multiple waveforms
    envelope: ADSR,                   // Enveloppe
    note: u8,
    velocity: u8,
    active: bool,
}

// Ne PAS développer dès le début :
// - Wavetable synthesis
// - Granular engine
// - FM synthesis
// → Ajouter seulement quand le besoin est réel
```

### Immutabilité et État partagé

**Préférer l'immutabilité** :

```rust
// ✅ Bon : Paramètres immutables, état interne mutable
struct Oscillator {
    frequency: f32,       // Set une fois, lu N fois
    phase: f32,          // État interne mutable
    phase_increment: f32, // Dérivé de frequency
}

impl Oscillator {
    fn set_frequency(&mut self, freq: f32) {
        self.frequency = freq;
        self.phase_increment = freq / self.sample_rate;
    }

    fn next_sample(&mut self) -> f32 {
        let sample = (self.phase * TAU).sin();
        self.phase += self.phase_increment;  // Seul changement
        self.phase %= TAU;
        sample
    }
}
```

**État partagé minimal** :

```rust
// Seulement ce qui doit être partagé entre threads
struct SharedState {
    volume: AtomicF32,           // UI → Audio
    cpu_usage: AtomicF32,        // Audio → UI
    midi_events: RingBuffer<Event>,  // MIDI → Audio
}

// Le reste reste local à chaque thread
// → Pas de contention, performance maximale
```

## Références techniques

- **Sample Rate** : 44100 Hz ou 48000 Hz
- **Buffer Size** : 512 samples (compromis latence/stabilité)
- **Format** : f32 interleaved stereo
- **MIDI** : Standard MIDI 1.0
- **Fréquence MIDI** : `440.0 * 2^((note - 69) / 12.0)`

### Décisions plateforme à préciser

- Systèmes cibles prioritaires: macOS (CoreAudio), Windows (WASAPI), Linux (ALSA/Pulse/JACK?)
- Stratégie de latence: taille de tampon par défaut et options d’ajustement UI
- Politique de fallback périphériques et reconnection automatique
- Politique de priorisation threads selon OS (capabilities/limitations)

## Stratégie de tests

### Tests unitaires DSP

Valider la précision des algorithmes audio :

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_sine_oscillator_frequency() {
        let mut osc = SineOscillator::new(440.0, 48000.0);
        // Générer 1 seconde, compter les zéro-crossings
        // Doit être proche de 440 Hz
    }

    #[test]
    fn test_adsr_envelope_timing() {
        let adsr = ADSR::new(0.1, 0.2, 0.7, 0.3);
        // Valider durées Attack, Decay, Release
    }
}
```

### Tests d'intégration

- Flux MIDI → Audio end-to-end
- Reconnexion périphériques
- Latence mesurée

### Benchmarks

```rust
#[bench]
fn bench_audio_callback(b: &mut Bencher) {
    // Mesurer temps d'exécution callback avec 16 voix actives
    // Target : < 50% du temps disponible
}
```

### Test Coverage Complet ✅ (TERMINÉ)

**Couverture actuelle** : **228 tests passants** 🎉

**Tests unitaires (55 tests)** :
- Oscillateurs (8 tests) : fréquence, amplitude, phase
- Voice Manager (8 tests) : allocation, voice stealing
- MIDI parsing (11 tests) : tous les types d'événements
- Anti-dénormaux et smoothing (4 tests) : paramètres continus
- Audio timing (6 tests) : conversions temps ↔ samples
- CPU monitoring (5 tests) : mesure charge callback
- Reconnexion automatique (3 tests) : détection/reconnexion
- Notifications (3 tests) : système UI non-bloquant
- Format conversion (8 tests) : f32 ↔ i16 ↔ u16

**Tests d'intégration (11 tests)** :
- MIDI → Audio end-to-end (4 tests)
- Latency benchmark (< 10ms target) - **ATTEINT: ~200ns NoteOn + 69µs buffer**
- Stabilité court (5 min) - **990M samples, 0 crash**
- Stabilité stress polyphonique (30s, 16 voix)
- Stabilité rapid notes (10,000 cycles)
- Stabilité long (1h) - disponible avec `--ignored`

**Tests spécifiques par module** :
- Command Pattern (13 tests)
- ADSR (10 tests)
- LFO (11 tests)
- Voice Stealing (2 tests)
- Polyphony Modes (14 tests)
- Portamento (9 tests)
- Filter (18 tests)
- Filter Integration (4 tests)
- Modulation Matrix (1 test)
- Voice (4 tests)
- Sampler (6 tests)
- Sampler Engine (18 tests)
- Sample Bank (3 tests)
- Sequencer (Timeline/Transport) (14 tests)
- Pattern (9 tests)
- Note (10 tests)
- SequencerPlayer (3 tests)

**Tests Tauri Bridge** :
- Rust Commands (tests unitaires complets)
- React Hooks (`useDawEngine.test.ts`, `useAudioEvents.test.ts`)
- Integration UI ↔ Audio

**Infrastructure de tests** :
- GitHub Actions CI/CD avec multi-platform builds
- Benchmarks Criterion avec HTML reports
- Tests de performance et stabilité
- Documentation complète dans `TESTING.md`

---

## Référence à la roadmap

Pour le suivi du développement et l'état d'avancement des fonctionnalités, se référer au fichier **TODO.md** qui contient :

- **Roadmap détaillée** par phases (Phase 1 → Phase 7)
- **État d'avancement** en temps réel de chaque fonctionnalité
- **Priorités actuelles** et prochaines étapes
- **Décisions architecturales** critiques avec justifications
- **Milestones** et versions de release

Le TODO.md sert de **source de vérité** pour l'état du projet, tandis que ce document AGENTS.md se concentre sur l'architecture technique et les principes de design.

**Version** : 2.0.0
**Dernière mise à jour** : 2025-11-14
