# TODO - MyMusic DAW

## Phase 1 : MVP - Synthétiseur polyphonique simple ✅ (TERMINÉ)

### Infrastructure de base
- [x] Créer le fichier AGENTS.md avec l'architecture du DAW
- [x] Définir la structure du projet Rust (modules principaux)
- [x] Configurer Cargo.toml avec les dépendances (cpal, midir, egui, ringbuf)

### Audio Engine
- [x] Implémenter l'audio backend avec CPAL (callback temps-réel)
  - [x] Initialisation du device audio
  - [x] Configuration du stream (sample rate auto, f32 stereo)
  - [x] Callback audio sacré (sans allocations, try_lock non-bloquant)
- [x] Créer le système de communication lock-free (ringbuffer)
  - [x] Channel MIDI → Audio
  - [x] Channel UI → Audio
  - [ ] Atomics pour les paramètres (volume, etc.) - À FAIRE

### Synthèse
- [x] Implémenter les oscillateurs de base
  - [x] Sine
  - [x] Square
  - [x] Saw
  - [x] Triangle
- [x] Système de voix (Voice)
- [x] Voice Manager (polyphonie 16 voix)

### MIDI
- [x] Types d'événements MIDI (NoteOn, NoteOff, CC, PitchBend)
- [x] Parser MIDI
- [x] Intégrer MIDI input avec midir
  - [x] Détection des ports MIDI disponibles
  - [x] Connexion au port MIDI (premier port auto)
  - [x] Envoyer les événements dans le ringbuffer

### Interface utilisateur
- [x] Créer l'UI de base avec egui/eframe
  - [x] Fenêtre principale
  - [x] Slider de volume (affiché mais pas connecté)
  - [ ] Sélecteur de forme d'onde - À FAIRE
  - [x] Visualisation des notes actives
  - [x] Clavier virtuel avec touches PC (A W S E D F T G Y H U J K)
  - [x] Clavier virtuel cliquable

### Intégration
- [x] Tester l'intégration complète (MIDI → Synth → Audio out)
  - [x] Test avec clavier MIDI externe (détection auto)
  - [x] Test avec clavier PC virtuel
  - [ ] Test de latence - À MESURER
  - [x] Test de stabilité du callback audio (fonctionnel)

---

## Phase 2 : Enrichissement du son

### Enveloppes
- [ ] Implémenter enveloppe ADSR
  - [ ] Attack
  - [ ] Decay
  - [ ] Sustain
  - [ ] Release
- [ ] Intégrer ADSR dans Voice
- [ ] UI pour contrôles ADSR

### Polyphonie avancée
- [ ] Améliorer le voice stealing (priorité par vélocité/âge)
- [ ] Modes de polyphonie (mono, legato, poly)
- [ ] Portamento/glide

### Modulation
- [ ] LFO (Low Frequency Oscillator)
  - [ ] Formes d'onde LFO
  - [ ] Routing LFO → paramètres
- [ ] Vélocité → intensité
- [ ] Aftertouch support

---

## Phase 3 : Filtres et effets

### Filtres
- [ ] Low-pass filter (Moog-style)
- [ ] High-pass filter
- [ ] Band-pass filter
- [ ] Résonance
- [ ] Cutoff modulation (envelope, LFO)

### Effets
- [ ] Delay
  - [ ] Time
  - [ ] Feedback
  - [ ] Mix
- [ ] Réverbération
  - [ ] Room size
  - [ ] Damping
  - [ ] Mix
- [ ] Distortion/Saturation
- [ ] Chorus

### Architecture effets
- [ ] Chain d'effets
- [ ] Bypass individuel
- [ ] Pré-allocation des buffers d'effets

---

## Phase 4 : Séquenceur

### Timeline
- [ ] Système de timeline (BPM, mesures)
- [ ] Transport (play, stop, pause, loop)
- [ ] Métronome

### Piano Roll
- [ ] Grille temporelle
- [ ] Édition de notes (ajout, suppression, déplacement)
- [ ] Vélocité par note
- [ ] Quantization

### Step Sequencer
- [ ] Grille de steps
- [ ] Patterns
- [ ] Automation

### Recording
- [ ] Enregistrement MIDI en temps réel
- [ ] Overdub
- [ ] Undo/Redo

---

## Phase 5 : Plugins et routing avancé

### Architecture de plugins
- [ ] Trait Plugin générique
- [ ] Chargement dynamique de plugins
- [ ] Preset system
- [ ] Plugin browser

### Routing audio
- [ ] Graph audio flexible
- [ ] Sends/Returns
- [ ] Sidechain

### Mixeur
- [ ] Multi-pistes
- [ ] Pan
- [ ] Solo/Mute
- [ ] VU meters
- [ ] Master bus

---

## Phase 6 : Fonctionnalités avancées

### Performance
- [ ] Optimisation SIMD pour DSP
- [ ] Profiling et métriques
- [ ] Multi-threading pour UI

### Persistance
- [ ] Sauvegarde de projets (format JSON/binaire)
- [ ] Chargement de projets
- [ ] Export audio (WAV, FLAC)
- [ ] Système de presets

### MIDI avancé
- [ ] MIDI learn
- [ ] MIDI mapping customisable
- [ ] MPE (MIDI Polyphonic Expression)

### Visualisation
- [ ] Waveform display
- [ ] Spectrum analyzer
- [ ] Oscilloscope

---

## Backlog / Idées futures

- [ ] Support VST3
- [ ] Support Audio Units (macOS)
- [ ] Mode spectral/granular synthesis
- [ ] Wavetable synthesis
- [ ] Sampling
- [ ] Arrangement view
- [ ] Automation curves
- [ ] Time stretching
- [ ] Pitch shifting
- [ ] Support multi-sortie audio
- [ ] Support JACK (Linux)
- [ ] Scripting (Lua/Python)

---

**Priorité actuelle** : Phase 1 - MVP
**Objectif** : Synthétiseur monophonique fonctionnel avec MIDI et audio out
