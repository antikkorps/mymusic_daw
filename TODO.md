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
  - [x] Atomics pour les paramètres (volume, etc.)

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
  - [x] Slider de volume (connecté via atomics)
  - [x] Visualisation des notes actives
  - [x] Clavier virtuel avec touches PC (A W S E D F T G Y H U J K)
  - [x] Clavier virtuel cliquable

### Intégration

- [x] Tester l'intégration complète (MIDI → Synth → Audio out)
  - [x] Test avec clavier MIDI externe (détection auto)
  - [x] Test avec clavier PC virtuel
  - [x] Test de stabilité du callback audio (fonctionnel)

---

## Phase 1.5 : Robustesse et UX de base ✅ (TERMINÉ)

**Objectif** : Rendre le DAW utilisable par d'autres personnes
**Release** : v0.2.0 🎉

### Gestion des périphériques audio/MIDI

- [x] Énumération des périphériques disponibles
  - [x] Lister périphériques audio (entrée/sortie)
  - [x] Lister ports MIDI
  - [x] Stocker infos périphériques (nom, ID, statut)
- [x] UI de sélection
  - [x] Menu déroulant pour sélection entrée MIDI
  - [x] Menu déroulant pour sélection sortie audio
  - [x] Refresh de la liste des périphériques
  - [x] Sélecteur de forme d'onde (déplacé depuis Phase 1)
- [x] Reconnexion automatique MIDI
  - [x] Détection déconnexion périphérique MIDI
  - [x] Tentative de reconnexion avec backoff exponentiel
  - [x] Fallback sur périphérique MIDI par défaut
  - [x] Journalisation hors callback, notification UI non-bloquante
- [x] Gestion des erreurs Audio (CPAL)
  - [x] Handler d'erreurs CPAL (callback d'erreur du stream) ✅
  - [x] Détection des erreurs de stream audio ✅
  - [x] Notification UI non-bloquante des erreurs audio ✅
  - [x] AtomicDeviceStatus pour suivre l'état de la connexion audio ✅
  - **Note**: Reconnexion automatique impossible sur macOS (CoreAudio Stream n'est pas Send/Sync)
  - **Solution**: L'error callback détecte les erreurs et notifie l'UI. La reconnexion manuelle est possible au redémarrage.

### Timing et précision (audio/MIDI)

- [x] Introduire `MidiEventTimed { event, samples_from_now: u32 }`
- [x] Timestamp relatif côté thread MIDI (quantification en samples)
  - Infrastructure complète avec module `AudioTiming`
  - Conversion microsecondes → samples implémentée
  - Pour l'instant : `samples_from_now = 0` (traitement immédiat)
  - TODO futur : utiliser les timestamps midir pour calcul précis
- [x] Scheduling sample-accurate dans le callback audio
  - Infrastructure de scheduling implémentée
  - Fonction `process_midi_event` avec support timing
  - TODO futur : queue d'événements pour scheduling différé
- [x] Dimensionner le ringbuffer SPSC pour la pire rafale MIDI
  - Capacité : 512 événements (>500ms buffer au max MIDI rate)
  - Documentation détaillée du dimensionnement
  - Tests unitaires du module timing (6 tests)

### Monitoring de la charge CPU

- [x] Mesure du temps callback audio (échantillonnée)
  - [x] Mesurer 1/N callbacks (N configurable) pour limiter l'overhead
  - [x] Accumuler temps total et compteurs dans des atomics
  - [x] Calcul CPU% = callback_time / available_time
  - [x] Publication vers UI via atomic ou ringbuffer (hors allocations)
  - [x] UI du monitoring
    - [x] Indicateur CPU dans la barre de statut
    - [x] Couleur : vert (<50%), orange (50-75%), rouge (>75%)
    - [x] Warning si surcharge détectée
  - [ ] **À RETESTER** : Le monitoring fonctionne mais impossible de charger le CPU avec juste le synthé. À revalider en Phase 3+ avec filtres/effets/plugins

### Gestion des erreurs UI

- [x] Barre de statut
  - [x] Composant UI en bas de fenêtre
  - [x] Affichage messages d'erreur/warning
  - [x] Queue de notifications (ringbuffer)
- [x] Types d'erreurs à gérer
  - [x] Échec connexion MIDI
  - [ ] Déconnexion carte son (CPAL stream error handler - optionnel)
  - [x] Surcharge CPU
  - [x] Errors génériques

### Hygiène DSP et paramètres

- [x] Anti-dénormaux (flush-to-zero ou DC offset minuscule)
- [x] Clamp ou soft-saturation (ex. tanh) sur la sortie [-1,1]
- [x] Smoothing 1-pole pour `volume` et autres paramètres continus
- [x] Représenter `f32` en `AtomicU32` via `to_bits/from_bits` (éviter lib)
- [x] Oscillateurs bandlimit: Saw/Square via PolyBLEP (réduction d'aliasing)

### Compatibilité formats/buffers CPAL

- [x] Support `i16` et `u16` en entrée/sortie (conversion sans allocation) ✅
  - [x] Module `format_conversion` avec conversions f32 ↔ i16 ↔ u16
  - [x] Tests unitaires (8 tests) couvrant conversions, roundtrip, clamping
  - [x] Fonction `write_mono_to_interleaved_frame` pour écriture optimisée
  - [x] Support automatique via trait `FromSample<f32>` de CPAL
- [x] Gérer interleaved vs non-interleaved ✅
  - [x] Système générique qui gère interleaved (format le plus courant)
  - [x] Détection automatique du format via `sample_format()`
  - [x] `AudioEngine::build_stream<T>` générique pour tous les formats
  - **Note**: Non-interleaved est rare et non supporté (peut être ajouté si besoin)
- [x] Architecture multi-format ✅
  - [x] Détection du format device avec `SampleFormat`
  - [x] Match sur F32/I16/U16 et création du stream approprié
  - [x] Callback audio unique qui fonctionne avec tous les formats
  - [x] Génération interne en f32, conversion automatique à la sortie
- [ ] Tests de conformité sur plusieurs hosts (CoreAudio/WASAPI/ALSA)
  - [ ] Test sur macOS (CoreAudio) - disponible localement
  - [ ] Test sur Windows (WASAPI) - nécessite VM ou machine Windows
  - [ ] Test sur Linux (ALSA/PulseAudio) - nécessite VM ou machine Linux

### Tests et CI/CD

- [ ] Setup CI (GitHub Actions) - **À FAIRE PLUS TARD (après Phase 1.5)**
  - [ ] Créer .github/workflows/test.yml
  - [ ] Tests unitaires auto sur chaque commit
  - [ ] Cargo clippy (linter)
  - [ ] Cargo fmt check (formatting)
  - [ ] Badge de statut dans README
- [x] Benchmarks avec Criterion (dev-dependency) ✅
  - [x] Setup Criterion avec HTML reports
  - [x] Benchmarks oscillateurs (toutes waveforms)
  - [x] Benchmarks voice processing (polyphonie 1-16 voix)
  - [x] Benchmarks MIDI processing
  - [x] Benchmarks latence MIDI → Audio
  - [x] Benchmarks timing conversions
  - [x] Benchmarks filtres (6 benchmarks - types, resonance, modulation, polyphony)
- [x] Tests unitaires
  - [x] Tests oscillateurs (fréquence, amplitude, phase) - 8 tests
  - [x] Tests Voice Manager (allocation, voice stealing) - 8 tests
  - [x] Tests MIDI parsing - 11 tests
  - [x] Tests anti-dénormaux et smoothing des paramètres - 4 tests
  - [x] Tests timing audio (AudioTiming module) - 6 tests
  - [x] Tests CPU monitoring - 5 tests
  - [x] Tests reconnexion automatique - 3 tests
  - [x] Tests notifications - 3 tests
  - [x] Tests format conversion - 8 tests
  - **Total : 55 tests unitaires ✅**
- [x] Tests d'intégration ✅
  - [x] Test MIDI → Audio end-to-end (4 tests)
  - [x] Test latency benchmark (< 10ms target) - **ATTEINT: ~200ns NoteOn + 69µs buffer** ⚡
  - [x] Test stabilité court (5 min) - **990M samples, 0 crash** ✅
  - [x] Test stabilité stress polyphonique (30s, 16 voix)
  - [x] Test stabilité rapid notes (10,000 cycles)
  - [x] Test stabilité long (1h) - disponible avec `--ignored`
  - **Total : 11 tests d'intégration ✅**
- [x] Documentation des tests ✅
  - [x] TESTING.md avec instructions complètes
  - [x] Métriques de performance documentées
  - [x] Commandes pour lancer tests et benchmarks

**Total tests : 141 tests passent** 🎉 (55 tests Phase 1.5 + 13 tests Command Pattern + 10 tests ADSR + 11 tests LFO + 2 tests Voice Stealing + 14 tests Polyphony Modes + 9 tests Portamento + 18 tests Filter + 4 tests Filter Integration + 1 test Modulation Matrix + 4 tests Voice)

### Documentation et communauté - **REPORTÉ POST-v1.0** ⏭️

---

## Phase 2 : Panning & Modulation Sources (Planned)

### Goals

- Expand panning capabilities (global pan + per‑voice spread).
- Extend modulation sources beyond Velocity/Aftertouch/Envelope/LFO0.
- Prepare for multiple LFOs without runtime allocations.
- Keep audio callback RT‑safe (no allocs, no I/O, no blocking).

### Panning Enhancements

- [ ] Global Pan parameter
  - [ ] Add `Command::SetPan(f32)` (range `[-1.0, 1.0]`).
  - [ ] Store `global_pan` in `VoiceManager` and propagate to voices (`Voice.pan`).
  - [ ] Add smoothing (One‑pole) for pan to avoid zipper noise (like volume).
  - [ ] UI: Synth tab slider “Pan” with undo/redo (`SetPanCommand`).
  - [ ] Tests: constant‑power panning (energy roughly stable at L/C/R).

- [ ] Pan Spread across polyphony
  - [ ] Add `Command::SetPanSpread(f32)` (range `[0.0, 1.0]`).
  - [ ] On `note_on`, assign per‑voice base pan in `[-spread, +spread]` (e.g., even distribution or simple alternating pattern).
  - [ ] UI: Synth tab slider “Pan Spread”.
  - [ ] Tests: distribution across N voices, ensures stereo widening without clipping.

### Modulation Sources Extensions

- [ ] Add common MIDI sources to `ModSource`
  - [ ] `ModSource::ModWheel` (CC1), `ModSource::Expression` (CC11), `ModSource::PitchBend`.
  - [ ] (Optional) `ModSource::Cc(u8)` for generic CC mapping (future‑proof).

- [ ] Engine handling (callback‑safe)
  - [ ] In `process_midi_event`, handle `ControlChange` (CC1/CC11) and `PitchBend`.
  - [ ] Normalize to `[0.0, 1.0]` (or `[-1.0, 1.0]` where appropriate) and store in `VoiceManager` atomics/fields.
  - [ ] Expose these normalized values to modulation evaluation without locks.

- [ ] Modulation Matrix API
  - [ ] Introduce a pre‑allocated `ModValues` struct passed to `apply()` containing: `velocity, aftertouch, envelope, pitch_bend, mod_wheel, expression, lfo: [f32; MAX_LFOS]`.
  - [ ] Keep current `apply` temporarily (compat) or migrate all call‑sites.
  - [ ] Bounds and clamping consistent with current behavior.

- [ ] UI updates (Modulation tab)
  - [ ] Add sources in the ComboBox: “ModWheel”, “Expression”, “Pitch Bend”.
  - [ ] Increase visible slots from 4 → 8 to match `MAX_ROUTINGS` (still pre‑allocated, no runtime allocs).
  - [ ] Tooltips indicating ranges and semantics (pitch amount = semitones; pan = −1..1; amp adds to 1.0 and clamps ≥ 0).

### Multiple LFOs (MVP)

- [ ] Support `MAX_LFOS = 2..4`
  - [ ] Store `[Lfo; MAX_LFOS]` in `Voice` (pre‑allocated) with identical API as current LFO.
  - [ ] Compute per‑sample LFO values once per voice and pass into `ModValues`.
  - [ ] Update `ModSource::Lfo(i)` to read `lfo[i]` (ignore out‑of‑range safely).

- [ ] UI for multiple LFOs
  - [ ] Add selector for LFO index (1..MAX_LFOS) when editing LFO params.
  - [ ] Allow routing selection to `Lfo(0..MAX_LFOS-1)` in the matrix.

### DSP/RT Safety

- [ ] No allocations or logging in callback; keep `try_lock` usage and ringbuffers.
- [ ] Smoothing for continuous params (pan, spread‑derived changes) to avoid zipper noise.
- [ ] Clamp outputs: amplitude ≥ 0, pan in [−1, 1], maintain constant‑power panning law.

### Tests

- [ ] Panning: constant‑power behavior and clamping.
- [ ] Pan Spread: stereo distribution across multiple voices.
- [ ] Sources: end‑to‑end routing for CC1/CC11/PitchBend to Pitch/Amplitude/Pan destinations.
- [ ] Multi‑LFO: ensure `Lfo(1)` affects destinations independently from `Lfo(0)`; bounds respected.
- [ ] Backward compatibility: legacy LFO destination and existing single‑LFO paths keep working.

### Acceptance Criteria

- Global pan + spread adjustable from UI with smooth, click‑free audio.
- New sources (ModWheel/Expression/PitchBend) routable in the matrix with predictable ranges.
- Two LFOs minimum routable independently; UI exposes routing and basic params.
- All changes respect real‑time constraints (no allocs/locks contention) and pass added tests.

**Décision** : Trop tôt pour ouvrir aux contributeurs externes. L'API et l'architecture vont encore beaucoup évoluer jusqu'à v1.0 (Phase 4). Cette section sera réactivée après avoir atteint le milestone v1.0.0 avec un DAW fonctionnel et stable.

**Reporté à** : Phase 6a (Performance et stabilité) - Quand le projet sera "production-ready"

- [ ] Documentation cargo doc des modules principaux
- [ ] README.md avec screenshots et getting started
- [ ] CONTRIBUTING.md (guidelines pour contributeurs)
- [ ] GitHub repo public avec issues templates
- [ ] Discord/Forum setup (optionnel, si communauté intéressée)
- [ ] Documentation utilisateur (manuel, FAQ)

---

## Phase 2 : Enrichissement du son 🎛️ ✅ (TERMINÉ)

**Objectif** : Synth expressif avec modulation
**Release** : v0.3.0 🎉

**⚠️ ARCHITECTURE CRITIQUE** : Implémenter le **Command Pattern** dès cette phase pour l'Undo/Redo (voir "Décisions Architecturales"). Toutes les modifications de paramètres (ADSR, LFO, etc.) doivent passer par des `UndoableCommand`.

### Command Pattern & Undo/Redo ✅ (TERMINÉ)

- [x] Implémenter le trait `UndoableCommand`
- [x] Créer le `CommandManager` avec undo/redo stacks
- [x] Implémenter `SetVolumeCommand` et `SetWaveformCommand` (premiers params)
- [x] Intégrer Ctrl+Z / Ctrl+Y dans l'UI
- [x] Tests unitaires (13 tests, 68 total avec intégration)
- [x] Documentation du pattern (doc/COMMAND_PATTERN.md)
- [x] Tester avec les paramètres ADSR ✅
- [x] Tester avec les paramètres LFO ✅

### Enveloppes ✅ (TERMINÉ)

- [x] Implémenter enveloppe ADSR
  - [x] Attack
  - [x] Decay
  - [x] Sustain
  - [x] Release
- [x] Intégrer ADSR dans Voice
- [x] UI pour contrôles ADSR (4 sliders avec undo/redo)
- [x] Tests unitaires ADSR (10 tests - timing, courbes, retriggering)

### Polyphonie avancée ✅ (TERMINÉ)

- [x] Améliorer le voice stealing (priorité par âge + releasing voices d'abord)
- [x] Modes de polyphonie (mono, legato, poly)
  - [x] Enum `PolyMode` (Poly, Mono, Legato)
  - [x] Implémentation dans `VoiceManager` (3 méthodes de note_on)
  - [x] Mode Poly : polyphonie complète (comportement par défaut)
  - [x] Mode Mono : monophonique avec retriggering de l'enveloppe
  - [x] Mode Legato : transitions de pitch fluides sans retriggering
  - [x] Méthode `force_stop()` pour couper les voix immédiatement (mono mode)
  - [x] UI avec ComboBox de sélection
  - [x] Intégration avec Command Pattern (undo/redo)
  - [x] Tests unitaires (14 tests - 11 voice_manager + 3 poly_mode)
- [x] Portamento/glide ✅ (TERMINÉ)
  - [x] Module `portamento.rs` avec `PortamentoGlide` et `PortamentoParams`
  - [x] Utilise `OnePoleSmoother` pour des glides fluides
  - [x] Intégration dans Voice (transitions de fréquence progressives)
  - [x] Méthode `force_stop()` pour compatibilité mono/legato
  - [x] Portamento + LFO combinés (portamento → base freq → LFO modulation)
  - [x] Command Pattern : `SetPortamentoCommand` avec undo/redo et merge
  - [x] UI : Slider "Glide Time" (0-2 secondes)
  - [x] Tests unitaires (9 tests couvrant tous les cas d'usage)
  - [x] Compatible tous les modes (Poly, Mono, Legato)

### Modulation ✅ (TERMINÉ)

- [x] LFO (Low Frequency Oscillator)
  - [x] Formes d'onde LFO (sine, square, saw, triangle)
  - [x] Routing LFO → paramètres (pitch vibrato, volume tremolo)
  - [x] UI pour contrôler le LFO (waveform, rate, depth, destination)
  - [x] Intégration avec Command Pattern (undo/redo)
  - [x] Tests unitaires LFO (11 tests)
  - [ ] Sync LFO au tempo (optionnel - Phase 4+)
  - [x] Vélocité → intensité (étendu via matrice de modulation)
  - [x] Aftertouch (Channel Pressure) support

### Architecture de modulation avancée

- [ ] Matrice de modulation générique
  - [x] MVP: matrice pré‑allouée (8 slots) sans allocations runtime
  - [x] Sources (MVP): LFO(0), Vélocité, Aftertouch
  - [x] Destinations (MVP): OscillatorPitch(0), Amplitude
  - [x] Assignation source → destination + amount [-1..1] (semitones pour Pitch)
  - [x] UI minimale (4 slots) + commandes `SetModRouting`/`ClearModRouting`
  - [x] Étendre sources (Enveloppes)
  - [x] Étendre destinations (Pan)
  - [x] Étendre destinations (FilterCutoff) ✅
  - [ ] Éditeur UI avancé (drag & drop, presets)

---

## Phase 2.5 : UX Design 🎨

**Objectif** : Préparer l'UI avant développement intensif
**Durée** : 1-2 semaines

### Wireframes et mockups

- [ ] Wireframe écran principal
- [ ] Wireframe piano roll (Phase 4)
- [ ] Wireframe mixer (Phase 5)
- [ ] Mockups haute fidélité (Figma/Sketch)

### Design system

- [ ] Palette de couleurs
- [ ] Typographie
- [ ] Composants UI (boutons, sliders, knobs)
- [ ] Iconographie
- [ ] Dark/Light themes

### User flows

- [ ] Flow : Nouveau projet → Composition
- [ ] Flow : Charger plugin → Tweaking
- [ ] Flow : Enregistrement MIDI → Export audio

---

## Phase 3a : Filtres et effets essentiels 🔊 ✅ (TERMINÉ)

**Objectif** : 1 filtre + 2 effets de qualité
**Release** : v0.4.0 🎉
**Durée** : 3-4 semaines

### Filtres ✅ (TERMINÉ)

- [x] State Variable Filter (Chamberlin) - 4 modes
  - [x] Implémentation algorithme State Variable Filter (12dB/octave)
  - [x] 4 types de filtres : LowPass, HighPass, BandPass, Notch
  - [x] Cutoff control (20Hz - 8kHz, avec smoothing)
  - [x] Résonance control (Q 0.5 - 20.0, self-oscillation capable)
  - [x] Cutoff modulation via matrice (envelope, LFO) avec `process_modulated()`
  - [x] Command Pattern : `SetFilterCommand` avec undo/redo
  - [x] UI complète (enable/disable, type selector, cutoff/resonance sliders)
  - [x] Tests unitaires (18 tests - frequency response, stability, resonance)
  - [x] Tests d'intégration (4 tests - envelope/LFO modulation, bypass)
  - [x] Benchmarks performance (6 benchmarks - ~11 ns/sample, excellent scaling)
  - [x] Documentation complète (commentaires, formules mathématiques)

### Effets prioritaires ✅ (TERMINÉ)

- [x] Delay ✅
  - [x] Delay line (buffer circulaire pré-alloué jusqu'à 1 seconde)
  - [x] Time control (0-1000ms avec smoothing)
  - [x] Feedback control (0-0.99 avec stabilité garantie)
  - [x] Mix (dry/wet 0-1)
  - [x] Tests (12 tests - pas de clics, feedback stable, circular buffer)
  - [x] Latency reporting précis
- [x] Réverbération (Freeverb) ✅
  - [x] Freeverb simplifié (4 comb + 2 allpass filters)
  - [x] Room size (0-1 avec scaling pour sample rate)
  - [x] Damping (low-pass filtering dans feedback loop)
  - [x] Mix (dry/wet 0-1)
  - [x] Tests (10 tests - pas de distorsion, decay tail, parameter changes)
  - [x] Tunings optimisés pour 44.1kHz

### Architecture effets ✅ (TERMINÉ)

- [x] Trait Effect générique (avec process, reset, enable, latency, name)
- [x] EffectChain (Vec pré-allouée avec capacité 4 effets)
  - [x] Wrappers : FilterEffect, DelayEffect, ReverbEffect
  - [x] Intégration dans Voice (pipeline: Oscillator → Filter → EffectChain → Envelope → Pan)
- [x] Bypass individuel par effet (click-free)
- [x] Latency reporting (méthode latency_samples())
- [x] Tests architecture (15 tests - chain, bypass, latency, multiple effects)

---

## Phase 3b : Dogfooding - Performance Live 🐕 ✅ (TERMINÉ)

**Objectif** : Tester le synthé en conditions réelles avec une performance live
**Durée** : 1 semaine
**Note** : Pas encore de séquenceur/enregistrement, donc focus sur jam session live

### Performance Live

- [x] Créer une performance/jam session live (5-10 min) avec le synthé
  - [x] Jouer avec MIDI controller ou clavier virtuel
  - [x] Tester tous les paramètres (ADSR, LFO, Filtres, Effets)
  - [x] Tweaking en temps réel
  - [x] Tester les modes polyphonie (Poly, Mono, Legato)
  - [x] Enregistrer en audio (via DAW externe ou capture système)
- [x] Identifier bugs UX et problèmes de workflow
- [x] Lister features manquantes critiques pour l'expressivité
- [x] Documenter l'expérience utilisateur

### Polissage

- [x] Fixer bugs critiques découverts
- [x] Améliorer qualité audio des filtres/effets
- [x] Optimiser performance si nécessaire
- [x] Améliorer réactivité des contrôles UI
- [x] Documenter limitations connues

---

## Phase 3.5 : Sampling 🎵

**Objectif** : Support de samples audio pour enrichir les possibilités sonores
**Release** : v0.5.0
**Durée** : 2-3 semaines
**Justification** : Nécessaire pour créer un morceau complet (Phase 4 - dogfooding réel)

**🎯 Plan de finalisation** (2-3 jours restants) :
1. ✅ Loop points + Preview UI (FAIT)
2. ✅ Suppression de samples (UI) (FAIT)
3. ✅ Reverse playback mode (FAIT)
4. ✅ Pitch offset (coarse tune) (FAIT)
5. 🔲 **Persistance** (Save/Load sample banks) - CRITIQUE pour Phase 4
6. 🔲 Tests d'intégration
7. 🔲 Release v0.5.0 🎉

### Lecteur de samples

- [x] Chargement de fichiers audio (WAV, FLAC)
  - [x] Intégration crate `hound` (WAV) et `claxon` (FLAC)
  - [x] Parsing des metadata (sample rate, channels, bit depth)
  - [x] Resampling automatique si sample rate ≠ audio engine
  - [x] Conversion mono/stereo
- [x] Support MP3
  - [x] Intégration crate `symphonia` (support multi-formats)
  - [x] Parsing MP3 metadata (bitrate, duration, tags)
  - [x] Décodage MP3 vers f32 avec resampling intégré
  - [x] Gestion des formats avec ou sans VBR (Variable Bitrate)
  - [x] Tests de compatibilité avec différents encodages MP3
  - [x] File picker UI updated to accept .mp3 files (macOS fix)
- [x] Structure Sample
  - [x] Buffer pré-alloué (Vec<f32>)
  - [x] Sample rate, durée, nom
  - [x] Loop points (start, end) ✅
  - [ ] Metadata (BPM original si disponible)

### Sampler Engine

- [x] Playback de samples
  - [x] Lecture linéaire avec interpolation (linear ou cubic)
  - [x] Pitch shifting via resampling (semitones MIDI)
  - [x] Volume et pan par sample
  - [x] Mode one-shot vs loop ✅
  - [x] Reverse playback mode ✅
  - [x] Pitch offset (coarse tune -12 à +12 semitones) ✅
  - [x] ADSR par sample (optionnel - peut réutiliser Envelope existant)
- [x] Sampler Voice
  - [x] Similaire à Voice mais lit depuis buffer au lieu d'oscillateur
  - [x] Support polyphonie (plusieurs samples simultanés)
  - [ ] Note-to-sample mapping (ex: kick sur C1, snare sur D1)
  - [x] Velocity → volume scaling
- [x] Intégration avec VoiceManager
  - [x] Choix synth vs sampler par note/channel
  - [ ] Ou: mode hybride (layers synth + sample)

### UI Sampling

- [x] Browser de samples ✅ (MVP)
  - [x] Liste des samples chargés ✅
  - [x] Bouton "Load Sample" (file picker) ✅
  - [x] Bouton "Delete" pour supprimer un sample ✅
  - [x] Preview audio (playback du sample) ✅
  - [x] Affichage waveform avec loop markers ✅
- [ ] Mapping MIDI → Sample (partiellement)
  - [x] UI basique pour assigner samples aux notes (text input + bouton)
  - [ ] Table complète note MIDI → sample assigné
  - [ ] UI drag & drop avancée
  - [ ] Indication visuelle des notes assignées sur clavier
- [x] Contrôles par sample ✅
  - [x] Volume, Pan ✅
  - [x] Pitch offset (coarse tuning -12 à +12 semitones) ✅
  - [x] Loop on/off ✅
  - [x] Mode one-shot/loop ✅
  - [x] Loop points (start/end) avec affichage temps ✅
  - [x] Reverse playback ✅

### Persistance 🔲 (CRITIQUE pour Phase 4)

- [ ] Save/Load sample banks
  - [ ] Format JSON pour mapping (note → sample path + params)
  - [ ] Sauvegarder : volume, pan, loop_mode, loop_start, loop_end, reverse, pitch_offset
  - [ ] Chemins relatifs au projet (préparation Phase 4)
  - [ ] Boutons UI : "Save Bank" / "Load Bank"
  - [ ] Command Pattern pour undo/redo des assignations (optionnel)

### Tests

- [x] Tests unitaires sampler ✅ (6 tests)
  - [x] Loop default values ✅
  - [x] Loop mode Forward (keeps voice active) ✅
  - [x] Loop mode Off (stops at end) ✅
  - [x] Loop points within bounds ✅
  - [x] Loop with pitch shift ✅
  - [x] Loop produces continuous audio ✅
  - [x] Format detection (WAV, FLAC, MP3) ✅
- [ ] Tests d'intégration (à compléter)
  - [ ] MIDI → Sampler end-to-end
  - [ ] Chargement WAV/FLAC/MP3 (formats testés)
  - [ ] Memory safety (pas de leaks)

---

## Phase 4 : Séquenceur 🎹

**Objectif** : DAW complet avec séquenceur fonctionnel + création d'un morceau
**Release** : v1.0.0 🎉 (MILESTONE MAJEUR)
**Durée** : 6-8 semaines

**⚠️ ARCHITECTURE CRITIQUE** : Format de projet en **ZIP container hybride** (voir "Décisions Architecturales"). JSON/RON pour l'état, binaire pour les samples, extensible et versionné.

**🎯 Dogfooding réel** : À la fin de cette phase, créer un morceau complet (2-3 min) avec :
- Séquences MIDI (synthé + modulation)
- Samples (drums, percussions)
- Automation des effets
- Export audio final

### Timeline

- [ ] Système de timeline (BPM, mesures, signature)
- [ ] Transport (play, stop, pause, loop)
- [ ] Métronome
- [ ] Position cursor avec snap-to-grid

### Piano Roll

- [ ] Grille temporelle (bars, beats, subdivisions)
- [ ] Édition de notes
  - [ ] Ajout de notes (clic + drag)
  - [ ] Suppression de notes (delete)
  - [ ] Déplacement de notes (drag)
  - [ ] Redimensionnement (durée)
- [ ] Vélocité par note
- [ ] Quantization (1/4, 1/8, 1/16, 1/32)
- [ ] Selection multiple (shift + clic)

### Step Sequencer (optionnel Phase 4)

- [ ] Grille de steps
- [ ] Patterns
- [ ] Automation basique

### Recording

- [ ] Enregistrement MIDI en temps réel
- [ ] Overdub
- [ ] Undo/Redo (command pattern)
- [ ] Count-in avant recording

### Synchronisation

- [ ] MIDI Clock
  - [ ] Envoi MIDI Clock (Master mode)
  - [ ] Réception MIDI Clock (Slave mode)
  - [ ] Sync avec boîtes à rythmes/séquenceurs externes
- [ ] Support Ableton Link (optionnel)

### Persistance projets

- [ ] Format de projet (ZIP container - voir "Décisions Architecturales")
  - [ ] Structure ZIP avec manifest.json, project.ron, tracks/*, audio/*
  - [ ] Serialization/Deserialization avec serde
  - [ ] Support versionning du format (migration)
  - [ ] Compression automatique via ZIP
- [ ] Save project (.mymusic)
- [ ] Load project avec validation et migration de version
- [ ] Export audio (WAV, FLAC)
- [ ] Auto-save toutes les 5 min (en arrière-plan)

---

## Phase 5 : Plugins CLAP et routing 🔌

**Objectif** : Support plugins tiers (CLAP) + routing flexible
**Release** : v1.1.0
**Durée** : 4-6 semaines

### Architecture de plugins (Foundation)

- [ ] Trait Plugin générique
  - [ ] Interface process (buffer audio)
  - [ ] Gestion des paramètres (get/set)
  - [ ] Save/Load state (serialization)
  - [ ] Latency reporting
  - [ ] Category (Instrument, Effect, etc.)
- [ ] Plugin Scanner
  - [ ] Scan directories pour plugins (.clap)
  - [ ] Validation (ne pas charger plugins cassés)
  - [ ] Blacklist persistante (JSON)
  - [ ] Cache des plugins scannés (accélération startup)
- [ ] Plugin Host (moteur)
  - [ ] Chargement dynamique (dll/so/dylib)
  - [ ] Instance management (plusieurs instances du même plugin)
  - [ ] Thread-safe parameter changes (ringbuffer UI → Audio)
  - [ ] Bypass system (sans clics)

### Support CLAP (apprentissage)

- [ ] Intégration crate `clack`
  - [ ] CLAP host implementation
  - [ ] Plugin discovery (.clap files)
  - [ ] Parameter automation (read/write)
  - [ ] Audio process callback
- [ ] GUI CLAP
  - [ ] Embedding fenêtre native CLAP
  - [ ] Gestion événements clavier/souris
  - [ ] Resize handling
- [ ] Preset system CLAP
  - [ ] Load/Save presets
  - [ ] Browser de presets dans UI
- [ ] Tests avec plugins CLAP
  - [ ] Surge XT (synth)
  - [ ] Airwindows (effets)
  - [ ] Vital (synth)

### Routing audio

- [ ] Graph audio flexible (node-based)
  - [ ] Nodes : Instruments, Effets, Mixeur
  - [ ] Connections : Source → Destination
  - [ ] Gestion cycles (détection + error)
- [ ] Sends/Returns (bus auxiliaire)
- [ ] Sidechain routing

### Mixeur

- [ ] Multi-pistes (4-16 tracks)
- [ ] Pan (stéréo)
- [ ] Solo/Mute par track
- [ ] VU meters par track
- [ ] Master bus avec limiter
- [ ] Faders avec automation

---

## Phase 6a : Performance et stabilité ⚡

**Objectif** : DAW optimisé et production-ready
**Release** : v1.2.0
**Durée** : 3-4 semaines

### Performance

- [ ] Optimisation SIMD pour DSP
  - [ ] Vectorisation oscillateurs
  - [ ] Vectorisation filtres
  - [ ] Benchmarks avant/après
- [ ] Profiling approfondi
  - [ ] Flamegraphs callback audio
  - [ ] Identifier bottlenecks
  - [ ] Mesurer allocations cachées
- [ ] Multi-threading pour UI (si nécessaire)

### Stabilité

- [ ] Tests de charge
  - [ ] 16 voix simultanées + 4 effets
  - [ ] Séquence complexe (1000+ notes)
  - [ ] Run 24h sans crash
- [ ] Memory leaks detection (Valgrind/AddressSanitizer)
- [ ] Fuzzing MIDI parser
- [ ] Edge cases handling

### Visualisation

- [ ] Waveform display (oscilloscope simple)
- [ ] Spectrum analyzer (FFT)
- [ ] VU meters améliorés

### Documentation et ouverture communauté (ACTIVÉ ICI)

Cette section était initialement en Phase 1.5 mais a été reportée car trop prématurée.
À ce stade (post v1.2.0), le DAW est stable et production-ready, donc prêt pour la communauté.

- [ ] Documentation technique (cargo doc)
  - [ ] Documentation complète des modules publics
  - [ ] Examples d'utilisation dans la doc
  - [ ] Architecture documentation (diagrammes)
- [ ] Documentation utilisateur
  - [ ] README.md avec screenshots et getting started
  - [ ] Manuel utilisateur (wiki/mdbook)
  - [ ] Video tutorials (YouTube)
  - [ ] FAQ et troubleshooting guide
- [ ] Ouverture communauté
  - [ ] CONTRIBUTING.md (guidelines pour contributeurs)
  - [ ] Code of Conduct
  - [ ] GitHub repo public avec issues templates
  - [ ] Discord/Forum setup (si demande communauté)
  - [ ] Roadmap publique et transparente

---

## Phase 6b : VST3 Support (OPTIONNEL) 🎚️

**Objectif** : Compatibilité écosystème VST3 existant
**Release** : v1.5.0
**Durée** : 12-16 semaines ⚠️ (complexe)
**Note** : Cette phase peut être reportée ou remplacée par focus CLAP

### Support VST3 (plugins tiers)

- [ ] VST3 SDK integration
  - [ ] Bindings Rust (vst3-sys ou custom)
  - [ ] Bridge FFI Rust ↔ C++
  - [ ] Gestion mémoire safe (wrapper safe autour API C++)
  - [ ] Tests unitaires FFI
- [ ] VST3 Host
  - [ ] Chargement plugins VST3 (.vst3)
  - [ ] Parameter automation VST3
  - [ ] Process audio VST3
  - [ ] Latency compensation
  - [ ] Sample-accurate automation
- [ ] GUI VST3
  - [ ] Embedding fenêtre native VST3 (Windows HWND)
  - [ ] Linux (X11/Wayland)
  - [ ] macOS (NSView)
  - [ ] Redimensionnement et focus
  - [ ] Gestion événements UI (clavier/souris)
- [ ] Validation et stabilité
  - [ ] Gestion crashes plugins (process isolation si possible)
  - [ ] Blacklist plugins problématiques
  - [ ] Tests avec plugins populaires
    - [ ] Serum
    - [ ] Vital
    - [ ] Diva
    - [ ] FabFilter Pro-Q3
  - [ ] Timeout detection (plugin freeze)

### Audio Units (macOS uniquement)

- [ ] AU support (si ciblage macOS sérieux)
  - [ ] AudioUnit framework bindings
  - [ ] AU host implementation
  - [ ] Tests avec Logic plugins
  - [ ] AUv3 support (optionnel)

### MIDI avancé

- [ ] MIDI learn (clic paramètre → assign CC)
- [ ] MIDI mapping customisable (save/load)
- [ ] MPE (MIDI Polyphonic Expression)
  - [ ] Per-note pitch bend
  - [ ] Per-note pressure
  - [ ] Per-note brightness

---

## Phase 7 : Frontend Tauri et Monétisation 🎨💰

**Objectif** : UI moderne, distribution et système de licensing
**Release** : v2.0.0
**Durée** : 6-8 semaines (étendu pour licensing)

**⚠️ ARCHITECTURE CRITIQUE** : Gestion de l'état global avec **Commands & Events** (voir "Décisions Architecturales"). Le moteur audio est la source de vérité, l'UI est une vue. Redux optionnel côté frontend.

### Architecture Tauri

- [ ] Setup projet Tauri
  - [ ] Configuration Tauri.conf.json
  - [ ] Choix du framework frontend (React/Vue/Svelte recommandé)
  - [ ] Configuration du build system (vite/webpack)
  - [ ] Migration graduelle depuis egui
- [ ] Bridge Rust ↔ Frontend
  - [ ] API Tauri Commands pour contrôle du moteur audio
  - [ ] Event system pour streaming des données audio/MIDI vers UI
  - [ ] État partagé (Tauri State) pour paramètres du synthé
  - [ ] IPC performance optimization (batch updates)

### Système de licensing et activation 🔐

- [ ] Architecture licensing
  - [ ] Choix du système (Gumroad, Paddle, LemonSqueezy, custom)
  - [ ] Licensing server (API REST)
  - [ ] Base de données licenses (PostgreSQL/SQLite)
  - [ ] Génération de clés de licence (algorithme sécurisé)
- [ ] Activation online
  - [ ] Écran d'activation dans l'app
  - [ ] Validation clé de licence (API call)
  - [ ] Stockage sécurisé de la licence localement (encrypted)
  - [ ] Machine fingerprint (hardware ID)
  - [ ] Limite d'activations (ex: 3 machines max)
- [ ] Gestion des désactivations
  - [ ] Désactivation depuis l'app
  - [ ] Portail web utilisateur (gérer ses activations)
  - [ ] Reset des activations (support client)
- [ ] Mode offline/grace period
  - [ ] Validation locale si pas d'internet
  - [ ] Grace period de 30 jours après activation
  - [ ] Re-validation périodique (tous les 7-30 jours)
- [ ] Versions et tiers
  - [ ] Free trial (14-30 jours, full featured)
  - [ ] Version Lite (limitations features)
  - [ ] Version Pro (full)
  - [ ] Upgrades (Lite → Pro)
- [ ] Anti-piratage (réaliste)
  - [ ] Obfuscation du code de validation
  - [ ] Code signing obligatoire
  - [ ] Détection de debuggers (optionnel)
  - [ ] Ne PAS bloquer trop fort (UX > DRM)
- [ ] Tests et edge cases
  - [ ] Changement de hardware
  - [ ] Réinstallation OS
  - [ ] Transfert de licence
  - [ ] Remboursements (invalidation licence)

### Interface utilisateur moderne

- [ ] Design system implémentation
  - [ ] Palette de couleurs (d'après Phase 2.5)
  - [ ] Composants UI (boutons, sliders, knobs)
  - [ ] Typographie
- [ ] Écrans principaux
  - [ ] Vue synthétiseur
  - [ ] Piano Roll
  - [ ] Mixer
  - [ ] Browser de plugins
- [ ] Composants interactifs
  - [ ] Knobs SVG rotatifs (drag vertical)
  - [ ] Sliders avec valeur affichée
  - [ ] Waveform display (Canvas2D ou WebGL)
  - [ ] VU meters animés
- [ ] Thèmes
  - [ ] Thème sombre (par défaut)
  - [ ] Thème clair
  - [ ] Persistance préférence utilisateur

### Optimisation performances UI

- [ ] Canvas/WebGL pour visualisations temps-réel
  - [ ] Oscilloscope (WebGL)
  - [ ] Spectrum analyzer (WebGL)
  - [ ] Piano roll rendering
- [ ] Throttling des updates UI
  - [ ] 60 FPS max pour métriques
  - [ ] Debounce pour sliders
- [ ] Web Workers pour calculs lourds côté frontend (optionnel)

### Distribution et monétisation

- [ ] Code signing (OBLIGATOIRE)
  - [ ] Windows (certificat Authenticode ~200€/an)
  - [ ] macOS (Developer ID Apple 99$/an)
  - [ ] Impact sur licensing : empêche modifications binaire
- [ ] Packaging
  - [ ] Linux (AppImage, deb, rpm)
  - [ ] Windows (MSI, NSIS installer)
  - [ ] macOS (DMG, app bundle notarized)
- [ ] Auto-update system (Tauri updater)
  - [ ] Vérification de la licence avant update
  - [ ] Update différentiel (économiser bande passante)
- [ ] Release pipeline CI/CD
  - [ ] GitHub Actions pour build multiplatform
  - [ ] Artifacts storage (S3/DigitalOcean Spaces)
  - [ ] Changelog automatique
- [ ] Infrastructure monétisation
  - [ ] Site web de vente (Gumroad/Paddle/custom)
  - [ ] Checkout sécurisé (Stripe/PayPal)
  - [ ] Génération automatique de licence après achat (webhook)
  - [ ] Email confirmation avec clé
  - [ ] Système de support client (Zendesk/Intercom/custom)

---

## Backlog / Idées futures

### Features techniques

- [ ] Mode spectral/granular synthesis
- [ ] Wavetable synthesis
- [ ] Arrangement view
- [ ] Automation curves avancées
- [ ] Time stretching
- [ ] Pitch shifting
- [ ] Support multi-sortie audio
- [ ] Support JACK (Linux)
- [ ] Scripting (Lua/Python)
- [ ] Support LV2 plugins (Linux)

### Features monétisation avancées

- [ ] Système d'abonnement (subscription vs perpetual license)
- [ ] In-app purchases (packs de presets, expansion sounds)
- [ ] Cloud storage pour projets (sync multi-machines)
- [ ] Collaboration en temps réel (multi-utilisateurs)
- [ ] Mobile remote control (iOS/Android) avec IAP
- [ ] Marketplace de plugins/presets communautaires (commission)
- [ ] Programme d'affiliation (referral program)
- [ ] Educational licenses (étudiants/écoles)
- [ ] NFT integration (ownership de presets/samples) - si pertinent

---

## Roadmap résumée

| Phase | Objectif | Durée | Release | Cumul |
|-------|----------|-------|---------|-------|
| **Phase 1** ✅ | MVP - Synth polyphonique | TERMINÉ | v0.1.0 | - |
| **Phase 1.5** ✅ | Robustesse + Tests | TERMINÉ | v0.2.0 | ~3 sem |
| **Phase 2** ✅ | ADSR, LFO, Modulation | TERMINÉ | v0.3.0 | ~7 sem |
| **Phase 2.5** | UX Design | 1-2 sem | - | ~9 sem |
| **Phase 3a** ✅ | Filtres + 2 Effets | TERMINÉ | v0.4.0 | ~13 sem |
| **Phase 3b** 🐕 | Dogfooding (performance live) | 1 sem | - | ~14 sem |
| **Phase 3.5** 🎵 | Sampling | 2-3 sem | v0.5.0 | ~17 sem |
| **Phase 4** | Séquenceur + Dogfooding réel | 6-8 sem | **v1.0.0** 🎉 | ~25 sem |
| **Phase 5** | CLAP plugins + Routing | 4-6 sem | v1.1.0 | ~31 sem |
| **Phase 6a** | Performance + Stabilité | 3-4 sem | v1.2.0 | ~35 sem |
| **Phase 6b** ⚠️ | VST3 (OPTIONNEL) | 12-16 sem | v1.5.0 | ~51 sem |
| **Phase 7** | Tauri + Licensing | 6-8 sem | v2.0.0 | ~43 sem* |

\* Sans Phase 6b (VST3)

### Durées estimées totales

- **Sans VST3** : ~43 semaines (11 mois) → DAW complet avec CLAP + licensing
- **Avec VST3** : ~59 semaines (15 mois) → DAW + écosystème VST3 + licensing

### Milestones clés

- **v0.2.0** ✅ (Phase 1.5) : DAW partageable avec d'autres devs
- **v0.3.0** ✅ (Phase 2) : Synth expressif avec ADSR, LFO, Modulation
- **v0.4.0** ✅ (Phase 3a) : Filtres et effets essentiels
- **v0.5.0** 🎵 (Phase 3.5) : Support sampling (À VENIR)
- **v1.0.0** 🎉 (Phase 4) : DAW fonctionnel avec séquenceur + morceau complet (MILESTONE MAJEUR)
- **v1.1.0** (Phase 5) : Support plugins CLAP (ouverture écosystème)
- **v1.5.0** (Phase 6b) : Support VST3 (optionnel, complexe)
- **v2.0.0** (Phase 7) : UI moderne + Distribution publique

---

**Priorité actuelle** : Phase 3.5 - Sampling 🎵

**Phase 1.5** ✅ : Robustesse et tests - **TERMINÉE** (v0.2.0)
**Phase 2** ✅ : ADSR, LFO, Modulation - **TERMINÉE** (v0.3.0)
**Phase 3a** ✅ : Filtres et effets essentiels - **TERMINÉE** (v0.4.0)
**Phase 3b** ✅ : Performance live - **TERMINÉE**

**Next milestone** : Phase 3.5 (Sampling) → v0.5.0, puis Phase 4 (Séquenceur) → v1.0.0 🎉

---

## Décisions Architecturales Critiques 🏗️

Ces décisions doivent être prises **tôt** car elles impactent toute l'architecture du DAW.

### 1. Gestion de l'état global (critique pour Phase 7 Tauri)

**Problème** : Avec Tauri, synchronisation de l'état entre UI (JS/TS) et moteur audio (Rust) devient complexe.

**Décision** :
- **Source de vérité unique** : Le moteur audio (backend Rust)
- **UI = Vue** de cet état (read-only + envoi de commandes)
- **Pattern Commands & Events** :
  - `Commands` : UI → Audio (actions, via ringbuffer)
  - `StateEvents` : Audio → UI (notifications, via ringbuffer)
  - Validation dans le backend avant application
- **Redux côté frontend** (optionnel) : Pour gérer l'état UI uniquement (pas l'état audio)

**À implémenter** : Phase 2-3 (avant que ça devienne ingérable)

### 2. Architecture Undo/Redo (URGENT - Phase 2) ⚠️

**Problème** : Ajouter l'undo/redo après coup sur toutes les actions est **extrêmement complexe**.

**Décision** :
- **Command Pattern générique** dès Phase 2
- Trait `UndoableCommand { execute(), undo(), redo() }`
- Toutes les actions passent par un `CommandManager`
- Stack d'undo avec limite mémoire (ex: 100 actions)
- S'applique à **tout** : params, notes, routing, plugins, etc.

**Exemple** :
```rust
trait UndoableCommand: Send {
    fn execute(&mut self, state: &mut DawState) -> Result<()>;
    fn undo(&mut self, state: &mut DawState) -> Result<()>;
    fn description(&self) -> String;
}
```

**À implémenter** : Phase 2 (ADSR/LFO) - en même temps que les premiers params complexes

### 3. Format de Projet (Phase 4)

**Problème** : JSON seul = lent pour gros projets, binaire seul = pas debuggable.

**Décision** : **ZIP container hybride** (standard industrie)
- Structure :
  ```
  project.mymusic (ZIP)
  ├── manifest.json      # Metadata
  ├── project.ron        # État DAW (JSON ou RON)
  ├── tracks/*.json      # Notes, automation
  ├── audio/*.wav        # Samples (binaire)
  └── plugins/*.bin      # États plugins
  ```
- **Avantages** :
  - JSON/RON : Git-friendly, debuggable
  - Binaire : Performance pour audio
  - ZIP : Compression automatique
  - Extensible : Ajout de fichiers sans breaking changes
  - Versionning : Migration de format possible

**À implémenter** : Phase 4 (Séquenceur)

---

## Notes importantes

### Phase 6b (VST3) - Décision stratégique

**Option A** : Faire VST3 après Phase 6a

- ✅ Compatibilité totale avec écosystème existant
- ❌ +3-4 mois de dev complexe
- ❌ FFI Rust/C++ délicat

**Option B** : Skip VST3, focus CLAP

- ✅ Gain de 3-4 mois
- ✅ CLAP = futur, plus simple
- ✅ Communauté CLAP en croissance (Bitwig, Reaper, etc.)
- ❌ Moins de plugins disponibles initialement

**Recommandation** : Commencer sans VST3, évaluer après v1.2.0 selon feedback utilisateurs.

### Stratégie de release

- **v0.x** : Releases fréquentes (toutes les 3-4 semaines)
- **v1.0** : Milestone majeur (DAW complet)
- **v1.x** : Features additionnelles (plugins, perf)
- **v2.0** : Refonte UI (Tauri)

Chaque release doit être **utilisable** et **stable**, pas juste des features.
