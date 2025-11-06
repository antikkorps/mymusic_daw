# MyMusic DAW

[![Test](https://github.com/franck/my-music-daw/actions/workflows/test.yml/badge.svg)](https://github.com/franck/my-music-daw/actions/workflows/test.yml)

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

### Phase 1.5 (Robustesse et UX) ✅ TERMINÉ - v0.2.0 🎉
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
- **Timing MIDI**
  - Structure `MidiEventTimed` avec `samples_from_now`
  - Module `AudioTiming` pour conversions précises
  - Scheduling sample-accurate dans callback audio
  - Ringbuffers dimensionnés pour pire rafale MIDI (512 événements)
- **Compatibilité formats audio**
  - Support F32, I16, U16 (conversion automatique)
  - Détection format device et adaptation
  - Tests de conversion et roundtrip
- **Tests complets** ✅
  - **66 tests passent** (55 unitaires + 11 intégration)
  - Tests oscillateurs, voice manager, MIDI parsing
  - Tests DSP (anti-dénormaux, smoothing, format conversion)
  - Tests timing, CPU monitoring, reconnexion
  - Tests d'intégration MIDI → Audio end-to-end
  - Tests de latence (< 10ms target **ATTEINT**)
  - Tests de stabilité (990M samples, 0 crash)
- **Benchmarks Criterion** ✅
  - Benchmarks oscillateurs, voice processing
  - Benchmark MIDI → Audio pipeline
  - Latence mesurée : ~200ns NoteOn, 69µs buffer (153x faster than real-time)
  - Rapports HTML disponibles

**Performance mesurée** :
- ⚡ Latence NoteOn : ~200ns
- ⚡ Génération audio : 153x plus rapide que temps réel
- ✅ Target < 10ms : **ATTEINT**
- ✅ Stabilité : 990M samples sans crash

### Phase 2 (Enrichissement du son) ✅ TERMINÉ - v0.3.0 🎉
✅ **Implémenté** :
- **Enveloppes ADSR**
  - Attack, Decay, Sustain, Release
  - Support vélocité MIDI
  - Intégration mod matrix (source Envelope)
- **Modulation complète**
  - 2 LFOs avec formes d'onde (Sine, Triangle, Saw, Square, Random)
  - Mod Matrix flexible (6 slots, 8 sources, 9 destinations)
  - Sources : LFO1, LFO2, Velocity, Aftertouch, ModWheel, Envelope, PitchBend, KeyTracking
  - Destinations : Pitch, Volume, FilterCutoff, FilterRes, LFO1Rate, LFO1Depth, LFO2Rate, LFO2Depth, Pan
  - Depth control par slot (-100% à +100%)
- **Polyphonie avancée**
  - 3 modes : Poly, Mono, Legato
  - Voice stealing intelligent (voix la plus ancienne)
  - Portamento/glide avec contrôle de temps
  - Note priority pour mode mono
- **Tests** : 156 tests passent (88 nouveaux pour Phase 2)

### Phase 3a (Filtres et Effets) ✅ TERMINÉ - v0.4.0 🎉
✅ **Implémenté** :
- **Architecture d'effets**
  - Trait générique `Effect` pour tous les effets audio
  - `EffectChain` pour chaîner plusieurs effets en série
  - Wrappers : FilterEffect, DelayEffect, ReverbEffect
  - Real-time safe : pas d'allocations, lock-free
- **Filtres**
  - State Variable Filter (SVF) avec LP, HP, BP
  - Cutoff 20Hz - 20kHz, Resonance 0-10
  - Modulation cutoff/resonance via mod matrix
- **Delay**
  - Circular buffer jusqu'à 1 seconde
  - Paramètres : time_ms, feedback (0-0.99), mix
  - Smoothing pour éviter les clicks
- **Reverb (Freeverb)**
  - 4 comb filters parallèles avec damping
  - 2 allpass filters pour diffusion
  - Paramètres : room_size, damping, mix
  - Tunings: CLAP [1116, 1188, 1277, 1356], ALLPASS [556, 441]
- **Pipeline audio** : Oscillator → Filter → EffectChain → Envelope → Pan
- **Tests** : 178 tests passent (22 nouveaux pour Phase 3a)

### Phase 5 (Plugins CLAP) ✅ INFRASTRUCTURE COMPLÈTE
✅ **Implémenté** (~3500 lignes, 7 parties) :
- **FFI & Loading** (`clap_ffi.rs` - 478 lignes)
  - Bindings C complets pour CLAP 1.0.0
  - Structures: clap_plugin, clap_host, clap_process, clap_audio_buffer
  - Extensions: parameters, GUI, state
  - Chargement dynamique avec libloading
  - Support cross-platform: macOS bundles (.clap), Linux (.so), Windows (.dll)
- **Instance & Lifecycle** (`clap_integration.rs`)
  - ClapPluginInstance avec gestion complète du cycle de vie
  - Minimal CLAP host implementation
  - init() → activate() → start_processing() → process() → stop_processing() → deactivate()
  - Drop trait pour cleanup automatique des ressources
- **Audio Processing**
  - Conversion bidirectionnelle AudioBuffer ↔ clap_audio_buffer
  - Appel réel du callback process() du plugin
  - Gestion des status: CONTINUE, TAIL, SLEEP, ERROR
  - Integration avec le système de buffers existant
- **MIDI Events**
  - Support complet des événements CLAP (note, MIDI, param)
  - ClapEventList avec callbacks FFI (size, get)
  - NoteOn/NoteOff avec vélocité et channel
  - Sample-accurate timing (offset en samples)
- **Parameter Automation**
  - clap_event_param_value pour automation sample-accurate
  - Parameter ID mapping (string → u32)
  - Queue de changements de paramètres
  - Support modulation en temps réel
- **GUI Embedding** (`clap_gui.rs` - 307 lignes)
  - ClapPluginGui wrapper cross-platform
  - Window handles: cocoa (macOS), x11/wayland (Linux), win32 (Windows)
  - API: create(), attach_to_window(), show(), hide(), set_size()
  - Détection automatique du meilleur API par plateforme
  - Support redimensionnement avec can_resize()
- **Buffer Pool Optimization** (`buffer_pool.rs` - 212 lignes)
  - AudioBufferPool avec pré-allocation (zero-allocation en RT)
  - prepare() pour réutilisation efficace des buffers
  - **Performance critique**: 10-20 allocations → 0 allocations par process()
  - Real-time safety garantie
- **Plugin Scanner**
  - Scan automatique des répertoires système
  - Cache JSON persistant pour performances
  - Blacklist pour plugins problématiques
  - Validation et gestion d'erreurs
- **UI Integration** (`app.rs` - Plugin tab)
  - Onglet Plugins dans l'interface principale
  - Boutons Scan/Rescan avec indicateur de progression
  - Liste détaillée: nom, vendor, version, catégorie, features
  - Affichage des chemins de recherche par plateforme
  - Méthode scan_plugins() avec gestion multi-directories

**Chemins de recherche CLAP par défaut** :
- macOS: `/Library/Audio/Plug-Ins/CLAP`, `~/Library/Audio/Plug-Ins/CLAP`
- Linux: `/usr/lib/clap`, `~/.clap`
- Windows: `C:\Program Files\Common Files\CLAP`, `%LOCALAPPDATA%\Programs\Common\CLAP`

**Test program** : `cargo run --bin test_clap`

🔄 **À venir (Phase 5 suite)** :
- Routing audio vers plugins
- Affichage des paramètres dans l'UI
- Automation dans le séquenceur
- Tests avec vrais plugins CLAP (Surge XT, Vital, Airwindows)

🚀 **Prochaine phase (Phase 3b)** :
- Dogfooding : créer une chanson complète avec le DAW
- UI pour contrôles Delay et Reverb
- Presets pour effets

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
├── lib.rs              # Exports pour tests et benchmarks
├── main.rs             # Point d'entrée binaire
├── audio/
│   ├── engine.rs       # Moteur CPAL et callback temps-réel
│   ├── export.rs       # Export audio (WAV/FLAC)
│   ├── timing.rs       # Timing sample-accurate pour MIDI
│   ├── cpu_monitor.rs  # Monitoring de la charge CPU
│   ├── dsp_utils.rs    # Utilitaires DSP (anti-dénormaux, smoothing)
│   ├── parameters.rs   # Paramètres atomiques thread-safe
│   ├── device.rs       # Gestion des périphériques audio
│   ├── format_conversion.rs # Conversions F32/I16/U16
│   └── buffer.rs       # Buffers audio
├── synth/
│   ├── oscillator.rs   # Oscillateurs (Sine, Square, Saw, Triangle)
│   ├── envelope.rs     # Enveloppes ADSR
│   ├── lfo.rs          # LFO (Sine, Triangle, Saw, Square, Random)
│   ├── modulation.rs   # Mod Matrix (6 slots, 8 sources, 9 destinations)
│   ├── filter.rs       # State Variable Filter (LP, HP, BP)
│   ├── effect.rs       # Architecture d'effets (Effect trait, EffectChain)
│   ├── delay.rs        # Delay avec circular buffer
│   ├── reverb.rs       # Reverb (Freeverb avec comb/allpass)
│   ├── poly_mode.rs    # Modes de polyphonie (Poly, Mono, Legato)
│   ├── portamento.rs   # Portamento/glide
│   ├── voice.rs        # Système de voix avec pipeline complet
│   └── voice_manager.rs # Polyphonie (16 voix) + voice stealing
├── plugin/             # ** NOUVEAU - Phase 5 **
│   ├── mod.rs          # Module exports et traits Plugin
│   ├── clap_ffi.rs     # FFI bindings CLAP 1.0.0 (478 lignes)
│   ├── clap_integration.rs # ClapPluginInstance et lifecycle
│   ├── clap_gui.rs     # GUI embedding cross-platform (307 lignes)
│   ├── buffer_pool.rs  # Zero-allocation buffer pool (212 lignes)
│   ├── scanner.rs      # Plugin scanner avec cache
│   └── factory.rs      # Plugin factory et descriptors
├── sequencer/
│   ├── timeline.rs     # Timeline, tempo, time signature
│   ├── transport.rs    # Transport controls (play/stop/record)
│   ├── pattern.rs      # Patterns MIDI
│   ├── note.rs         # Notes MIDI avec timing
│   └── recorder.rs     # Recording MIDI en temps réel
├── project/
│   ├── manager.rs      # Gestion des projets (save/load)
│   ├── format.rs       # Format ZIP container
│   └── migration.rs    # Migration de versions
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
    ├── app.rs          # Interface egui/eframe principale
    └── piano_roll.rs   # Piano roll editor

bin/
└── test_clap.rs        # Test program pour CLAP plugins

tests/
├── midi_to_audio.rs    # Tests end-to-end MIDI → Audio
├── latency.rs          # Tests de latence et performance
└── stability.rs        # Tests de stabilité longue durée

benches/
└── audio_benchmarks.rs # Benchmarks Criterion (oscillateurs, latence, etc.)
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

### Phase 1.5 (Robustesse) ✅ TERMINÉ - v0.2.0
- [x] Gestion des périphériques audio/MIDI
- [x] Reconnexion automatique
- [x] Timing MIDI sample-accurate
- [x] Monitoring CPU
- [x] Hygiène DSP et paramètres
- [x] Compatibilité formats audio (F32/I16/U16)
- [x] 66 tests (55 unitaires + 11 intégration)
- [x] Benchmarks Criterion avec rapports HTML
- [x] Documentation tests (TESTING.md)

### Phase 2 (Enrichissement du son) ✅ TERMINÉ - v0.3.0
- [x] Enveloppes ADSR
- [x] Modulation (LFO, vélocité, mod matrix)
- [x] Polyphonie avancée (Poly, Mono, Legato)
- [x] Portamento/glide
- [x] 156 tests (88 nouveaux pour Phase 2)

### Phase 3a (Filtres et effets) ✅ TERMINÉ - v0.4.0
- [x] Filtres (SVF : LP, HP, BP)
- [x] Architecture d'effets (Effect trait, EffectChain)
- [x] Delay (circular buffer, feedback)
- [x] Reverb (Freeverb avec comb/allpass)
- [x] 178 tests (22 nouveaux pour Phase 3a)

### Phase 3b (Dogfooding)
- [ ] Créer une chanson complète avec le DAW
- [ ] UI pour Delay et Reverb
- [ ] Presets pour effets

### Phase 4 (Séquenceur) ✅ TERMINÉ
- [x] Timeline et transport
- [x] Piano roll
- [x] Recording MIDI
- [x] Persistance projets
- [x] Export audio (WAV/FLAC)

### Phase 5 (Plugins CLAP) 🔄 EN COURS
- [x] Infrastructure CLAP complète (~3500 lignes)
- [x] FFI bindings CLAP 1.0.0
- [x] Chargement dynamique de plugins
- [x] Audio processing et lifecycle
- [x] MIDI events et parameter automation
- [x] GUI embedding (cross-platform)
- [x] Buffer pool optimization (zero-allocation)
- [x] Plugin scanner avec cache
- [x] UI Plugin tab
- [ ] Routing audio vers plugins
- [ ] Tests avec vrais plugins (Surge XT, Vital, Airwindows)

### Phase 6+ (Distribution)
- Routing audio avancé
- Mixeur multi-pistes
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
# Tous les tests (178 tests : unitaires + intégration)
cargo test

# Tests unitaires uniquement
cargo test --lib

# Tests d'intégration uniquement
cargo test --tests

# Afficher la sortie des tests (println!)
cargo test -- --nocapture

# Tests spécifiques
cargo test --test midi_to_audio          # Pipeline MIDI → Audio
cargo test --test latency -- --nocapture # Mesures de latence
cargo test --test stability               # Stabilité (court + stress)

# Test de stabilité longue durée (1 heure, marqué comme ignored)
cargo test --test stability -- --ignored --nocapture
```

### Benchmarks

```bash
# Tous les benchmarks Criterion
cargo bench

# Benchmark spécifique
cargo bench oscillator
cargo bench latency

# Test rapide des benchmarks (sans mesures complètes)
cargo bench -- --test

# Voir les rapports HTML (après avoir lancé les benchmarks)
open target/criterion/report/index.html
```

Voir [TESTING.md](TESTING.md) pour la documentation complète des tests.

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
