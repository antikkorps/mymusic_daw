# MyMusic DAW

Un DAW (Digital Audio Workstation) minimaliste écrit en Rust.

## Architecture

- **Audio Backend** : CPAL (callback temps-réel, lock-free)
- **MIDI Input** : midir
- **Interface** : egui/eframe (à venir)
- **Communication** : Ringbuffer lock-free entre threads

Voir [AGENTS.md](AGENTS.md) pour l'architecture complète.

## État actuel (MVP - Phase 1) ✅ TERMINÉ

✅ **Fonctionnalités implémentées** :
- Moteur audio CPAL avec callback temps-réel
- Système de communication lock-free (2 ringbufs : MIDI + UI → Audio)
- Oscillateurs (Sine, Square, Saw, Triangle)
- Voice Manager avec polyphonie (16 voix)
- Input MIDI (détection automatique du premier port)
- Conversion MIDI note → fréquence
- **Interface utilisateur egui/eframe**
  - Clavier virtuel avec touches PC (A W S E D F T G Y H U J K)
  - Clavier visuel cliquable
  - Slider de volume (UI seulement, pas encore connecté à l'audio)
  - Affichage du nombre de notes actives

🎯 **Prochaines étapes (Phase 2)** :
- Connecter le slider de volume à l'audio
- Sélecteur de forme d'onde
- Enveloppe ADSR
- Modulation LFO

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
├── audio/          # Moteur CPAL et callback temps-réel
├── synth/          # Oscillateurs, voix, polyphonie
├── midi/           # Input MIDI et parsing
├── ui/             # Interface egui (à venir)
└── messaging/      # Communication lock-free
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

### Phase 1 (MVP - en cours)
- [x] Audio engine CPAL
- [x] MIDI input
- [x] Oscillateurs de base
- [x] Polyphonie
- [ ] UI basique
- [ ] Tests d'intégration

### Phase 2
- Enveloppes ADSR
- Modulation (LFO, vélocité)
- Polyphonie avancée

### Phase 3
- Filtres (LP, HP, BP)
- Effets (delay, reverb, distortion)

### Phase 4+
- Séquenceur / Piano roll
- Architecture de plugins
- Export audio

## Développement

### Build

```bash
cargo build
```

### Run

```bash
cargo run
```

### Check

```bash
cargo check
```

## License

MIT (à définir)

## Credits

- CPAL : Cross-platform audio I/O
- midir : Cross-platform MIDI I/O
- egui : Immediate mode GUI
- ringbuf : Lock-free ring buffer
