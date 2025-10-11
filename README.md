# MyMusic DAW

Un DAW (Digital Audio Workstation) minimaliste écrit en Rust.

## Architecture

- **Audio Backend** : CPAL (callback temps-réel, lock-free)
- **MIDI Input** : midir
- **Interface** : egui/eframe (à venir)
- **Communication** : Ringbuffer lock-free entre threads

Voir [AGENTS.md](AGENTS.md) pour l'architecture complète.

## État actuel

### Phase 1 (MVP) ✅ TERMINÉ
- ✅ Moteur audio CPAL avec callback temps-réel
- ✅ Système de communication lock-free (2 ringbufs : MIDI + UI → Audio)
- ✅ Oscillateurs (Sine, Square, Saw, Triangle)
- ✅ Voice Manager avec polyphonie (16 voix)
- ✅ Input MIDI (détection automatique du premier port)
- ✅ Conversion MIDI note → fréquence

### Phase 1.5 (Robustesse et UX) 🔥 EN COURS
✅ **Implémenté** :
- **Gestion des périphériques**
  - Énumération des périphériques audio/MIDI
  - Sélecteurs UI pour audio output et MIDI input
  - Reconnexion automatique avec backoff exponentiel
  - Hot-swapping des périphériques
- **Interface utilisateur améliorée**
  - Clavier virtuel avec touches PC (A W S E D F T G Y H U J K)
  - Clavier visuel cliquable
  - Slider de volume (connecté à l'audio avec smoothing)
  - Sélecteur de forme d'onde (Sine, Square, Saw, Triangle)
  - Affichage du nombre de notes actives
  - Barre de statut avec notifications
- **Monitoring CPU**
  - Indicateur de charge CPU en temps réel
  - Couleurs : vert (<50%), orange (50-75%), rouge (>75%)
  - Alertes en cas de surcharge
- **Hygiène DSP**
  - Anti-dénormaux (flush-to-zero)
  - Soft-saturation sur la sortie
  - Smoothing 1-pole pour paramètres continus
  - AtomicF32 thread-safe
- **Timing MIDI (infrastructure)**
  - Structure `MidiEventTimed` avec `samples_from_now`
  - Module `AudioTiming` pour conversions précises
  - Scheduling sample-accurate dans callback audio
  - Ringbuffers dimensionnés pour pire rafale MIDI (512 événements)
- **Tests**
  - 47 tests unitaires ✅
  - Tests oscillateurs, voice manager, MIDI parsing
  - Tests DSP (anti-dénormaux, smoothing)
  - Tests timing, CPU monitoring, reconnexion

🎯 **Prochaines étapes (Phase 1.5)** :
- Tests d'intégration (MIDI → Audio end-to-end)
- Test de latency benchmark (< 10ms target)
- Support formats CPAL (i16/u16)
- Documentation (cargo doc, README, CONTRIBUTING)

🚀 **Prochaine phase (Phase 2)** :
- Enveloppe ADSR
- Modulation LFO
- Polyphonie avancée

## Utilisation

### Prérequis

- Rust (edition 2024)
- Un device audio de sortie
- (Optionnel) Un clavier MIDI

### Lancer le DAW

```bash
cargo run
```

Le programme va :
1. Initialiser le moteur audio (CPAL)
2. Détecter et se connecter au premier port MIDI disponible
3. Attendre des événements MIDI

### Tester avec un clavier MIDI

Branchez un clavier MIDI et jouez des notes. Vous devriez entendre un son d'oscillateur sinus.

### Tester sans clavier MIDI

Si aucun port MIDI n'est détecté, le programme continuera mais n'émettra pas de son (attente d'événements MIDI).

Pour tester sans clavier physique :
- Sur macOS : Utiliser un IAC Driver ou un clavier virtuel MIDI
- Sur Linux : Utiliser ALSA MIDI ou JACK
- Sur Windows : Utiliser un driver MIDI virtuel

## Architecture du code

```
src/
├── audio/
│   ├── engine.rs       # Moteur CPAL et callback temps-réel
│   ├── timing.rs       # Timing sample-accurate pour MIDI
│   ├── cpu_monitor.rs  # Monitoring de la charge CPU
│   ├── dsp_utils.rs    # Utilitaires DSP (anti-dénormaux, smoothing)
│   ├── parameters.rs   # Paramètres atomiques thread-safe
│   ├── device.rs       # Gestion des périphériques audio
│   └── buffer.rs       # Buffers audio (future)
├── synth/
│   ├── oscillator.rs   # Oscillateurs (Sine, Square, Saw, Triangle)
│   ├── voice.rs        # Système de voix
│   └── voice_manager.rs # Polyphonie (16 voix)
├── midi/
│   ├── event.rs        # Types MIDI et MidiEventTimed
│   ├── input.rs        # Input MIDI de base (legacy)
│   ├── manager.rs      # Connection manager avec reconnexion auto
│   └── device.rs       # Énumération des périphériques MIDI
├── connection/
│   ├── status.rs       # Status atomique des connexions
│   └── reconnect.rs    # Stratégie de reconnexion avec backoff
├── messaging/
│   ├── channels.rs     # Création des ringbuffers lock-free
│   ├── command.rs      # Types de commandes (UI → Audio)
│   └── notification.rs # Système de notifications (Audio → UI)
└── ui/
    └── app.rs          # Interface egui/eframe
```

## Règles du callback audio (Zone Sacrée)

Le callback audio CPAL est **critique pour la performance** :

❌ **INTERDIT** :
- Allocations mémoire
- I/O (println!, fichiers)
- Mutex bloquants
- Appels système

✅ **AUTORISÉ** :
- Lecture de structures pré-allouées
- Ringbuffer lock-free
- Calculs DSP simples
- try_lock (non-bloquant)

## Roadmap

Voir [TODO.md](TODO.md) pour la roadmap complète.

### Phase 1 (MVP) ✅ TERMINÉ
- [x] Audio engine CPAL
- [x] MIDI input
- [x] Oscillateurs de base
- [x] Polyphonie
- [x] UI basique

### Phase 1.5 (Robustesse - en cours) 🔥
- [x] Gestion des périphériques audio/MIDI
- [x] Reconnexion automatique
- [x] Timing MIDI (infrastructure)
- [x] Monitoring CPU
- [x] Hygiène DSP et paramètres
- [x] 47 tests unitaires
- [ ] Tests d'intégration
- [ ] Documentation complète

### Phase 2 (Enrichissement du son)
- Enveloppes ADSR
- Modulation (LFO, vélocité)
- Polyphonie avancée

### Phase 3 (Filtres et effets)
- Filtres (LP, HP, BP)
- Effets (delay, reverb)

### Phase 4 (Séquenceur)
- Timeline et transport
- Piano roll
- Recording MIDI
- Persistance projets

### Phase 5+ (Plugins et distribution)
- Support CLAP plugins
- Routing audio avancé
- VST3 (optionnel)
- Distribution (Tauri + licensing)

## Développement

### Build

```bash
cargo build          # Debug build
cargo build --release # Release build (optimized)
```

### Run

```bash
cargo run            # Debug mode
cargo run --release  # Release mode (better audio performance)
```

### Tests

```bash
cargo test           # Run all 47 unit tests
cargo test -- --nocapture # Show println! output
```

### Check

```bash
cargo check          # Fast compile check
cargo clippy         # Linter
cargo fmt            # Format code
```

## License

MIT (à définir)

## Credits

- CPAL : Cross-platform audio I/O
- midir : Cross-platform MIDI I/O
- egui : Immediate mode GUI
- ringbuf : Lock-free ring buffer
