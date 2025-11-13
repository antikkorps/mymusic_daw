/**
 * Modern Synthesizer Controls Demo
 * Tests the complete Tauri bridge with real-time audio events
 */

import { useState } from 'react';
import { useDawEngine } from '../hooks/useDawEngine';
import { useAudioEvents, useMidiKeyboard, usePerformanceMonitor } from '../hooks/useAudioEvents';

export default function SynthesizerDemo() {
  const [waveform, setWaveformState] = useState<'sine' | 'square' | 'saw' | 'triangle'>('sine');
  const [adsr, setAdsrState] = useState({
    attack: 0.01,
    decay: 0.1,
    sustain: 0.7,
    release: 0.2,
  });
  const [lfo, setLfoState] = useState({
    waveform: 'sine' as const,
    rate: 2.0,
    depth: 0.1,
    destination: 'pitch' as const,
  });
  const [filter, setFilterState] = useState({
    filter_type: 'lowpass' as const,
    cutoff: 1000,
    resonance: 0.7,
  });
  const [polyMode, setPolyModeState] = useState<'poly' | 'mono' | 'legato'>('poly');

  // DAW engine controls
  const { 
    volume, 
    setVolume, 
    setWaveform, 
    setAdsr, 
    setLfo, 
    setFilter, 
    setPolyMode, 
    playNote,
    stopNote,
    isEngineReady 
  } = useDawEngine();

  // Real-time audio events
  const { activeNotes, cpuUsage, audioLevel } = useAudioEvents();
  const { getNoteName, isNoteActive } = useMidiKeyboard();
  const { getCpuStatusColor, isClipping } = usePerformanceMonitor();

  // UI Handlers
  const handleWaveformChange = (newWaveform: typeof waveform) => {
    setWaveformState(newWaveform);
    setWaveform(newWaveform);
  };

  const handleAdsrChange = (param: keyof typeof adsr, value: number) => {
    const newAdsr = { ...adsr, [param]: value };
    setAdsrState(newAdsr);
    setAdsr(newAdsr);
  };

  const handleLfoChange = (param: keyof typeof lfo, value: any) => {
    const newLfo = { ...lfo, [param]: value };
    setLfoState(newLfo);
    setLfo(newLfo);
  };

  const handleFilterChange = (param: keyof typeof filter, value: any) => {
    const newFilter = { ...filter, [param]: value };
    setFilterState(newFilter);
    setFilter(newFilter);
  };

  const handlePolyModeChange = (newMode: typeof polyMode) => {
    setPolyModeState(newMode);
    setPolyMode(newMode);
  };

  

  // Virtual keyboard handlers
  const handleKeyDown = (note: number) => {
    playNote(note, 100);
  };

  const handleKeyUp = (note: number) => {
    stopNote(note);
  };

  // Piano keyboard mapping (simplified)
  const pianoKeys = [
    { note: 60, label: 'C4', key: 'a' },
    { note: 62, label: 'D4', key: 's' },
    { note: 64, label: 'E4', key: 'd' },
    { note: 65, label: 'F4', key: 'f' },
    { note: 67, label: 'G4', key: 'g' },
    { note: 69, label: 'A4', key: 'h' },
    { note: 71, label: 'B4', key: 'j' },
    { note: 72, label: 'C5', key: 'k' },
  ];

  if (!isEngineReady) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-gray-900 text-white">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-t-2 border-blue-500 mx-auto mb-4"></div>
          <p className="text-xl">Initializing DAW Engine...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-900 text-white p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-3xl font-bold mb-2">🎛️ MyMusic DAW - Synthesizer Demo</h1>
          <p className="text-gray-400">Complete Tauri bridge test with real-time audio events</p>
        </div>

        {/* Performance Monitor */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-8">
          <div className="bg-gray-800 rounded-lg p-4">
            <h3 className="text-sm font-medium mb-2 text-gray-400">CPU Usage</h3>
            <div className={`text-2xl font-bold ${getCpuStatusColor()}`}>
              {cpuUsage.toFixed(1)}%
            </div>
          </div>
          
          <div className="bg-gray-800 rounded-lg p-4">
            <h3 className="text-sm font-medium mb-2 text-gray-400">Active Voices</h3>
            <div className="text-2xl font-bold text-blue-400">
              {activeNotes.size}
            </div>
          </div>
          
          <div className="bg-gray-800 rounded-lg p-4">
            <h3 className="text-sm font-medium mb-2 text-gray-400">Audio Level</h3>
            <div className={`text-2xl font-bold ${isClipping() ? 'text-red-500' : 'text-green-400'}`}>
              L:{audioLevel.left.toFixed(3)} R:{audioLevel.right.toFixed(3)}
            </div>
          </div>
        </div>

        {/* Main Controls */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-8 mb-8">
          {/* Left Column - Basic Controls */}
          <div className="space-y-6">
            {/* Volume Control */}
            <div className="bg-gray-800 rounded-lg p-4">
              <h3 className="text-lg font-semibold mb-4">Master Volume</h3>
              <input
                type="range"
                min="0"
                max="1"
                step="0.01"
                value={volume}
                onChange={(e) => setVolume(parseFloat(e.target.value))}
                className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer"
              />
              <div className="text-center mt-2 text-sm text-gray-400">
                {(volume * 100).toFixed(0)}%
              </div>
            </div>

            {/* Waveform Selector */}
            <div className="bg-gray-800 rounded-lg p-4">
              <h3 className="text-lg font-semibold mb-4">Oscillator Waveform</h3>
              <div className="grid grid-cols-2 gap-2">
                {(['sine', 'square', 'saw', 'triangle'] as const).map((w) => (
                  <button
                    key={w}
                    onClick={() => handleWaveformChange(w)}
                    className={`px-4 py-2 rounded font-medium transition-colors ${
                      waveform === w 
                        ? 'bg-blue-600 text-white' 
                        : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
                    }`}
                  >
                    {w.charAt(0).toUpperCase() + w.slice(1)}
                  </button>
                ))}
              </div>
            </div>

            {/* Polyphony Mode */}
            <div className="bg-gray-800 rounded-lg p-4">
              <h3 className="text-lg font-semibold mb-4">Polyphony Mode</h3>
              <div className="grid grid-cols-3 gap-2">
                {(['poly', 'mono', 'legato'] as const).map((mode) => (
                  <button
                    key={mode}
                    onClick={() => handlePolyModeChange(mode)}
                    className={`px-3 py-2 rounded font-medium transition-colors ${
                      polyMode === mode 
                        ? 'bg-purple-600 text-white' 
                        : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
                    }`}
                  >
                    {mode.charAt(0).toUpperCase() + mode.slice(1)}
                  </button>
                ))}
              </div>
            </div>
          </div>

          {/* Right Column - Advanced Controls */}
          <div className="space-y-6">
            {/* ADSR Envelope */}
            <div className="bg-gray-800 rounded-lg p-4">
              <h3 className="text-lg font-semibold mb-4">ADSR Envelope</h3>
              <div className="space-y-3">
                {Object.entries(adsr).map(([param, value]) => (
                  <div key={param}>
                    <label className="block text-sm font-medium mb-1 capitalize">
                      {param}: {value.toFixed(3)}
                    </label>
                    <input
                      type="range"
                      min="0"
                      max={param === 'sustain' ? '1' : '2'}
                      step="0.01"
                      value={value}
                      onChange={(e) => handleAdsrChange(param as keyof typeof adsr, parseFloat(e.target.value))}
                      className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer"
                    />
                  </div>
                ))}
              </div>
            </div>

            {/* LFO Controls */}
            <div className="bg-gray-800 rounded-lg p-4">
              <h3 className="text-lg font-semibold mb-4">LFO Modulation</h3>
              <div className="space-y-3">
                <div>
                  <label className="block text-sm font-medium mb-1">
                    Waveform: {lfo.waveform}
                  </label>
                  <div className="grid grid-cols-2 gap-2">
                    {(['sine', 'square', 'saw', 'triangle'] as const).map((w) => (
                      <button
                        key={w}
                        onClick={() => handleLfoChange('waveform', w)}
                        className={`px-3 py-1 rounded text-sm font-medium transition-colors ${
                          lfo.waveform === w 
                            ? 'bg-green-600 text-white' 
                            : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
                        }`}
                      >
                        {w.charAt(0).toUpperCase() + w.slice(1)}
                      </button>
                    ))}
                  </div>
                </div>
                
                <div>
                  <label className="block text-sm font-medium mb-1">
                    Rate: {lfo.rate.toFixed(1)} Hz
                  </label>
                  <input
                    type="range"
                    min="0.1"
                    max="20"
                    step="0.1"
                    value={lfo.rate}
                    onChange={(e) => handleLfoChange('rate', parseFloat(e.target.value))}
                    className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer"
                  />
                </div>
                
                <div>
                  <label className="block text-sm font-medium mb-1">
                    Depth: {(lfo.depth * 100).toFixed(0)}%
                  </label>
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.01"
                    value={lfo.depth}
                    onChange={(e) => handleLfoChange('depth', parseFloat(e.target.value))}
                    className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer"
                  />
                </div>
              </div>
            </div>

            {/* Filter Controls */}
            <div className="bg-gray-800 rounded-lg p-4">
              <h3 className="text-lg font-semibold mb-4">Filter</h3>
              <div className="space-y-3">
                <div>
                  <label className="block text-sm font-medium mb-1">
                    Type: {filter.filter_type}
                  </label>
                  <div className="grid grid-cols-2 gap-2">
                    {(['lowpass', 'highpass', 'bandpass', 'notch'] as const).map((type) => (
                      <button
                        key={type}
                        onClick={() => handleFilterChange('filter_type', type)}
                        className={`px-3 py-1 rounded text-sm font-medium transition-colors ${
                          filter.filter_type === type 
                            ? 'bg-orange-600 text-white' 
                            : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
                        }`}
                      >
                        {type.charAt(0).toUpperCase() + type.slice(1)}
                      </button>
                    ))}
                  </div>
                </div>
                
                <div>
                  <label className="block text-sm font-medium mb-1">
                    Cutoff: {filter.cutoff.toFixed(0)} Hz
                  </label>
                  <input
                    type="range"
                    min="20"
                    max="20000"
                    step="10"
                    value={filter.cutoff}
                    onChange={(e) => handleFilterChange('cutoff', parseFloat(e.target.value))}
                    className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer"
                  />
                </div>
                
                <div>
                  <label className="block text-sm font-medium mb-1">
                    Resonance: {filter.resonance.toFixed(2)}
                  </label>
                  <input
                    type="range"
                    min="0.5"
                    max="20"
                    step="0.1"
                    value={filter.resonance}
                    onChange={(e) => handleFilterChange('resonance', parseFloat(e.target.value))}
                    className="w-full h-2 bg-gray-700 rounded-lg appearance-none cursor-pointer"
                  />
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Virtual Piano Keyboard */}
        <div className="bg-gray-800 rounded-lg p-6 mb-8">
          <h3 className="text-lg font-semibold mb-4">Virtual Piano (use computer keyboard)</h3>
          <div className="flex justify-center space-x-2">
            {pianoKeys.map(({ note, label, key }) => (
              <button
                key={note}
                onMouseDown={() => handleKeyDown(note)}
                onMouseUp={() => handleKeyUp(note)}
                className={`w-12 h-24 rounded-b-lg border-2 border-gray-600 transition-colors ${
                  isNoteActive(note)
                    ? 'bg-blue-500 text-white border-blue-400'
                    : 'bg-white text-black hover:bg-gray-200'
                }`}
              >
                <div className="text-xs font-medium">{label}</div>
                <div className="text-xs text-gray-500">({key})</div>
              </button>
            ))}
          </div>
          <div className="text-center mt-4 text-sm text-gray-400">
            Active Notes: {Array.from(activeNotes).map(n => getNoteName(n)).join(', ') || 'None'}
          </div>
        </div>

        {/* Status */}
        <div className="bg-gray-800 rounded-lg p-4">
          <h3 className="text-lg font-semibold mb-2">🎛️ Bridge Status</h3>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
            <div>
              <span className="text-gray-400">Engine:</span>
              <span className={`ml-2 ${isEngineReady ? 'text-green-400' : 'text-red-400'}`}>
                {isEngineReady ? '✅ Ready' : '❌ Offline'}
              </span>
            </div>
            <div>
              <span className="text-gray-400">Volume:</span>
              <span className="ml-2 text-blue-400">{(volume * 100).toFixed(0)}%</span>
            </div>
            <div>
              <span className="text-gray-400">Waveform:</span>
              <span className="ml-2 text-purple-400">{waveform}</span>
            </div>
            <div>
              <span className="text-gray-400">Polyphony:</span>
              <span className="ml-2 text-yellow-400">{polyMode}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}