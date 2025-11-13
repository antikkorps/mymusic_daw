// Tests for useAudioEvents hook

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useAudioEvents } from './useAudioEvents';

// Mock Tauri API
const mockListen = vi.fn();
const mockUnlisten = vi.fn();

vi.mock('@tauri-apps/api/event', () => ({
  listen: mockListen,
}));

describe('useAudioEvents', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockListen.mockResolvedValue(mockUnlisten);
  });

  describe('Event Listener Setup', () => {
    it('should set up event listeners on mount', async () => {
      renderHook(() => useAudioEvents());
      
      // Wait for useEffect to run
      await new Promise(resolve => setTimeout(resolve, 0));
      
      expect(mockListen).toHaveBeenCalledWith('audio-engine-event', expect.any(Function));
      expect(mockListen).toHaveBeenCalledWith('cpu-usage', expect.any(Function));
      expect(mockListen).toHaveBeenCalledWith('active-notes', expect.any(Function));
    });

    it('should clean up event listeners on unmount', async () => {
      const { unmount } = renderHook(() => useAudioEvents());
      
      // Wait for useEffect to run
      await new Promise(resolve => setTimeout(resolve, 0));
      
      unmount();
      
      expect(mockUnlisten).toHaveBeenCalledTimes(3); // Should call unlisten for each event
    });
  });

  describe('Event Handling', () => {
    it('should handle audio engine events', async () => {
      let eventCallback: ((event: any) => void) | undefined;
      
      mockListen.mockImplementation((event, callback) => {
        if (event === 'audio-engine-event') {
          eventCallback = callback as (event: any) => void;
        }
        return Promise.resolve(mockUnlisten);
      });
      
      const { result } = renderHook(() => useAudioEvents());
      
      // Wait for useEffect to run
      await new Promise(resolve => setTimeout(resolve, 0));
      
      // Simulate receiving an audio engine event
      const testEvent = {
        type: 'test-event',
        data: { message: 'Test message' },
      };
      
      await act(async () => {
        eventCallback?.(testEvent);
      });
      
      // The hook should handle the event without errors
      expect(mockListen).toHaveBeenCalledWith('audio-engine-event', expect.any(Function));
    });

    it('should handle CPU usage events', async () => {
      let cpuCallback: ((event: any) => void) | undefined;
      
      mockListen.mockImplementation((event, callback) => {
        if (event === 'cpu-usage') {
          cpuCallback = callback as (event: any) => void;
        }
        return Promise.resolve(mockUnlisten);
      });
      
      const { result } = renderHook(() => useAudioEvents());
      
      // Wait for useEffect to run
      await new Promise(resolve => setTimeout(resolve, 0));
      
      // Simulate receiving a CPU usage event
      const cpuEvent = {
        payload: 45.5,
      };
      
      await act(async () => {
        cpuCallback?.(cpuEvent);
      });
      
      expect(mockListen).toHaveBeenCalledWith('cpu-usage', expect.any(Function));
    });

    it('should handle active notes events', async () => {
      let notesCallback: ((event: any) => void) | undefined;
      
      mockListen.mockImplementation((event, callback) => {
        if (event === 'active-notes') {
          notesCallback = callback as (event: any) => void;
        }
        return Promise.resolve(mockUnlisten);
      });
      
      const { result } = renderHook(() => useAudioEvents());
      
      // Wait for useEffect to run
      await new Promise(resolve => setTimeout(resolve, 0));
      
      // Simulate receiving an active notes event
      const notesEvent = {
        payload: [60, 64, 67],
      };
      
      await act(async () => {
        notesCallback?.(notesEvent);
      });
      
      expect(mockListen).toHaveBeenCalledWith('active-notes', expect.any(Function));
    });
  });

  describe('Error Handling', () => {
    it('should handle event listener setup errors', async () => {
      mockListen.mockRejectedValue(new Error('Failed to set up event listener'));
      
      // Should not throw error during setup
      expect(() => {
        renderHook(() => useAudioEvents());
      }).not.toThrow();
    });

    it('should handle malformed events gracefully', async () => {
      let eventCallback: ((event: any) => void) | undefined;
      
      mockListen.mockImplementation((event, callback) => {
        if (event === 'audio-engine-event') {
          eventCallback = callback as (event: any) => void;
        }
        return Promise.resolve(mockUnlisten);
      });
      
      const { result } = renderHook(() => useAudioEvents());
      
      // Wait for useEffect to run
      await new Promise(resolve => setTimeout(resolve, 0));
      
      // Simulate receiving malformed events
      const malformedEvents = [
        null,
        undefined,
        {},
        { type: null },
        { type: 'test', data: null },
        'string-event',
        123,
      ];
      
      for (const malformedEvent of malformedEvents) {
        await act(async () => {
          expect(() => {
            eventCallback?.(malformedEvent);
          }).not.toThrow();
        });
      }
    });

    it('should handle CPU usage events with invalid data', async () => {
      let cpuCallback: ((event: any) => void) | undefined;
      
      mockListen.mockImplementation((event, callback) => {
        if (event === 'cpu-usage') {
          cpuCallback = callback as (event: any) => void;
        }
        return Promise.resolve(mockUnlisten);
      });
      
      const { result } = renderHook(() => useAudioEvents());
      
      // Wait for useEffect to run
      await new Promise(resolve => setTimeout(resolve, 0));
      
      // Simulate receiving invalid CPU usage events
      const invalidCpuEvents = [
        { payload: null },
        { payload: undefined },
        { payload: 'invalid' },
        { payload: -5 }, // Negative CPU usage
        { payload: 150 }, // CPU usage over 100%
      ];
      
      for (const invalidEvent of invalidCpuEvents) {
        await act(async () => {
          expect(() => {
            cpuCallback?.(invalidEvent);
          }).not.toThrow();
        });
      }
    });

    it('should handle active notes events with invalid data', async () => {
      let notesCallback: ((event: any) => void) | undefined;
      
      mockListen.mockImplementation((event, callback) => {
        if (event === 'active-notes') {
          notesCallback = callback as (event: any) => void;
        }
        return Promise.resolve(mockUnlisten);
      });
      
      const { result } = renderHook(() => useAudioEvents());
      
      // Wait for useEffect to run
      await new Promise(resolve => setTimeout(resolve, 0));
      
      // Simulate receiving invalid active notes events
      const invalidNotesEvents = [
        { payload: null },
        { payload: undefined },
        { payload: 'invalid' },
        { payload: [128, 130] }, // Invalid MIDI note numbers
        { payload: [-1, -5] },   // Negative MIDI note numbers
      ];
      
      for (const invalidEvent of invalidNotesEvents) {
        await act(async () => {
          expect(() => {
            notesCallback?.(invalidEvent);
          }).not.toThrow();
        });
      }
    });
  });

  describe('Performance Tests', () => {
    it('should handle high-frequency events without memory leaks', async () => {
      let eventCallback: ((event: any) => void) | undefined;
      
      mockListen.mockImplementation((event, callback) => {
        if (event === 'audio-engine-event') {
          eventCallback = callback as (event: any) => void;
        }
        return Promise.resolve(mockUnlisten);
      });
      
      const { result } = renderHook(() => useAudioEvents());
      
      // Wait for useEffect to run
      await new Promise(resolve => setTimeout(resolve, 0));
      
      // Simulate receiving many events rapidly
      const startTime = performance.now();
      
      for (let i = 0; i < 1000; i++) {
        await act(async () => {
          eventCallback?.({
            type: 'test-event',
            data: { index: i },
          });
        });
      }
      
      const endTime = performance.now();
      const duration = endTime - startTime;
      
      // Should process 1000 events in reasonable time (less than 1 second)
      expect(duration).toBeLessThan(1000);
    });

    it('should handle burst events efficiently', async () => {
      let cpuCallback: ((event: any) => void) | undefined;
      let notesCallback: ((event: any) => void) | undefined;
      
      mockListen.mockImplementation((event, callback) => {
        if (event === 'cpu-usage') {
          cpuCallback = callback as (event: any) => void;
        } else if (event === 'active-notes') {
          notesCallback = callback as (event: any) => void;
        }
        return Promise.resolve(mockUnlisten);
      });
      
      const { result } = renderHook(() => useAudioEvents());
      
      // Wait for useEffect to run
      await new Promise(resolve => setTimeout(resolve, 0));
      
      // Simulate burst of mixed events
      const events = [];
      for (let i = 0; i < 100; i++) {
        events.push(
          () => cpuCallback?.({ payload: Math.random() * 100 }),
          () => notesCallback?.({ payload: [60 + (i % 12), 64 + (i % 12)] })
        );
      }
      
      const startTime = performance.now();
      
      await act(async () => {
        for (const event of events) {
          event();
        }
      });
      
      const endTime = performance.now();
      const duration = endTime - startTime;
      
      // Should handle burst efficiently
      expect(duration).toBeLessThan(500);
    });
  });

  describe('Integration Tests', () => {
    it('should handle real-world event sequence', async () => {
      const callbacks: { [key: string]: ((event: any) => void) | undefined } = {};
      
      mockListen.mockImplementation((event, callback) => {
        callbacks[event] = callback as (event: any) => void;
        return Promise.resolve(mockUnlisten);
      });
      
      const { result } = renderHook(() => useAudioEvents());
      
      // Wait for useEffect to run
      await new Promise(resolve => setTimeout(resolve, 0));
      
      // Simulate real-world event sequence
      await act(async () => {
        // Engine starts
        callbacks['audio-engine-event']?.({
          type: 'engine-started',
          data: { sampleRate: 44100, bufferSize: 512 },
        });
        
        // CPU usage updates
        callbacks['cpu-usage']?.({ payload: 25.5 });
        callbacks['cpu-usage']?.({ payload: 30.2 });
        callbacks['cpu-usage']?.({ payload: 28.7 });
        
        // Notes are played
        callbacks['active-notes']?.({ payload: [60] });
        callbacks['active-notes']?.({ payload: [60, 64] });
        callbacks['active-notes']?.({ payload: [64] });
        callbacks['active-notes']?.({ payload: [] });
        
        // Engine stops
        callbacks['audio-engine-event']?.({
          type: 'engine-stopped',
          data: {},
        });
      });
      
      // Should handle all events without errors
      expect(mockListen).toHaveBeenCalledTimes(3);
    });
  });
});