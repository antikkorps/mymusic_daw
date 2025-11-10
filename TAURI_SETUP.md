# MyMusic DAW - Tauri Setup Guide

Guide complet pour lancer MyMusic DAW avec le frontend React et Tauri.

## 📋 Prérequis

### Système

- **Rust** 1.70+ (edition 2024)
- **Node.js** 18+ et npm
- **Cargo** (installé avec Rust)

### Dépendances système (Linux/Ubuntu)

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.0-dev \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libasound2-dev \
    pkg-config
```

### Dépendances système (macOS)

```bash
xcode-select --install
```

### Dépendances système (Windows)

- Installez [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- Installez [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

## 🚀 Installation

### 1. Cloner et préparer le projet

```bash
cd mymusic_daw/
```

### 2. Installer Tauri CLI

```bash
cargo install tauri-cli --version "^2.0.0"
```

### 3. Installer les dépendances du frontend

```bash
cd ui/
npm install
```

### 4. Configuration du frontend (première fois)

Si vous n'avez pas encore de projet Vite, initialisez-le :

```bash
cd ui/
npm create vite@latest . -- --template react-ts
```

Copiez les fichiers d'exemple :

```bash
# Copier le composant de test
cp app/App.example.tsx ../src/App.tsx

# Copier le hook (si nécessaire)
mkdir -p ../src/hooks
cp app/hooks/useDawEngine.ts ../src/hooks/

# Copier le composant
mkdir -p ../src/components
cp app/components/DawEngineTest.tsx ../src/components/
```

## 🎵 Lancer l'application

### Mode développement (recommandé)

**Option 1: Commande unifiée Tauri**

```bash
cd src-tauri/
cargo tauri dev
```

Cela va :
1. Compiler le backend Rust
2. Lancer le serveur de dev Vite
3. Ouvrir la fenêtre Tauri

**Option 2: Terminaux séparés (pour debug)**

Terminal 1 - Frontend React :
```bash
cd ui/
npm run dev
```

Terminal 2 - Tauri :
```bash
cd src-tauri/
cargo run
```

### Mode production

```bash
cd src-tauri/
cargo tauri build
```

Le binaire sera dans `src-tauri/target/release/`

## 🧪 Tester les fonctionnalités

Une fois l'application lancée :

1. **Status du moteur** : Devrait afficher 🟢 Engine Ready
2. **Volume** : Déplacer le slider pour ajuster (0-100%)
3. **Quick Triggers** : Cliquer sur les boutons de notes (C4-B4)
4. **Sustained Notes** : Maintenir les boutons enfoncés

## 🔧 Structure du projet

```
mymusic_daw/
├── src/                    # Code Rust du moteur audio (original)
├── src-tauri/              # Backend Tauri
│   ├── src/
│   │   ├── lib.rs         # Commandes Tauri exposées
│   │   └── main.rs        # Entry point Tauri
│   ├── Cargo.toml         # Dépendances Tauri
│   ├── tauri.conf.json    # Configuration Tauri
│   └── build.rs           # Build script
└── ui/                     # Frontend React
    ├── app/
    │   ├── hooks/
    │   │   └── useDawEngine.ts
    │   └── components/
    │       └── DawEngineTest.tsx
    ├── package.json
    ├── vite.config.ts
    └── tsconfig.json
```

## 🎹 API Tauri disponible

### Commandes

```typescript
// Volume
await invoke('set_volume', { volume: 0.5 });
const vol = await invoke<number>('get_volume');

// Notes MIDI
await invoke('play_note', { note: 60, velocity: 100 });
await invoke('stop_note', { note: 60 });

// Status
const status = await invoke('get_engine_status');
```

### Hook React

```typescript
import { useDawEngine } from './hooks/useDawEngine';

function MyComponent() {
  const {
    volume,
    setVolume,
    playNote,
    stopNote,
    isEngineReady
  } = useDawEngine();

  // ...
}
```

## 🐛 Dépannage

### Erreur: "tauri not found"

```bash
cargo install tauri-cli --version "^2.0.0"
```

### Erreur: "failed to load config"

Vérifiez que `src-tauri/tauri.conf.json` existe et est valide.

### Erreur: Port 5173 déjà utilisé

Changez le port dans `ui/vite.config.ts` et `src-tauri/tauri.conf.json`.

### Audio ne fonctionne pas

1. Vérifiez les permissions audio de votre système
2. Regardez les logs Rust : `RUST_LOG=debug cargo tauri dev`
3. Vérifiez que votre carte son est détectée

### Build Rust échoue

```bash
# Nettoyer et rebuild
cd src-tauri/
cargo clean
cargo build
```

## 📊 Logs et Debug

### Activer les logs détaillés

```bash
RUST_LOG=debug cargo tauri dev
```

### Console DevTools

En mode développement, appuyez sur `F12` pour ouvrir les DevTools Chrome.

### Logs du moteur audio

Les logs apparaissent dans le terminal où vous avez lancé `cargo tauri dev` :

```
🎵 Initializing MyMusic DAW...
📢 Available audio devices:
  ✓ Default Audio Device
✅ Audio engine started successfully
🚀 Tauri app initialized
🎹 DAW is ready!
```

## 🎯 Prochaines étapes

Une fois le setup fonctionnel :

1. **Personnaliser l'UI** : Modifier `DawEngineTest.tsx`
2. **Ajouter des contrôles** : ADSR, LFO, filtres
3. **Intégrer le piano roll** : Portage de l'egui vers React
4. **Ajouter le séquenceur** : Timeline, transport controls
5. **Plugins CLAP** : UI pour charger et contrôler les plugins

## 📚 Ressources

- [Tauri Documentation](https://tauri.app/)
- [Vite Documentation](https://vitejs.dev/)
- [React Documentation](https://react.dev/)
- [MyMusic DAW - Original README](./README.md)

## 💡 Tips

### Hot Reload

En mode dev, les modifications React sont rechargées automatiquement. Pour recharger le Rust :

```bash
# Tauri recompile automatiquement si vous relancez
cargo tauri dev
```

### Performance

Pour de meilleures performances audio, compilez en mode release :

```bash
cargo tauri build --release
```

### Multi-plateforme

Tauri compile nativement pour chaque plateforme. Le même code fonctionne sur :
- 🐧 Linux
- 🍎 macOS
- 🪟 Windows

## ⚙️ Configuration avancée

### Changer la taille de fenêtre

Modifier `src-tauri/tauri.conf.json` :

```json
{
  "app": {
    "windows": [{
      "width": 1600,
      "height": 1000
    }]
  }
}
```

### Désactiver DevTools en production

Dans `src-tauri/Cargo.toml`, retirer la feature `devtools` :

```toml
[dependencies]
tauri = { version = "2", features = [] }  # sans "devtools"
```

### Icône personnalisée

Placez vos icônes dans `src-tauri/icons/` et mettez à jour `tauri.conf.json`.
