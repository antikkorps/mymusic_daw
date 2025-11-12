/**
 * React hook for MyMusic DAW Engine
 * Provides functions to control the audio engine via Tauri commands
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';

// Engine status interface
interface EngineStatus {
  name: string;
  version: string;
  status: string;
}

// Hook return type
interface UseDawEngine {
  // State
  volume: number;
  isEngineReady: boolean;
  engineStatus: EngineStatus | null;
  error: string | null;

  // Actions
  setVolume: (volume: number) => Promise<void>;
  playNote: (note: number, velocity: number) => Promise<void>;
  stopNote: (note: number) => Promise<void>;
  refreshEngineStatus: () => Promise<void>;
}

/**
 * Custom hook to interact with the DAW audio engine
 *
 * @example
 * ```tsx
 * function VolumeControl() {
 *   const { volume, setVolume } = useDawEngine();
 *
 *   return (
 *     <input
 *       type="range"
 *       min="0"
 *       max="1"
 *       step="0.01"
 *       value={volume}
 *       onChange={(e) => setVolume(parseFloat(e.target.value))}
 *     />
 *   );
 * }
 * ```
 */
export function useDawEngine(): UseDawEngine {
  const [volume, setVolumeState] = useState<number>(0.5);
  const [isEngineReady, setIsEngineReady] = useState<boolean>(false);
  const [engineStatus, setEngineStatus] = useState<EngineStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  /**
   * Set the master volume
   * @param newVolume - Volume level between 0.0 and 1.0
   */
  const setVolume = useCallback(async (newVolume: number) => {
    try {
      // Clamp volume to valid range
      const clampedVolume = Math.max(0, Math.min(1, newVolume));

      await invoke('set_volume', { volume: clampedVolume });
      setVolumeState(clampedVolume);
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(`Failed to set volume: ${errorMessage}`);
      console.error('Failed to set volume:', err);
    }
  }, []);

  /**
   * Play a MIDI note
   * @param note - MIDI note number (0-127)
   * @param velocity - Note velocity (0-127)
   */
  const playNote = useCallback(async (note: number, velocity: number = 100) => {
    try {
      // Validate inputs
      if (note < 0 || note > 127) {
        throw new Error(`Invalid note number: ${note} (must be 0-127)`);
      }
      if (velocity < 0 || velocity > 127) {
        throw new Error(`Invalid velocity: ${velocity} (must be 0-127)`);
      }

      await invoke('play_note', { note, velocity });
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(`Failed to play note: ${errorMessage}`);
      console.error('Failed to play note:', err);
    }
  }, []);

  /**
   * Stop a MIDI note
   * @param note - MIDI note number (0-127)
   */
  const stopNote = useCallback(async (note: number) => {
    try {
      // Validate input
      if (note < 0 || note > 127) {
        throw new Error(`Invalid note number: ${note} (must be 0-127)`);
      }

      await invoke('stop_note', { note });
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(`Failed to stop note: ${errorMessage}`);
      console.error('Failed to stop note:', err);
    }
  }, []);

  /**
   * Fetch the current engine status
   */
  const refreshEngineStatus = useCallback(async () => {
    try {
      const status = await invoke<EngineStatus>('get_engine_status');
      setEngineStatus(status);
      setIsEngineReady(status.status === 'running');
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(`Failed to get engine status: ${errorMessage}`);
      console.error('Failed to get engine status:', err);
      setIsEngineReady(false);
    }
  }, []);

  /**
   * Initialize the hook - fetch initial volume and engine status
   */
  useEffect(() => {
    async function initialize() {
      try {
        // Get initial volume
        const currentVolume = await invoke<number>('get_volume');
        setVolumeState(currentVolume);

        // Get engine status
        await refreshEngineStatus();
      } catch (err) {
        console.error('Failed to initialize DAW engine:', err);
        setError('Failed to initialize DAW engine');
      }
    }

    initialize();
  }, [refreshEngineStatus]);

  return {
    volume,
    isEngineReady,
    engineStatus,
    error,
    setVolume,
    playNote,
    stopNote,
    refreshEngineStatus,
  };
}

/**
 * Helper hook for playing notes with automatic note-off
 * Useful for button-based note triggers
 *
 * @example
 * ```tsx
 * function PianoKey({ note }: { note: number }) {
 *   const { triggerNote } = useNotePlayer();
 *
 *   return (
 *     <button onClick={() => triggerNote(note, 100, 500)}>
 *       Play Note {note}
 *     </button>
 *   );
 * }
 * ```
 */
export function useNotePlayer() {
  const { playNote, stopNote } = useDawEngine();
  const timeoutIdsRef = useRef<Set<NodeJS.Timeout>>(new Set());

  /**
   * Play a note and automatically stop it after a duration
   * @param note - MIDI note number (0-127)
   * @param velocity - Note velocity (0-127)
   * @param duration - Duration in milliseconds
   */
  const triggerNote = useCallback(
    async (note: number, velocity: number = 100, duration: number = 500) => {
      await playNote(note, velocity);

      const timeoutId = setTimeout(() => {
        stopNote(note);
        timeoutIdsRef.current.delete(timeoutId);
      }, duration);

      timeoutIdsRef.current.add(timeoutId);
    },
    [playNote, stopNote]
  );

  /**
   * Clean up all pending timeouts when component unmounts
   */
  useEffect(() => {
    return () => {
      timeoutIdsRef.current.forEach((timeoutId) => {
        clearTimeout(timeoutId);
      });
      timeoutIdsRef.current.clear();
    };
  }, []);

  return {
    triggerNote,
    playNote,
    stopNote,
  };
}
