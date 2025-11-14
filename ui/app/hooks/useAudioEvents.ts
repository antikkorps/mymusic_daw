/**
 * React hook for listening to real-time audio events from Tauri
 * Provides streaming data for MIDI notes, CPU usage, audio levels, etc.
 * 
 * Performance optimizations:
 * - Throttling for high-frequency events (CPU, audio levels)
 * - Debouncing for parameter changes
 * - Batched state updates
 * - Memory-efficient event handling
 */

import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { listen } from '@tauri-apps/api/event';

// Performance constants
const CPU_UPDATE_THROTTLE_MS = 100; // Update CPU display max 10 times per second
const AUDIO_LEVEL_THROTTLE_MS = 50; // Update audio levels max 20 times per second
const PARAMETER_DEBOUNCE_MS = 50; // Debounce parameter changes
const MAX_ERRORS_STORED = 50; // Limit error history
const BATCH_UPDATE_MS = 16; // ~60fps for UI updates

// Throttle utility function
function throttle<T extends (...args: any[]) => void>(
  func: T,
  delay: number
): T {
  let timeoutId: NodeJS.Timeout | null = null;
  let lastExecTime = 0;
  
  return ((...args: any[]) => {
    const currentTime = Date.now();
    
    if (currentTime - lastExecTime > delay) {
      func(...args);
      lastExecTime = currentTime;
    } else {
      if (timeoutId) {
        clearTimeout(timeoutId);
      }
      timeoutId = setTimeout(() => {
        func(...args);
        lastExecTime = Date.now();
      }, delay - (currentTime - lastExecTime));
    }
  }) as T;
}

// Debounce utility function
function debounce<T extends (...args: any[]) => void>(
  func: T,
  delay: number
): T {
  let timeoutId: NodeJS.Timeout | null = null;
  
  return ((...args: any[]) => {
    if (timeoutId) {
      clearTimeout(timeoutId);
    }
    timeoutId = setTimeout(() => func(...args), delay);
  }) as T;
}

// Event interfaces matching the Rust AudioEvent enum
export interface MidiNoteEvent {
  note: number;
  velocity: number;
  on: boolean;
  timestamp: number;
}

export interface ActiveVoicesEvent {
  count: number;
  timestamp: number;
}

export interface CpuUsageEvent {
  percentage: number;
  timestamp: number;
}

export interface AudioLevelEvent {
  left: number;
  right: number;
  peak_left: number;
  peak_right: number;
  timestamp: number;
}

export interface ParameterChangedEvent {
  parameter: string;
  value: any;
  timestamp: number;
}

export interface TransportPositionEvent {
  samples: number;
  musical_time: string;
  is_playing: boolean;
  tempo: number;
  timestamp: number;
}

export interface MetronomeTickEvent {
  beat: number;
  is_accent: boolean;
  timestamp: number;
}

export interface ErrorEvent {
  message: string;
  severity: 'warning' | 'error' | 'info';
  timestamp: number;
}

// Hook return type
interface UseAudioEvents {
  // Real-time state
  activeNotes: Set<number>;
  activeVoicesCount: number;
  cpuUsage: number;
  audioLevel: { left: number; right: number; peak_left: number; peak_right: number };
  transportPosition: {
    samples: number;
    musical_time: string;
    is_playing: boolean;
    tempo: number;
  };
  errors: ErrorEvent[];

  // Event handlers (for custom handling)
  onMidiNote?: (event: MidiNoteEvent) => void;
  onActiveVoices?: (event: ActiveVoicesEvent) => void;
  onCpuUsage?: (event: CpuUsageEvent) => void;
  onAudioLevel?: (event: AudioLevelEvent) => void;
  onParameterChanged?: (event: ParameterChangedEvent) => void;
  onTransportPosition?: (event: TransportPositionEvent) => void;
  onMetronomeTick?: (event: MetronomeTickEvent) => void;
  onError?: (event: ErrorEvent) => void;

  // Control
  isConnected: boolean;
  clearErrors: () => void;
}

/**
 * Custom hook to listen to real-time audio events from Tauri
 * 
 * Performance features:
 * - Throttled high-frequency updates
 * - Batched state updates for smooth UI
 * - Memory-efficient event handling
 * - Automatic cleanup and error recovery
 *
 * @example
 * ```tsx
 * function AudioMonitor() {
 *   const { activeNotes, cpuUsage, audioLevel, isConnected } = useAudioEvents();
 *
 *   return (
 *     <div>
 *       <div>CPU: {cpuUsage.toFixed(1)}%</div>
 *       <div>Active Notes: {activeNotes.size}</div>
 *       <div>Audio Level: L={audioLevel.left.toFixed(3)} R={audioLevel.right.toFixed(3)}</div>
 *       <div>Connected: {isConnected ? 'Yes' : 'No'}</div>
 *     </div>
 *   );
 * }
 * ```
 */
export function useAudioEvents(options: Partial<UseAudioEvents> = {}): UseAudioEvents {
  const [activeNotes, setActiveNotes] = useState<Set<number>>(new Set());
  const [activeVoicesCount, setActiveVoicesCount] = useState<number>(0);
  const [cpuUsage, setCpuUsage] = useState<number>(0);
  const [audioLevel, setAudioLevel] = useState({
    left: 0,
    right: 0,
    peak_left: 0,
    peak_right: 0,
  });
  const [transportPosition, setTransportPosition] = useState({
    samples: 0,
    musical_time: '1:1:0',
    is_playing: false,
    tempo: 120,
  });
  const [errors, setErrors] = useState<ErrorEvent[]>([]);
  const [isConnected, setIsConnected] = useState<boolean>(false);

  const unlistenFunctions = useRef<Array<() => void>>([]);
  const batchTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const pendingUpdates = useRef<Partial<{
    cpuUsage: number;
    audioLevel: typeof audioLevel;
    activeVoicesCount: number;
    transportPosition: typeof transportPosition;
  }>>({});

  // Clear errors function
  const clearErrors = useCallback(() => {
    setErrors([]);
  }, []);

  // Batch state updates for better performance
  const flushUpdates = useCallback(() => {
    if (Object.keys(pendingUpdates.current).length === 0) return;

    const updates = pendingUpdates.current;
    pendingUpdates.current = {};

    // Apply all pending updates in a single batch
    if (updates.cpuUsage !== undefined) {
      setCpuUsage(updates.cpuUsage);
    }
    if (updates.audioLevel !== undefined) {
      setAudioLevel(updates.audioLevel);
    }
    if (updates.activeVoicesCount !== undefined) {
      setActiveVoicesCount(updates.activeVoicesCount);
    }
    if (updates.transportPosition !== undefined) {
      setTransportPosition(updates.transportPosition);
    }
  }, []);

  // Throttled update functions
  const throttledCpuUpdate = useMemo(
    () => throttle((percentage: number) => {
      pendingUpdates.current.cpuUsage = percentage;
      if (!batchTimeoutRef.current) {
        batchTimeoutRef.current = setTimeout(flushUpdates, BATCH_UPDATE_MS);
      }
    }, CPU_UPDATE_THROTTLE_MS),
    [flushUpdates]
  );

  const throttledAudioLevelUpdate = useMemo(
    () => throttle((level: typeof audioLevel) => {
      pendingUpdates.current.audioLevel = level;
      if (!batchTimeoutRef.current) {
        batchTimeoutRef.current = setTimeout(flushUpdates, BATCH_UPDATE_MS);
      }
    }, AUDIO_LEVEL_THROTTLE_MS),
    [flushUpdates]
  );

  // Debounced parameter update
  const debouncedParameterUpdate = useMemo(
    () => debounce((paramEvent: ParameterChangedEvent) => {
      if (options.onParameterChanged) {
        options.onParameterChanged(paramEvent);
      }
    }, PARAMETER_DEBOUNCE_MS),
    [options.onParameterChanged]
  );

  // Setup event listeners
  useEffect(() => {
    const setupListeners = async () => {
      try {
        const unlistenFunctionsArray: Array<() => void> = [];

        // MIDI Note events
        const unlistenMidiNote = await listen<MidiNoteEvent>('audio:midi-note', (event) => {
          const noteEvent = event.payload;
          
          // Update active notes
          setActiveNotes(prev => {
            const newSet = new Set(prev);
            if (noteEvent.on) {
              newSet.add(noteEvent.note);
            } else {
              newSet.delete(noteEvent.note);
            }
            return newSet;
          });

          // Call custom handler if provided
          if (options.onMidiNote) {
            options.onMidiNote(noteEvent);
          }
        });
        unlistenFunctionsArray.push(unlistenMidiNote);

        // Active voices events
        const unlistenActiveVoices = await listen<ActiveVoicesEvent>('audio:active-voices', (event) => {
          const voicesEvent = event.payload;
          setActiveVoicesCount(voicesEvent.count);
          
          if (options.onActiveVoices) {
            options.onActiveVoices(voicesEvent);
          }
        });
        unlistenFunctionsArray.push(unlistenActiveVoices);

        // CPU usage events (throttled for performance)
        const unlistenCpuUsage = await listen<CpuUsageEvent>('audio:cpu-usage', (event) => {
          const cpuEvent = event.payload;
          
          // Use throttled update for UI
          throttledCpuUpdate(cpuEvent.percentage);
          
          // Call custom handler immediately if provided
          if (options.onCpuUsage) {
            options.onCpuUsage(cpuEvent);
          }
        });
        unlistenFunctionsArray.push(unlistenCpuUsage);

        // Audio level events (throttled for performance)
        const unlistenAudioLevel = await listen<AudioLevelEvent>('audio:level', (event) => {
          const levelEvent = event.payload;
          
          // Use throttled update for UI
          throttledAudioLevelUpdate({
            left: levelEvent.left,
            right: levelEvent.right,
            peak_left: levelEvent.peak_left,
            peak_right: levelEvent.peak_right,
          });
          
          // Call custom handler immediately if provided
          if (options.onAudioLevel) {
            options.onAudioLevel(levelEvent);
          }
        });
        unlistenFunctionsArray.push(unlistenAudioLevel);

        // Parameter changed events (debounced to prevent spam)
        const unlistenParameterChanged = await listen<ParameterChangedEvent>('audio:parameter-changed', (event) => {
          const paramEvent = event.payload;
          
          // Use debounced update for custom handler
          debouncedParameterUpdate(paramEvent);
        });
        unlistenFunctionsArray.push(unlistenParameterChanged);

        // Transport position events
        const unlistenTransportPosition = await listen<TransportPositionEvent>('audio:transport-position', (event) => {
          const transportEvent = event.payload;
          setTransportPosition({
            samples: transportEvent.samples,
            musical_time: transportEvent.musical_time,
            is_playing: transportEvent.is_playing,
            tempo: transportEvent.tempo,
          });
          
          if (options.onTransportPosition) {
            options.onTransportPosition(transportEvent);
          }
        });
        unlistenFunctionsArray.push(unlistenTransportPosition);

        // Metronome tick events
        const unlistenMetronomeTick = await listen<MetronomeTickEvent>('audio:metronome-tick', (event) => {
          const metroEvent = event.payload;
          
          if (options.onMetronomeTick) {
            options.onMetronomeTick(metroEvent);
          }
        });
        unlistenFunctionsArray.push(unlistenMetronomeTick);

        // Error events (with memory management)
        const unlistenError = await listen<ErrorEvent>('audio:error', (event) => {
          const errorEvent = event.payload;
          
          // Limit error history to prevent memory leaks
          setErrors(prev => {
            const newErrors = [...prev, errorEvent];
            return newErrors.length > MAX_ERRORS_STORED 
              ? newErrors.slice(-MAX_ERRORS_STORED)
              : newErrors;
          });
          
          // Call custom handler if provided
          if (options.onError) {
            options.onError(errorEvent);
          }
        });
        unlistenFunctionsArray.push(unlistenError);

        // Store unlisten functions
        unlistenFunctions.current = unlistenFunctionsArray;
        setIsConnected(true);

      } catch (error) {
        console.error('Failed to setup audio event listeners:', error);
        setIsConnected(false);
      }
    };

    setupListeners();

    // Cleanup function with enhanced error handling
    return () => {
      // Clear any pending batch updates
      if (batchTimeoutRef.current) {
        clearTimeout(batchTimeoutRef.current);
        batchTimeoutRef.current = null;
      }
      
      // Flush any pending updates before cleanup
      flushUpdates();
      
      // Unlisten all event listeners
      unlistenFunctions.current.forEach(unlisten => {
        try {
          unlisten();
        } catch (error) {
          console.error('Error during event listener cleanup:', error);
        }
      });
      
      // Reset connection state
      setIsConnected(false);
    };
  }, [options.onMidiNote, options.onActiveVoices, options.onCpuUsage, 
      options.onAudioLevel, options.onParameterChanged, options.onTransportPosition,
      options.onMetronomeTick, options.onError, throttledCpuUpdate, 
      throttledAudioLevelUpdate, debouncedParameterUpdate, flushUpdates]);

  return {
    activeNotes,
    activeVoicesCount,
    cpuUsage,
    audioLevel,
    transportPosition,
    errors,
    isConnected,
    clearErrors,
    ...options,
  };
}

/**
 * Hook for visualizing MIDI notes on a virtual keyboard
 */
export function useMidiKeyboard() {
  const { activeNotes } = useAudioEvents();

  // Convert MIDI note number to piano key name
  const getNoteName = (note: number): string => {
    const noteNames = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
    const octave = Math.floor(note / 12) - 1;
    const noteName = noteNames[note % 12];
    return `${noteName}${octave}`;
  };

  // Check if a note is currently active
  const isNoteActive = (note: number): boolean => {
    return activeNotes.has(note);
  };

  return {
    activeNotes,
    getNoteName,
    isNoteActive,
  };
}

/**
 * Hook for performance monitoring with enhanced metrics
 */
export function usePerformanceMonitor() {
  const { cpuUsage, activeVoicesCount, audioLevel, errors, isConnected } = useAudioEvents();

  // Performance metrics with memoization
  const performanceMetrics = useMemo(() => {
    // Get CPU status color
    const getCpuStatusColor = (): string => {
      if (cpuUsage < 50) return 'text-green-500';
      if (cpuUsage < 75) return 'text-yellow-500';
      return 'text-red-500';
    };

    // Get CPU status level
    const getCpuStatusLevel = (): 'good' | 'warning' | 'critical' => {
      if (cpuUsage < 50) return 'good';
      if (cpuUsage < 75) return 'warning';
      return 'critical';
    };

    // Get peak level
    const getPeakLevel = (): number => {
      return Math.max(audioLevel.peak_left, audioLevel.peak_right);
    };

    // Check if clipping
    const isClipping = (): boolean => {
      return getPeakLevel() > 0.95;
    };

    // Get audio level status
    const getAudioLevelStatus = (): 'normal' | 'hot' | 'clipping' => {
      const peak = getPeakLevel();
      if (peak > 0.95) return 'clipping';
      if (peak > 0.85) return 'hot';
      return 'normal';
    };

    // Get voice usage percentage (assuming 16 voice max)
    const getVoiceUsagePercentage = (): number => {
      return (activeVoicesCount / 16) * 100;
    };

    // Get overall system health
    const getSystemHealth = (): 'excellent' | 'good' | 'fair' | 'poor' => {
      const cpuLevel = getCpuStatusLevel();
      const audioStatus = getAudioLevelStatus();
      const voiceUsage = getVoiceUsagePercentage();
      const hasErrors = errors.length > 0;

      if (cpuLevel === 'critical' || audioStatus === 'clipping' || voiceUsage > 90 || hasErrors) {
        return 'poor';
      }
      if (cpuLevel === 'warning' || audioStatus === 'hot' || voiceUsage > 75) {
        return 'fair';
      }
      if (cpuUsage > 25 || voiceUsage > 50) {
        return 'good';
      }
      return 'excellent';
    };

    return {
      getCpuStatusColor,
      getCpuStatusLevel,
      getPeakLevel,
      isClipping,
      getAudioLevelStatus,
      getVoiceUsagePercentage,
      getSystemHealth,
    };
  }, [cpuUsage, activeVoicesCount, audioLevel, errors]);

  return {
    cpuUsage,
    activeVoicesCount,
    audioLevel,
    errors,
    isConnected,
    ...performanceMetrics,
  };
}

/**
 * Hook for optimized event debugging and monitoring
 */
export function useEventDebugMonitor() {
  const { isConnected, errors } = useAudioEvents();
  
  const [eventStats, setEventStats] = useState({
    totalEvents: 0,
    eventsPerSecond: 0,
    lastEventTime: 0,
  });

  const eventCountRef = useRef(0);
  const lastSecondRef = useRef(Date.now());

  // Track event statistics
  const trackEvent = useCallback(() => {
    eventCountRef.current++;
    const now = Date.now();
    
    // Calculate events per second every second
    if (now - lastSecondRef.current >= 1000) {
      setEventStats({
        totalEvents: eventCountRef.current,
        eventsPerSecond: eventCountRef.current,
        lastEventTime: now,
      });
      eventCountRef.current = 0;
      lastSecondRef.current = now;
    }
  }, []);

  // Performance status
  const performanceStatus = useMemo(() => {
    if (!isConnected) return 'disconnected';
    if (eventStats.eventsPerSecond > 1000) return 'high-load';
    if (eventStats.eventsPerSecond > 500) return 'moderate-load';
    if (errors.length > 5) return 'error-prone';
    return 'healthy';
  }, [isConnected, eventStats.eventsPerSecond, errors.length]);

  return {
    eventStats,
    performanceStatus,
    trackEvent,
    isConnected,
    errors,
  };
}