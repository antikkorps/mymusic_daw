# DAW Frontend POC - AGENTS

## Project Overview

This is a **Proof of Concept (POC)** frontend for a Digital Audio Workstation (DAW) built with React, designed to validate the UI/UX before integration with a Rust backend via Tauri.

### Tech Stack

- **React 19** with TypeScript
- **React Router 7** (SPA mode)
- **Vite** (bundler)
- **Tailwind CSS 4** (styling)
- **Shadcn/ui** inspired components
- **Lucide React** (icons)
- **React Flow** (audio routing visualization)
- **Framer Motion** (animations)
- **Recharts** (visualizations)

## Architecture

### State Management
- Currently using local React state (`useState`)
- For production, consider:
  - **Zustand** for global audio engine state
  - **Jotai** for atomic state management
  - **TanStack Query** for backend data synchronization

### Backend Integration (Future)
The Rust backend features:
- CPAL audio engine
- 16-voice polyphony
- MIDI support
- Effects: delay, reverb, SVF filters
- CLAP plugin support
- Lock-free real-time architecture

Integration approach with Tauri:
1. Replace mock data with Tauri IPC calls
2. Implement audio engine state synchronization
3. Add WebSocket for real-time meter updates
4. Use SharedArrayBuffer for low-latency audio visualization

## Component Architecture

### Reusable Audio Components
- **Fader** (`app/components/audio/Fader.tsx`): Vertical fader with dB scale
- **Knob** (`app/components/audio/Knob.tsx`): Rotary knob with drag control
- **VUMeter** (`app/components/audio/VUMeter.tsx`): Real-time level meter with peak hold
- **Waveform** (`app/components/audio/Waveform.tsx`): Canvas-based waveform visualization

### Layout Components
- **Sidebar** (`app/components/layout/Sidebar.tsx`): Navigation sidebar
- **Toolbar** (`app/components/layout/Toolbar.tsx`): Transport controls and tempo
- **StatusBar** (`app/components/layout/StatusBar.tsx`): System info (CPU, RAM, latency)

### Page Components
- **Dashboard** (`app/routes/_index.tsx`): Overview, stats, quick actions
- **Mixer** (`app/routes/mixer.tsx`): 8-track mixer with faders, pan, effects
- **Piano Roll** (`app/routes/piano-roll.tsx`): MIDI note editor
- **Timeline** (`app/routes/timeline.tsx`): Audio/MIDI clip arranger
- **Plugins** (`app/routes/plugins.tsx`): Plugin browser and manager
- **Effects** (`app/routes/effects.tsx`): Visual effect routing with React Flow

## Data Flow

### Current (Mock Mode)
```
Mock Data → React State → Components → UI
```

### Future (Tauri Integration)
```
Rust Backend (CPAL) → Tauri IPC → React State → Components → UI
                    ↓
                WebSocket → Real-time meters/waveforms
```

## Development Principles

### DRY (Don't Repeat Yourself)
- Audio controls are abstracted into reusable components
- Shared utilities in `app/lib/utils.ts`
- Type definitions centralized in `app/types/`

### Performance
- Canvas-based rendering for waveforms (60fps target)
- React.memo for expensive components (consider for production)
- VU meters update at 20fps to balance smoothness and CPU
- Virtual scrolling for large lists (consider for plugin browser)

### Accessibility
- Keyboard navigation for transport controls
- ARIA labels on interactive elements
- High contrast colors for readability
- Focus indicators on all interactive elements

## Styling Conventions

### Color Palette
- Background: `zinc-950` (#09090b)
- Surface: `zinc-900` (#18181b)
- Border: `zinc-800` (#27272a)
- Text: `zinc-50` (white), `zinc-400` (secondary)
- Accent: `cyan-500` (#06b6d4)
- Warning: `yellow-500` (#eab308)
- Error: `red-500` (#ef4444)

### Component Patterns
- Cards for content grouping
- Consistent spacing (4px grid)
- Rounded corners (lg: 8px, md: 6px, sm: 4px)
- Hover states for all interactive elements
- Transition effects for state changes

## Future Enhancements

### Priority 1 (Core Features)
- [ ] Undo/Redo system
- [ ] Keyboard shortcuts
- [ ] Track automation lanes
- [ ] Plugin window management
- [ ] Project save/load

### Priority 2 (Enhanced UX)
- [ ] Drag & drop for clips
- [ ] MIDI input monitoring
- [ ] Audio recording
- [ ] Real waveform rendering from audio buffers
- [ ] Plugin parameter automation

### Priority 3 (Advanced)
- [ ] Multiple undo history branches
- [ ] Collaborative editing (CRDT)
- [ ] Cloud project storage
- [ ] VST3/AU plugin support
- [ ] MPE (MIDI Polyphonic Expression)

## Testing Strategy

### Unit Tests (To Implement)
- Audio utility functions (dB conversion, pan, etc.)
- State management logic
- Component render tests

### Integration Tests (To Implement)
- Full page rendering
- User interactions (clicks, drags)
- Route navigation

### E2E Tests (To Implement)
- Full workflow: create track → add clip → add effect → play
- Plugin loading and management
- MIDI note editing

## Performance Targets

- **Initial Load**: < 1s
- **Route Transition**: < 100ms
- **VU Meter Update**: 20fps (50ms interval)
- **Waveform Animation**: 60fps
- **UI Interaction Response**: < 16ms (60fps)

## Browser Support

- Chrome/Edge 120+
- Firefox 120+
- Safari 17+ (limited - WebAudio API differences)

## Development Workflow

### Local Development
```bash
npm run dev          # Start dev server (port 5173)
npm run build        # Build for production
npm run typecheck    # Check TypeScript types
```

### Code Style
- Use TypeScript strict mode
- Prefer functional components with hooks
- Use destructuring for props
- Keep components under 300 lines
- Extract complex logic into custom hooks

### Git Workflow
- Feature branches: `feature/component-name`
- Commit messages: Conventional Commits format
- PR reviews required before merge

## Contact & Resources

- **Backend Repo**: [Link to Rust DAW repo]
- **Design System**: Shadcn/ui + custom DAW components
- **Documentation**: This file + inline code comments

---

**Last Updated**: 2025-11-08
**Version**: 1.0.0-poc
