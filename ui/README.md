# MyMusic DAW - React Frontend

Interface utilisateur React pour MyMusic DAW, connectée au moteur audio Rust via Tauri.

## 🎯 Structure

```
ui/
├── app/
│   ├── hooks/
│   │   └── useDawEngine.ts      # Hook principal pour contrôler le moteur audio
│   └── components/
│       └── DawEngineTest.tsx    # Composant de test/démonstration
└── README.md
```

## 🔌 Integration Tauri

### Architecture

```
┌─────────────────┐          ┌──────────────────┐
│  React Frontend │ ◄──IPC──► │  Tauri Backend   │
│  (TypeScript)   │          │  (Rust)          │
└─────────────────┘          └──────────────────┘
                                      │
                                      ▼
                             ┌─────────────────┐
                             │  Audio Engine   │
                             │  (CPAL + Synth) │
                             └─────────────────┘
```

### Commandes Tauri Disponibles

#### 1. `set_volume(volume: number)`
Définit le volume maître du DAW.

```typescript
import { invoke } from '@tauri-apps/api/core';

await invoke('set_volume', { volume: 0.5 }); // 50%
```

#### 2. `get_volume()`
Récupère le volume actuel.

```typescript
const volume = await invoke<number>('get_volume');
console.log('Current volume:', volume);
```

#### 3. `play_note(note: number, velocity: number)`
Joue une note MIDI.

```typescript
// Jouer middle C (60) avec vélocité 100
await invoke('play_note', { note: 60, velocity: 100 });
```

#### 4. `stop_note(note: number)`
Arrête une note MIDI.

```typescript
await invoke('stop_note', { note: 60 });
```

#### 5. `get_engine_status()`
Récupère le statut du moteur audio.

```typescript
const status = await invoke('get_engine_status');
// { name: "MyMusic DAW", version: "0.1.0", status: "running" }
```

## 🪝 Utilisation du Hook `useDawEngine`

### Exemple basique

```typescript
import { useDawEngine } from './hooks/useDawEngine';

function VolumeControl() {
  const { volume, setVolume, isEngineReady } = useDawEngine();

  return (
    <div>
      <h3>Volume: {Math.round(volume * 100)}%</h3>
      <input
        type="range"
        min="0"
        max="1"
        step="0.01"
        value={volume}
        onChange={(e) => setVolume(parseFloat(e.target.value))}
        disabled={!isEngineReady}
      />
    </div>
  );
}
```

### Exemple avec notes

```typescript
import { useDawEngine } from './hooks/useDawEngine';

function PianoKey({ note }: { note: number }) {
  const { playNote, stopNote, isEngineReady } = useDawEngine();

  return (
    <button
      onMouseDown={() => playNote(note, 100)}
      onMouseUp={() => stopNote(note)}
      disabled={!isEngineReady}
    >
      Play Note {note}
    </button>
  );
}
```

### Hook `useNotePlayer` (helper)

Pour jouer des notes avec note-off automatique :

```typescript
import { useNotePlayer } from './hooks/useDawEngine';

function QuickNoteButton({ note }: { note: number }) {
  const { triggerNote } = useNotePlayer();

  return (
    <button onClick={() => triggerNote(note, 100, 500)}>
      Play {note} (500ms)
    </button>
  );
}
```

## 🧪 Composant de Test

Le composant `DawEngineTest` démontre toutes les fonctionnalités :

```typescript
import { DawEngineTest } from './components/DawEngineTest';

function App() {
  return <DawEngineTest />;
}
```

Fonctionnalités démontrées :
- ✅ Contrôle de volume avec slider
- ✅ Boutons presets de volume (0%, 25%, 50%, 75%, 100%)
- ✅ Triggers rapides de notes (300ms)
- ✅ Notes maintenues (press & hold)
- ✅ Statut du moteur en temps réel
- ✅ Gestion d'erreurs

## 📦 Installation (si vous n'avez pas encore configuré React)

### 1. Initialiser le projet React

```bash
cd ui/
npm create vite@latest . -- --template react-ts
npm install
```

### 2. Installer Tauri API

```bash
npm install @tauri-apps/api
```

### 3. Ajouter le composant au App.tsx

```typescript
// ui/src/App.tsx
import { DawEngineTest } from './app/components/DawEngineTest';

function App() {
  return (
    <div className="App">
      <DawEngineTest />
    </div>
  );
}

export default App;
```

## 🚀 Lancer l'application

### Mode développement

```bash
# Terminal 1: Lancer le frontend React
cd ui/
npm run dev

# Terminal 2: Lancer Tauri
cd src-tauri/
cargo tauri dev
```

### Mode production

```bash
cd src-tauri/
cargo tauri build
```

## 🎹 Notes MIDI de référence

| Note | Nom   | Description      |
|------|-------|------------------|
| 60   | C4    | Middle C         |
| 61   | C#4   |                  |
| 62   | D4    |                  |
| 63   | D#4   |                  |
| 64   | E4    |                  |
| 65   | F4    |                  |
| 66   | F#4   |                  |
| 67   | G4    |                  |
| 68   | G#4   |                  |
| 69   | A4    | Concert pitch    |
| 70   | A#4   |                  |
| 71   | B4    |                  |

## 🔧 Dépannage

### Erreur: "Failed to get volume"
- Vérifiez que le moteur audio est bien démarré
- Regardez les logs dans la console Rust

### Notes ne sonnent pas
- Vérifiez le volume (peut-être à 0%)
- Vérifiez que votre carte son est bien détectée
- Regardez les logs du moteur audio

### Build Tauri échoue
```bash
# Installer les dépendances système (Ubuntu/Debian)
sudo apt update
sudo apt install libwebkit2gtk-4.0-dev \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev

# macOS
xcode-select --install
```

## 📚 Ressources

- [Tauri Documentation](https://tauri.app/)
- [Tauri API Docs](https://tauri.app/v1/api/js/)
- [React Documentation](https://react.dev/)
- [MIDI Specification](https://www.midi.org/specifications)

## 🎵 Prochaines étapes

- [ ] Ajouter des contrôles ADSR
- [ ] Interface pour LFO et modulation
- [ ] Piano roll interactif
- [ ] Gestion des plugins CLAP
- [ ] Mixeur multi-pistes
- [ ] Séquenceur avec timeline
