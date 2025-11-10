/**
 * React hook for MyMusic DAW Engine
 * Provides functions to control the audio engine via Tauri commands
 */

import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

// Engine info interface (matches Rust struct)
export interface EngineInfo {
  name: string;
  version: string;
  status: string;
  audio_engine: string;
  sample_rate: number;
  buffer_size: number;
}

// Hook return type
interface UseDawEngine {
  // State
  engineInfo: EngineInfo | null;
  waveforms: string[];
  isLoading: boolean;
  error: string | null;

  // Actions
  playTestBeep: () => Promise<void>;
  refreshEngineInfo: () => Promise<void>;
}

/**
 * Custom hook to interact with the DAW audio engine
 *
 * @example
 * ```tsx
 * function EngineStatus() {
 *   const { engineInfo, playTestBeep } = useDawEngine();
 *
 *   return (
 *     <div>
 *       <p>Engine: {engineInfo?.name} v{engineInfo?.version}</p>
 *       <button onClick={playTestBeep}>Test Beep</button>
 *     </div>
 *   );
 * }
 * ```
 */
export function useDawEngine(): UseDawEngine {
  const [engineInfo, setEngineInfo] = useState<EngineInfo | null>(null);
  const [waveforms, setWaveforms] = useState<string[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  /**
   * Play a test beep sound
   */
  const playTestBeep = useCallback(async () => {
    try {
      const result = await invoke<string>('play_test_beep');
      console.log('✅', result);
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(`Failed to play test beep: ${errorMessage}`);
      console.error('❌ Failed to play test beep:', err);
    }
  }, []);

  /**
   * Fetch the current engine info
   */
  const refreshEngineInfo = useCallback(async () => {
    try {
      setIsLoading(true);
      const info = await invoke<EngineInfo>('get_engine_info');
      setEngineInfo(info);
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(`Failed to get engine info: ${errorMessage}`);
      console.error('❌ Failed to get engine info:', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  /**
   * Fetch available waveforms
   */
  const fetchWaveforms = useCallback(async () => {
    try {
      const waveformList = await invoke<string[]>('get_waveforms');
      setWaveforms(waveformList);
    } catch (err) {
      console.error('❌ Failed to get waveforms:', err);
    }
  }, []);

  /**
   * Initialize the hook - fetch engine info and waveforms
   */
  useEffect(() => {
    async function initialize() {
      await refreshEngineInfo();
      await fetchWaveforms();
    }

    initialize();
  }, [refreshEngineInfo, fetchWaveforms]);

  return {
    engineInfo,
    waveforms,
    isLoading,
    error,
    playTestBeep,
    refreshEngineInfo,
  };
}
