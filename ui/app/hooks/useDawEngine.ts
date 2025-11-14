/**
 * React hook for MyMusic DAW Engine
 * Provides functions to control the audio engine via Tauri commands
 * Enhanced with comprehensive error handling and user feedback
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import { useErrorReporting } from '../components/ErrorHandling';

// Access Tauri API directly from window object (safer than imports)
const invoke = typeof window !== 'undefined' && (window as any).__TAURI__
  ? (window as any).__TAURI__.core.invoke
  : undefined;

/**
 * Wrapper function for Tauri invoke calls with centralized error handling
 * and automatic user notification
 */
async function invokeWithErrorHandling<T>(
  command: string,
  args?: any,
  errorContext?: string,
  reportError?: (message: string, type: 'error' | 'warning', source: 'audio' | 'midi' | 'ui' | 'system', details?: any) => void
): Promise<T> {
  // Check if Tauri is available
  if (!invoke) {
    throw new Error('Tauri API not available. Please run the app with "npm run tauri dev" or build the native app.');
  }

  try {
    const result = await invoke<T>(command, args);
    return result;
  } catch (err) {
    const errorMessage = err instanceof Error ? err.message : String(err);
    const context = errorContext || command;
    const fullErrorMessage = `Failed to ${context}: ${errorMessage}`;
    
    // Report error to UI if error reporting function is available
    if (reportError) {
      // Determine error source based on command
      let source: 'audio' | 'midi' | 'ui' | 'system' = 'system';
      if (command.includes('midi') || command.includes('note')) {
        source = 'midi';
      } else if (command.includes('volume') || command.includes('waveform') || command.includes('filter')) {
        source = 'audio';
      } else if (command.includes('ui') || command.includes('interface')) {
        source = 'ui';
      }
      
      reportError(fullErrorMessage, 'error', source, {
        command,
        args,
        originalError: errorMessage,
      });
    }
    
    throw new Error(fullErrorMessage);
  }
}

// Engine status interface
interface EngineStatus {
  name: string;
  version: string;
  status: string;
}

// Synthesizer parameter interfaces
export interface AdsrParams {
  attack: number;
  decay: number;
  sustain: number;
  release: number;
}

export interface LfoParams {
  waveform: 'sine' | 'square' | 'saw' | 'triangle';
  rate: number;
  depth: number;
  destination: 'pitch' | 'amplitude' | 'filter';
}

export interface FilterParams {
  filter_type: 'lowpass' | 'highpass' | 'bandpass' | 'notch';
  cutoff: number;
  resonance: number;
}

export interface ModRoutingParams {
  index: number;
  source: 'lfo' | 'velocity' | 'aftertouch' | 'envelope';
  destination: 'pitch' | 'amplitude' | 'filter' | 'pan';
  amount: number;
}

// Hook return type
interface UseDawEngine {
  // State
  volume: number;
  isEngineReady: boolean;
  engineStatus: EngineStatus | null;
  error: string | null;

  // Basic actions
  setVolume: (volume: number) => Promise<void>;
  playNote: (note: number, velocity: number) => Promise<void>;
  stopNote: (note: number) => Promise<void>;
  refreshEngineStatus: () => Promise<void>;

  // Synthesizer parameters
  setWaveform: (waveform: 'sine' | 'square' | 'saw' | 'triangle') => Promise<void>;
  setAdsr: (params: AdsrParams) => Promise<void>;
  setLfo: (params: LfoParams) => Promise<void>;
  setFilter: (params: FilterParams) => Promise<void>;
  setPolyMode: (mode: 'poly' | 'mono' | 'legato') => Promise<void>;
  setPortamento: (time: number) => Promise<void>;
  setVoiceMode: (mode: 'synth' | 'sampler') => Promise<void>;
  setModRouting: (params: ModRoutingParams) => Promise<void>;
  clearModRouting: (index: number) => Promise<void>;
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
  const isInitialized = useRef<boolean>(false);
  
  // Error reporting integration
  const { reportError, reportSuccess, reportAudioError, reportMidiError } = useErrorReporting();

  /**
   * Set the master volume
   * @param newVolume - Volume level between 0.0 and 1.0
   */
  const setVolume = useCallback(async (newVolume: number) => {
    try {
      // Clamp volume to valid range
      const clampedVolume = Math.max(0, Math.min(1, newVolume));

      await invokeWithErrorHandling('set_volume', { volume: clampedVolume }, 'set volume', reportError);
      setVolumeState(clampedVolume);
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
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

      await invokeWithErrorHandling('play_note', { note, velocity }, 'play note', reportMidiError);
      setError(null);
      reportSuccess(`Note ${note} played with velocity ${velocity}`);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      console.error('Failed to play note:', err);
    }
  }, [reportMidiError, reportSuccess]);

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

      await invokeWithErrorHandling('stop_note', { note }, 'stop note', reportMidiError);
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      console.error('Failed to stop note:', err);
    }
  }, [reportMidiError]);

  /**
   * Fetch the current engine status
   */
  const refreshEngineStatus = useCallback(async () => {
    try {
      const status = await invokeWithErrorHandling<EngineStatus>('get_engine_status', undefined, 'get engine status');
      setEngineStatus(status);
      setIsEngineReady(status.status === 'running');
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      console.error('Failed to get engine status:', err);
      setIsEngineReady(false);
    }
  }, []);

  /**
   * Set oscillator waveform type
   * @param waveform - Waveform type ('sine', 'square', 'saw', 'triangle')
   */
  const setWaveform = useCallback(async (waveform: 'sine' | 'square' | 'saw' | 'triangle') => {
    try {
      await invokeWithErrorHandling('set_waveform', { waveform }, 'set waveform');
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      console.error('Failed to set waveform:', err);
    }
  }, []);

  /**
   * Set ADSR envelope parameters
   * @param params - ADSR parameters
   */
  const setAdsr = useCallback(async (params: AdsrParams) => {
    try {
      await invokeWithErrorHandling('set_adsr', params, 'set ADSR');
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      console.error('Failed to set ADSR:', err);
    }
  }, []);

  /**
   * Set LFO parameters
   * @param params - LFO parameters
   */
  const setLfo = useCallback(async (params: LfoParams) => {
    try {
      await invokeWithErrorHandling('set_lfo', params, 'set LFO');
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      console.error('Failed to set LFO:', err);
    }
  }, []);

  /**
   * Set filter parameters
   * @param params - Filter parameters
   */
  const setFilter = useCallback(async (params: FilterParams) => {
    try {
      await invokeWithErrorHandling('set_filter', params, 'set filter');
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      console.error('Failed to set filter:', err);
    }
  }, []);

  /**
   * Set polyphony mode
   * @param mode - Polyphony mode ('poly', 'mono', 'legato')
   */
  const setPolyMode = useCallback(async (mode: 'poly' | 'mono' | 'legato') => {
    try {
      await invokeWithErrorHandling('set_poly_mode', { mode }, 'set poly mode');
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      console.error('Failed to set poly mode:', err);
    }
  }, []);

  /**
   * Set portamento (glide) time
   * @param time - Glide time in seconds
   */
  const setPortamento = useCallback(async (time: number) => {
    try {
      await invokeWithErrorHandling('set_portamento', { time }, 'set portamento');
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      console.error('Failed to set portamento:', err);
    }
  }, []);

  /**
   * Set voice mode (synth vs sampler)
   * @param mode - Voice mode ('synth', 'sampler')
   */
  const setVoiceMode = useCallback(async (mode: 'synth' | 'sampler') => {
    try {
      await invokeWithErrorHandling('set_voice_mode', { mode }, 'set voice mode');
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      console.error('Failed to set voice mode:', err);
    }
  }, []);

  /**
   * Set modulation routing
   * @param params - Modulation routing parameters
   */
  const setModRouting = useCallback(async (params: ModRoutingParams) => {
    try {
      await invokeWithErrorHandling('set_mod_routing', params, 'set mod routing');
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      console.error('Failed to set mod routing:', err);
    }
  }, []);

  /**
   * Clear modulation routing
   * @param index - Routing index to clear
   */
  const clearModRouting = useCallback(async (index: number) => {
    try {
      await invokeWithErrorHandling('clear_mod_routing', { index }, 'clear mod routing');
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      console.error('Failed to clear mod routing:', err);
    }
  }, []);

  /**
   * Initialize the hook - fetch initial volume and engine status
   */
  useEffect(() => {
    // Prevent re-initialization
    if (isInitialized.current) {
      return;
    }

    async function initialize() {
      try {
        // Get initial volume
        const currentVolume = await invokeWithErrorHandling<number>('get_volume', undefined, 'get initial volume');
        setVolumeState(currentVolume);

        // Get engine status
        const status = await invokeWithErrorHandling<EngineStatus>('get_engine_status', undefined, 'get engine status');
        setEngineStatus(status);
        setIsEngineReady(status.status === 'running');
        setError(null);

        // Mark as initialized
        isInitialized.current = true;
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        console.error('Failed to initialize DAW engine:', err);
        setError(errorMessage);
      }
    }

    initialize();
  }, []);

  return {
    volume,
    isEngineReady,
    engineStatus,
    error,
    setVolume,
    playNote,
    stopNote,
    refreshEngineStatus,
    setWaveform,
    setAdsr,
    setLfo,
    setFilter,
    setPolyMode,
    setPortamento,
    setVoiceMode,
    setModRouting,
    clearModRouting,
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
