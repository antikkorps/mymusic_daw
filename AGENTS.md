# Architecture du DAW Rust - MyMusic DAW

## Vue d'ensemble

Ce DAW est construit autour d'une architecture modulaire qui respecte les contraintes temps-réel de l'audio tout en permettant une évolution future.

## Principes architecturaux fondamentaux

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

## Architecture modulaire

```
mymusic_daw/
├── src/
│   ├── main.rs              # Point d'entrée, initialisation
│   ├── audio/
│   │   ├── mod.rs           # Module audio principal
│   │   ├── engine.rs        # Moteur audio (callback CPAL)
│   │   ├── device.rs        # Gestion des devices CPAL
│   │   └── buffer.rs        # Gestion des buffers audio
│   ├── synth/
│   │   ├── mod.rs           # Module synthèse
│   │   ├── oscillator.rs    # Oscillateurs (sine, square, saw, triangle)
│   │   ├── voice.rs         # Voix individuelle (note + oscillateur)
│   │   ├── voice_manager.rs # Gestion polyphonie
│   │   ├── envelope.rs      # Enveloppes ADSR (futur)
│   │   └── filter.rs        # Filtres (futur)
│   ├── midi/
│   │   ├── mod.rs           # Module MIDI
│   │   ├── input.rs         # Réception MIDI (midir)
│   │   └── event.rs         # Types d'événements MIDI
│   ├── ui/
│   │   ├── mod.rs           # Module UI
│   │   ├── app.rs           # Application egui principale
│   │   ├── keyboard.rs      # Clavier virtuel
│   │   └── controls.rs      # Contrôles (sliders, boutons)
│   └── messaging/
│       ├── mod.rs           # Module communication
│       ├── command.rs       # Types de commandes (UI → Audio)
│       └── channels.rs      # Ringbuffers et canaux
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

### Phase 3 : Filtres et effets
- Low-pass filter (Moog-style)
- High-pass, band-pass
- Réverbération
- Delay

### Phase 4 : Séquenceur
- Piano roll
- Step sequencer
- Timeline

### Phase 5 : Plugins et routing
- Architecture de plugins
- Routing audio flexible
- Mixeur multi-pistes

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

### Debugging
- Métriques de performance (temps callback)
- Logs en dehors du callback audio
- Mode "safe" pour désactiver optimisations pendant debug

## Références techniques

- **Sample Rate** : 44100 Hz ou 48000 Hz
- **Buffer Size** : 512 samples (compromis latence/stabilité)
- **Format** : f32 interleaved stereo
- **MIDI** : Standard MIDI 1.0
- **Fréquence MIDI** : `440.0 * 2^((note - 69) / 12.0)`

---

**Version** : 0.1.0
**Dernière mise à jour** : 2025-10-05
