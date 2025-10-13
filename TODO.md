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

**Total tests : 66 tests passent** 🎉

### Documentation et communauté - **REPORTÉ POST-v1.0** ⏭️

**Décision** : Trop tôt pour ouvrir aux contributeurs externes. L'API et l'architecture vont encore beaucoup évoluer jusqu'à v1.0 (Phase 4). Cette section sera réactivée après avoir atteint le milestone v1.0.0 avec un DAW fonctionnel et stable.

**Reporté à** : Phase 6a (Performance et stabilité) - Quand le projet sera "production-ready"

- [ ] Documentation cargo doc des modules principaux
- [ ] README.md avec screenshots et getting started
- [ ] CONTRIBUTING.md (guidelines pour contributeurs)
- [ ] GitHub repo public avec issues templates
- [ ] Discord/Forum setup (optionnel, si communauté intéressée)
- [ ] Documentation utilisateur (manuel, FAQ)

---

## Phase 2 : Enrichissement du son 🎛️

**Objectif** : Synth expressif avec modulation
**Release** : v0.3.0

**⚠️ ARCHITECTURE CRITIQUE** : Implémenter le **Command Pattern** dès cette phase pour l'Undo/Redo (voir "Décisions Architecturales"). Toutes les modifications de paramètres (ADSR, LFO, etc.) doivent passer par des `UndoableCommand`.

### Command Pattern & Undo/Redo (PRIORITAIRE)

- [ ] Implémenter le trait `UndoableCommand`
- [ ] Créer le `CommandManager` avec undo/redo stacks
- [ ] Implémenter `SetParameterCommand` pour les params audio
- [ ] Intégrer Ctrl+Z / Ctrl+Y dans l'UI
- [ ] Tester avec les paramètres ADSR et LFO
- [ ] Documentation du pattern pour futures features

### Enveloppes

- [ ] Implémenter enveloppe ADSR
  - [ ] Attack
  - [ ] Decay
  - [ ] Sustain
  - [ ] Release
- [ ] Intégrer ADSR dans Voice
- [ ] UI pour contrôles ADSR
- [ ] Tests unitaires ADSR (timing, courbes)

### Polyphonie avancée

- [ ] Améliorer le voice stealing (priorité par vélocité/âge)
- [ ] Modes de polyphonie (mono, legato, poly)
- [ ] Portamento/glide

### Modulation

- [ ] LFO (Low Frequency Oscillator)
  - [ ] Formes d'onde LFO (sine, square, saw, triangle)
  - [ ] Routing LFO → paramètres (pitch, cutoff)
  - [ ] Sync LFO au tempo (optionnel)
- [ ] Vélocité → intensité
- [ ] Aftertouch support

### Architecture de modulation avancée

- [ ] Matrice de modulation générique
  - [ ] Sources de modulation (LFO, Enveloppes, Vélocité, Aftertouch, etc.)
  - [ ] Destinations de modulation (Pitch, Cutoff, Amplitude, Pan, etc.)
  - [ ] Système d'assignation flexible source → destination
  - [ ] Quantité de modulation réglable par routing
  - [ ] UI pour visualiser et éditer la matrice

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

## Phase 3a : Filtres et effets essentiels 🔊

**Objectif** : 1 filtre + 2 effets de qualité
**Release** : v0.4.0
**Durée** : 3-4 semaines

### Filtres

- [ ] Low-pass filter (Moog-style)
  - [ ] Implémentation algorithme (State Variable Filter ou Moog Ladder)
  - [ ] Cutoff control
  - [ ] Résonance control
  - [ ] Cutoff modulation (envelope, LFO)
  - [ ] Tests audio (pas d'artefacts, stabilité)

### Effets prioritaires

- [ ] Delay
  - [ ] Delay line (buffer circulaire pré-alloué)
  - [ ] Time control (ms ou sync tempo)
  - [ ] Feedback control
  - [ ] Mix (dry/wet)
  - [ ] Tests (pas de clics, feedback stable)
- [ ] Réverbération (algorithme simple)
  - [ ] Freeverb ou Schroeder reverb
  - [ ] Room size
  - [ ] Damping
  - [ ] Mix
  - [ ] Tests (pas de distorsion)

### Architecture effets

- [ ] Trait Effect générique
- [ ] Chain d'effets (Vec pré-allouée)
- [ ] Bypass individuel
- [ ] Latency reporting (futur)

---

## Phase 3b : Dogfooding et amélioration qualité 🐕

**Objectif** : Utiliser le DAW pour créer un morceau complet
**Durée** : 1 semaine

### Création d'un morceau

- [ ] Créer un morceau complet (2-3 min) avec le DAW
- [ ] Identifier tous les bugs UX
- [ ] Lister features manquantes critiques
- [ ] Améliorer workflow d'après l'expérience

### Polissage

- [ ] Fixer bugs critiques découverts
- [ ] Améliorer qualité audio des filtres/effets
- [ ] Optimiser performance si nécessaire
- [ ] Documenter limitations connues

---

## Phase 4 : Séquenceur 🎹

**Objectif** : DAW complet avec séquenceur fonctionnel
**Release** : v1.0.0 🎉 (MILESTONE MAJEUR)
**Durée** : 6-8 semaines

**⚠️ ARCHITECTURE CRITIQUE** : Format de projet en **ZIP container hybride** (voir "Décisions Architecturales"). JSON/RON pour l'état, binaire pour les samples, extensible et versionné.

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
- [ ] Sampling
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
| **Phase 2** | ADSR, LFO, Modulation | 3-4 sem | v0.3.0 | ~7 sem |
| **Phase 2.5** | UX Design | 1-2 sem | - | ~9 sem |
| **Phase 3a** | Filtres + 2 Effets | 3-4 sem | v0.4.0 | ~13 sem |
| **Phase 3b** 🐕 | Dogfooding (créer morceau) | 1 sem | - | ~14 sem |
| **Phase 4** | Séquenceur + MIDI Clock | 6-8 sem | **v1.0.0** 🎉 | ~22 sem |
| **Phase 5** | CLAP plugins + Routing | 4-6 sem | v1.1.0 | ~28 sem |
| **Phase 6a** | Performance + Stabilité | 3-4 sem | v1.2.0 | ~32 sem |
| **Phase 6b** ⚠️ | VST3 (OPTIONNEL) | 12-16 sem | v1.5.0 | ~48 sem |
| **Phase 7** | Tauri + Licensing | 6-8 sem | v2.0.0 | ~40 sem* |

\* Sans Phase 6b (VST3)

### Durées estimées totales

- **Sans VST3** : ~40 semaines (10 mois) → DAW complet avec CLAP + licensing
- **Avec VST3** : ~52 semaines (13 mois) → DAW + écosystème VST3 + licensing

### Milestones clés

- **v0.2.0** (Phase 1.5) : DAW partageable avec d'autres devs
- **v1.0.0** (Phase 4) : 🎉 DAW fonctionnel avec séquenceur (MILESTONE MAJEUR)
- **v1.1.0** (Phase 5) : Support plugins CLAP (ouverture écosystème)
- **v1.5.0** (Phase 6b) : Support VST3 (optionnel, complexe)
- **v2.0.0** (Phase 7) : UI moderne + Distribution publique

---

**Priorité actuelle** : Phase 1.5 - Robustesse et UX de base ✅ **TERMINÉE**
**Objectif** : Rendre le DAW utilisable par d'autres personnes
**Progrès Phase 1.5** :
  - ✅ Gestion des périphériques audio/MIDI
  - ✅ Reconnexion automatique MIDI
  - ✅ Gestion des erreurs Audio (CPAL)
  - ✅ Timing et précision audio/MIDI
  - ✅ Monitoring CPU
  - ✅ Compatibilité formats CPAL (F32/I16/U16)
  - ✅ Tests d'intégration (66 tests passent)
  - ✅ Benchmarks Criterion (latence < 10ms atteinte)
  - ⏭️ Documentation (reportée post-v1.0)

**Release v0.2.0 prête** 🎉

**Next milestone** : Phase 2 - Enrichissement du son (ADSR, LFO, Command Pattern)

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
