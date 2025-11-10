# DAW Studio - Frontend POC

A modern Digital Audio Workstation (DAW) frontend built with React, designed as a proof-of-concept to validate UI/UX before integration with a Rust audio engine backend via Tauri.

![Version](https://img.shields.io/badge/version-1.0.0--poc-blue)
![React](https://img.shields.io/badge/React-19-blue)
![TypeScript](https://img.shields.io/badge/TypeScript-5.9-blue)
![License](https://img.shields.io/badge/license-MIT-green)

## ✨ Features

### Phase 1 (Complete)

- **🎛️ Professional Mixer**: 8-track mixer with vertical faders, pan knobs, solo/mute, and VU meters
- **🎹 Piano Roll**: MIDI note editor with grid snapping and velocity visualization
- **⏱️ Timeline**: Audio/MIDI clip arranger with visual waveforms
- **🔌 Plugin Manager**: Browse, load, and manage CLAP/VST3/AU plugins
- **🌊 Effect Routing**: Visual effect chain with React Flow
- **📊 Dashboard**: Project overview with stats and quick actions
- **🎨 Dark Theme**: Professional DAW-style dark UI with custom components

## 🚀 Quick Start

### Prerequisites

- Node.js 20+ (recommended: use nvm)
- npm or pnpm

### Installation

```bash
# Clone the repository
git clone <repo-url>
cd poc_front_daw

# Install dependencies
npm install

# Start development server
npm run dev
```

The app will be available at `http://localhost:5173`

### Available Scripts

```bash
npm run dev        # Start development server
npm run build      # Build for production
npm run typecheck  # Run TypeScript type checking
npm run start      # Preview production build
```

## 📁 Project Structure

```
app/
├── routes/              # Page components
│   ├── _index.tsx      # Dashboard
│   ├── mixer.tsx       # Mixer page
│   ├── piano-roll.tsx  # Piano Roll editor
│   ├── timeline.tsx    # Timeline/Arranger
│   ├── plugins.tsx     # Plugin Manager
│   └── effects.tsx     # Effect Routing
├── components/
│   ├── audio/          # Audio-specific components
│   │   ├── Fader.tsx
│   │   ├── Knob.tsx
│   │   ├── VUMeter.tsx
│   │   └── Waveform.tsx
│   ├── layout/         # Layout components
│   │   ├── Sidebar.tsx
│   │   ├── Toolbar.tsx
│   │   └── StatusBar.tsx
│   ├── mixer/          # Mixer components
│   │   └── Track.tsx
│   └── ui/             # Base UI components
│       ├── button.tsx
│       ├── card.tsx
│       └── slider.tsx
├── lib/
│   ├── utils.ts        # Utility functions
│   └── mockData.ts     # Mock data for POC
├── types/
│   ├── audio.ts        # Audio type definitions
│   ├── midi.ts         # MIDI type definitions
│   └── plugin.ts       # Plugin type definitions
└── hooks/              # Custom React hooks (future)
```

## 🎨 Tech Stack

- **React 19** - UI framework
- **TypeScript 5.9** - Type safety
- **Vite** - Build tool
- **React Router 7** - Routing (SPA mode)
- **Tailwind CSS 4** - Styling
- **Lucide React** - Icons
- **React Flow** - Node-based UI for effect routing
- **Framer Motion** - Animations
- **Recharts** - Data visualization

## 🎛️ Features Overview

### Mixer
- 8 audio/MIDI tracks + master track
- Vertical faders (-60dB to +6dB)
- Pan control (-100% L to +100% R)
- Real-time VU meters with peak hold
- Solo/Mute/Arm buttons
- Effect insert slots (3 per track)

### Piano Roll
- MIDI note editor (C3-C6 range)
- Grid snapping (1/4, 1/8, 1/16, 1/32)
- Velocity visualization
- Piano keyboard sidebar
- Note selection and editing

### Timeline
- Horizontal track lanes
- Audio/MIDI clips with colors
- Zoom controls
- Clip selection
- Visual waveforms

### Plugin Manager
- Browse available plugins
- Filter by category (Instrument, Effect, etc.)
- Load/Unload plugins
- Plugin info (vendor, version, format)
- Search functionality

### Effects
- Visual effect routing with React Flow
- Effect nodes: Filter, Delay, Reverb
- Adjustable parameters per effect
- Animated signal flow

## 🔧 Configuration

### Tailwind CSS
The project uses Tailwind CSS 4 with custom theme configuration in `app.css`.

### TypeScript
Strict mode enabled. Type definitions are in `app/types/`.

## 🚧 Roadmap

See [TODO.md](./TODO.md) for detailed roadmap.

### Phase 2: UI Enhancements
- Drag-and-drop for clips
- Framer Motion animations
- Context menus
- Keyboard shortcuts

### Phase 3: Tauri Integration
- Connect to Rust audio engine
- Real-time audio processing
- MIDI input
- Plugin hosting
- Project save/load

## 📚 Documentation

- [AGENTS.md](./AGENTS.md) - Architecture and development guide
- [TODO.md](./TODO.md) - Detailed task list and roadmap

## 🤝 Contributing

This is a POC project. Contributions are welcome!

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 🐛 Known Issues

- VU meters use mock data (random values)
- Drag-and-drop not yet implemented for timeline clips
- No keyboard shortcuts yet
- Desktop-only (not responsive for mobile)

## 📝 License

MIT License - see [LICENSE](./LICENSE) file for details

## 🙏 Acknowledgments

- Inspired by professional DAWs (Ableton Live, Logic Pro, Reaper)
- UI components based on Shadcn/ui design system
- Audio concepts from CPAL and CLAP plugin standards

---

**Status**: Phase 1 Complete ✅
**Next**: Phase 2 - UI Enhancements

Built with ❤️ using React Router and TypeScript.
