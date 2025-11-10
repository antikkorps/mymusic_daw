/**
 * Test component for DAW Engine integration
 * Demonstrates volume control and note playback via Tauri
 */

import React from 'react';
import { useDawEngine, useNotePlayer } from '../hooks/useDawEngine';

// MIDI note names for reference
const NOTE_NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];

function getNoteName(note: number): string {
  const octave = Math.floor(note / 12) - 1;
  const noteName = NOTE_NAMES[note % 12];
  return `${noteName}${octave}`;
}

export function DawEngineTest() {
  const {
    volume,
    isEngineReady,
    engineStatus,
    error,
    setVolume,
    playNote,
    stopNote,
    refreshEngineStatus,
  } = useDawEngine();

  const { triggerNote } = useNotePlayer();

  // Middle C octave (C4-B4) - notes 60-71
  const middleCOctave = Array.from({ length: 12 }, (_, i) => i + 60);

  // Active notes state (for visual feedback)
  const [activeNotes, setActiveNotes] = React.useState<Set<number>>(new Set());

  const handleNotePress = async (note: number) => {
    setActiveNotes((prev) => new Set(prev).add(note));
    await playNote(note, 100);
  };

  const handleNoteRelease = async (note: number) => {
    setActiveNotes((prev) => {
      const newSet = new Set(prev);
      newSet.delete(note);
      return newSet;
    });
    await stopNote(note);
  };

  const handleQuickNote = (note: number) => {
    triggerNote(note, 100, 300); // Play for 300ms
  };

  // Keyboard event handlers for sustained notes
  const handleKeyDown = (e: React.KeyboardEvent, note: number) => {
    // Only handle Space and Enter keys
    if (e.key !== ' ' && e.key !== 'Enter') return;
    
    // Prevent default behavior (e.g., scrolling for Space)
    e.preventDefault();
    
    // Don't trigger if already active (prevents key repeat)
    if (activeNotes.has(note)) return;
    
    handleNotePress(note);
  };

  const handleKeyUp = (e: React.KeyboardEvent, note: number) => {
    // Only handle Space and Enter keys
    if (e.key !== ' ' && e.key !== 'Enter') return;
    
    e.preventDefault();
    handleNoteRelease(note);
  };

  return (
    <div style={styles.container}>
      <h2 style={styles.title}>🎵 MyMusic DAW - Engine Test</h2>

      {/* Engine Status */}
      <div style={styles.statusContainer}>
        <div style={styles.statusBadge(isEngineReady)}>
          {isEngineReady ? '🟢 Engine Ready' : '🔴 Engine Not Ready'}
        </div>
        {engineStatus && (
          <div style={styles.statusInfo}>
            <span>{engineStatus.name} v{engineStatus.version}</span>
            <button onClick={refreshEngineStatus} style={styles.refreshButton}>
              🔄 Refresh
            </button>
          </div>
        )}
      </div>

      {/* Error Display */}
      {error && (
        <div style={styles.errorBox}>
          ⚠️ {error}
        </div>
      )}

      {/* Volume Control */}
      <div style={styles.section}>
        <h3 style={styles.sectionTitle}>🔊 Master Volume</h3>
        <div style={styles.volumeControl}>
          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={volume}
            onChange={(e) => setVolume(parseFloat(e.target.value))}
            style={styles.slider}
            disabled={!isEngineReady}
          />
          <span style={styles.volumeLabel}>{Math.round(volume * 100)}%</span>
        </div>
        <div style={styles.presetButtons}>
          <button onClick={() => setVolume(0.0)} style={styles.presetButton} disabled={!isEngineReady}>
            Mute
          </button>
          <button onClick={() => setVolume(0.25)} style={styles.presetButton} disabled={!isEngineReady}>
            25%
          </button>
          <button onClick={() => setVolume(0.5)} style={styles.presetButton} disabled={!isEngineReady}>
            50%
          </button>
          <button onClick={() => setVolume(0.75)} style={styles.presetButton} disabled={!isEngineReady}>
            75%
          </button>
          <button onClick={() => setVolume(1.0)} style={styles.presetButton} disabled={!isEngineReady}>
            100%
          </button>
        </div>
      </div>

      {/* Quick Note Triggers */}
      <div style={styles.section}>
        <h3 style={styles.sectionTitle}>⚡ Quick Note Triggers (300ms)</h3>
        <div style={styles.noteGrid}>
          {middleCOctave.map((note) => (
            <button
              key={note}
              onClick={() => handleQuickNote(note)}
              style={styles.noteButton}
              disabled={!isEngineReady}
            >
              {getNoteName(note)}
            </button>
          ))}
        </div>
      </div>

      {/* Sustained Notes */}
      <div style={styles.section}>
        <h3 style={styles.sectionTitle}>🎹 Sustained Notes (Hold)</h3>
        <p style={styles.helperText}>Press and hold to play, release to stop (mouse or keyboard)</p>
        <div style={styles.noteGrid}>
          {middleCOctave.map((note) => (
            <button
              key={note}
              onMouseDown={() => handleNotePress(note)}
              onMouseUp={() => handleNoteRelease(note)}
              onMouseLeave={() => activeNotes.has(note) && handleNoteRelease(note)}
              onKeyDown={(e) => handleKeyDown(e, note)}
              onKeyUp={(e) => handleKeyUp(e, note)}
              style={{
                ...styles.noteButton,
                ...(activeNotes.has(note) ? styles.noteButtonActive : {}),
              }}
              disabled={!isEngineReady}
            >
              {getNoteName(note)}
            </button>
          ))}
        </div>
      </div>

      {/* Info */}
      <div style={styles.infoBox}>
        <h4>ℹ️ About This Test</h4>
        <ul style={styles.infoList}>
          <li><strong>Volume Control:</strong> Adjusts master volume via Tauri command <code>set_volume</code></li>
          <li><strong>Quick Triggers:</strong> Play notes with automatic note-off using <code>useNotePlayer</code> hook</li>
          <li><strong>Sustained Notes:</strong> Manual note on/off control with <code>playNote</code> and <code>stopNote</code></li>
          <li><strong>MIDI Range:</strong> Displaying notes 60-71 (C4-B4, middle C octave)</li>
        </ul>
      </div>
    </div>
  );
}

// Inline styles for simplicity
const styles = {
  container: {
    fontFamily: 'system-ui, -apple-system, sans-serif',
    maxWidth: '900px',
    margin: '0 auto',
    padding: '20px',
    backgroundColor: '#1a1a1a',
    color: '#ffffff',
    borderRadius: '8px',
  },
  title: {
    fontSize: '28px',
    fontWeight: 'bold',
    marginBottom: '20px',
    textAlign: 'center' as const,
  },
  statusContainer: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '20px',
    padding: '15px',
    backgroundColor: '#2a2a2a',
    borderRadius: '6px',
  },
  statusBadge: (ready: boolean) => ({
    padding: '8px 16px',
    borderRadius: '20px',
    backgroundColor: ready ? '#22c55e' : '#ef4444',
    color: '#ffffff',
    fontWeight: 'bold' as const,
    fontSize: '14px',
  }),
  statusInfo: {
    display: 'flex',
    gap: '10px',
    alignItems: 'center',
  },
  refreshButton: {
    padding: '6px 12px',
    backgroundColor: '#3b82f6',
    color: '#ffffff',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '12px',
  },
  errorBox: {
    padding: '12px',
    backgroundColor: '#dc2626',
    borderRadius: '4px',
    marginBottom: '20px',
    fontSize: '14px',
  },
  section: {
    marginBottom: '30px',
    padding: '20px',
    backgroundColor: '#2a2a2a',
    borderRadius: '6px',
  },
  sectionTitle: {
    fontSize: '20px',
    marginBottom: '15px',
    fontWeight: '600' as const,
  },
  volumeControl: {
    display: 'flex',
    alignItems: 'center',
    gap: '15px',
    marginBottom: '15px',
  },
  slider: {
    flex: 1,
    height: '8px',
    borderRadius: '4px',
    outline: 'none',
  },
  volumeLabel: {
    fontSize: '18px',
    fontWeight: 'bold' as const,
    minWidth: '60px',
    textAlign: 'right' as const,
  },
  presetButtons: {
    display: 'flex',
    gap: '10px',
  },
  presetButton: {
    padding: '8px 16px',
    backgroundColor: '#4a4a4a',
    color: '#ffffff',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '14px',
    transition: 'background-color 0.2s',
  },
  noteGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(80px, 1fr))',
    gap: '10px',
  },
  noteButton: {
    padding: '15px',
    backgroundColor: '#4a4a4a',
    color: '#ffffff',
    border: '2px solid #6a6a6a',
    borderRadius: '6px',
    cursor: 'pointer',
    fontSize: '16px',
    fontWeight: 'bold' as const,
    transition: 'all 0.15s',
    userSelect: 'none' as const,
  },
  noteButtonActive: {
    backgroundColor: '#3b82f6',
    borderColor: '#60a5fa',
    transform: 'scale(0.95)',
  },
  helperText: {
    fontSize: '14px',
    color: '#9ca3af',
    marginBottom: '10px',
  },
  infoBox: {
    marginTop: '30px',
    padding: '20px',
    backgroundColor: '#2a2a2a',
    borderRadius: '6px',
    borderLeft: '4px solid #3b82f6',
  },
  infoList: {
    lineHeight: '1.8',
    fontSize: '14px',
    color: '#d1d5db',
  },
};

export default DawEngineTest;
