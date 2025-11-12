import type { Track, Effect, Clip, TransportState } from "~/types/audio";
import type { Plugin, PluginInstance } from "~/types/plugin";
import type { MidiNote } from "~/types/midi";

// Track colors
const TRACK_COLORS = [
  "#ef4444", // red
  "#f59e0b", // amber
  "#10b981", // emerald
  "#3b82f6", // blue
  "#8b5cf6", // violet
  "#ec4899", // pink
  "#06b6d4", // cyan
  "#f97316", // orange
];

// Mock Effects
export const mockEffects: Effect[] = [
  {
    id: "fx-filter-1",
    name: "SVF Filter",
    type: "filter",
    enabled: true,
    parameters: {
      cutoff: 1000,
      resonance: 0.7,
      type: 0, // 0=LP, 1=HP, 2=BP
    },
  },
  {
    id: "fx-delay-1",
    name: "Stereo Delay",
    type: "delay",
    enabled: true,
    parameters: {
      time: 0.375,
      feedback: 0.4,
      mix: 0.3,
    },
  },
  {
    id: "fx-reverb-1",
    name: "Plate Reverb",
    type: "reverb",
    enabled: true,
    parameters: {
      roomSize: 0.7,
      damping: 0.5,
      mix: 0.25,
    },
  },
];

// Mock Clips
export const mockClips: Clip[] = [
  {
    id: "clip-1",
    trackId: "track-1",
    startTime: 0,
    duration: 8,
    name: "Drums Loop 1",
    type: "audio",
    color: TRACK_COLORS[0],
    gain: 0,
    fadeIn: 0.1,
    fadeOut: 0.1,
  },
  {
    id: "clip-2",
    trackId: "track-1",
    startTime: 8,
    duration: 8,
    name: "Drums Loop 2",
    type: "audio",
    color: TRACK_COLORS[0],
    gain: 0,
  },
  {
    id: "clip-3",
    trackId: "track-2",
    startTime: 4,
    duration: 12,
    name: "Bass Line",
    type: "audio",
    color: TRACK_COLORS[1],
    gain: -3,
  },
  {
    id: "clip-4",
    trackId: "track-3",
    startTime: 0,
    duration: 16,
    name: "Piano Chords",
    type: "midi",
    color: TRACK_COLORS[2],
  },
  {
    id: "clip-5",
    trackId: "track-4",
    startTime: 8,
    duration: 8,
    name: "Lead Synth",
    type: "midi",
    color: TRACK_COLORS[3],
  },
  {
    id: "clip-6",
    trackId: "track-5",
    startTime: 0,
    duration: 16,
    name: "Pad",
    type: "audio",
    color: TRACK_COLORS[4],
    gain: -6,
  },
];

// Mock Tracks
export const mockTracks: Track[] = [
  {
    id: "track-1",
    name: "Drums",
    color: TRACK_COLORS[0],
    volume: -6,
    pan: 0,
    solo: false,
    mute: false,
    type: "audio",
    effects: [mockEffects[0]],
    clips: mockClips.filter((c) => c.trackId === "track-1"),
    armed: false,
    monitored: false,
    input: "Audio Input 1-2",
    output: "Master",
  },
  {
    id: "track-2",
    name: "Bass",
    color: TRACK_COLORS[1],
    volume: -8,
    pan: 0,
    solo: false,
    mute: false,
    type: "audio",
    effects: [],
    clips: mockClips.filter((c) => c.trackId === "track-2"),
    armed: false,
    monitored: false,
    input: "Audio Input 3-4",
    output: "Master",
  },
  {
    id: "track-3",
    name: "Piano",
    color: TRACK_COLORS[2],
    volume: -10,
    pan: -0.3,
    solo: false,
    mute: false,
    type: "midi",
    effects: [mockEffects[2]],
    clips: mockClips.filter((c) => c.trackId === "track-3"),
    armed: false,
    monitored: false,
    input: "MIDI Input 1",
    output: "Master",
  },
  {
    id: "track-4",
    name: "Lead",
    color: TRACK_COLORS[3],
    volume: -12,
    pan: 0.2,
    solo: false,
    mute: false,
    type: "midi",
    effects: [mockEffects[1]],
    clips: mockClips.filter((c) => c.trackId === "track-4"),
    armed: false,
    monitored: false,
    input: "MIDI Input 2",
    output: "Master",
  },
  {
    id: "track-5",
    name: "Pad",
    color: TRACK_COLORS[4],
    volume: -15,
    pan: 0,
    solo: false,
    mute: false,
    type: "audio",
    effects: [mockEffects[2]],
    clips: mockClips.filter((c) => c.trackId === "track-5"),
    armed: false,
    monitored: false,
    input: "Audio Input 5-6",
    output: "Master",
  },
  {
    id: "track-6",
    name: "Vocal",
    color: TRACK_COLORS[5],
    volume: -9,
    pan: 0,
    solo: false,
    mute: false,
    type: "audio",
    effects: [],
    clips: [],
    armed: true,
    monitored: true,
    input: "Audio Input 7-8",
    output: "Master",
  },
  {
    id: "track-7",
    name: "FX",
    color: TRACK_COLORS[6],
    volume: -18,
    pan: 0,
    solo: false,
    mute: false,
    type: "audio",
    effects: [mockEffects[1], mockEffects[2]],
    clips: [],
    armed: false,
    monitored: false,
    input: "Audio Input 9-10",
    output: "Master",
  },
  {
    id: "track-8",
    name: "Percussion",
    color: TRACK_COLORS[7],
    volume: -7,
    pan: 0.4,
    solo: false,
    mute: false,
    type: "audio",
    effects: [],
    clips: [],
    armed: false,
    monitored: false,
    input: "Audio Input 11-12",
    output: "Master",
  },
];

// Mock Master Track
export const mockMasterTrack: Track = {
  id: "master",
  name: "Master",
  color: "#ffffff",
  volume: 0,
  pan: 0,
  solo: false,
  mute: false,
  type: "audio",
  effects: [],
  clips: [],
  armed: false,
  monitored: false,
  output: "System Output",
};

// Mock Plugins
export const mockPlugins: Plugin[] = [
  {
    id: "plugin-1",
    name: "TAL-NoiseMaker",
    vendor: "Togu Audio Line",
    version: "4.5.3",
    format: "CLAP",
    category: "Instrument",
    path: "/usr/lib/clap/TAL-NoiseMaker.clap",
    description: "Improved version of TAL-Elek7ro and has a completely new synth engine",
    features: ["Stereo", "Poly", "MIDI"],
    loaded: false,
  },
  {
    id: "plugin-2",
    name: "Dexed",
    vendor: "Digital Suburban",
    version: "0.9.6",
    format: "CLAP",
    category: "Instrument",
    path: "/usr/lib/clap/Dexed.clap",
    description: "FM synthesizer closely modeled on the Yamaha DX7",
    features: ["Stereo", "Poly", "MIDI"],
    loaded: true,
    instanceId: "inst-1",
  },
  {
    id: "plugin-3",
    name: "Valhalla FreqEcho",
    vendor: "Valhalla DSP",
    version: "1.2.1",
    format: "CLAP",
    category: "Delay",
    path: "/usr/lib/clap/ValhallaFreqEcho.clap",
    description: "Frequency shifter + analog echo emulation",
    features: ["Stereo", "Effect"],
    loaded: false,
  },
  {
    id: "plugin-4",
    name: "Surge XT",
    vendor: "Surge Synth Team",
    version: "1.3.2",
    format: "CLAP",
    category: "Instrument",
    path: "/usr/lib/clap/Surge-XT.clap",
    description: "Hybrid synthesizer featuring many synthesis techniques",
    features: ["Stereo", "Poly", "MIDI", "MPE"],
    loaded: false,
  },
  {
    id: "plugin-5",
    name: "LSP Multiband Compressor",
    vendor: "LSP",
    version: "1.2.8",
    format: "CLAP",
    category: "Dynamics",
    path: "/usr/lib/clap/lsp-mb-compressor.clap",
    description: "Multiband dynamics processor with side-chain",
    features: ["Stereo", "Effect"],
    loaded: true,
    instanceId: "inst-2",
  },
  {
    id: "plugin-6",
    name: "Dragonfly Plate Reverb",
    vendor: "Dragonfly Reverb",
    version: "3.2.10",
    format: "CLAP",
    category: "Reverb",
    path: "/usr/lib/clap/DragonflyPlateReverb.clap",
    description: "Plate reverb based on Jon Dattorro's algorithm",
    features: ["Stereo", "Effect"],
    loaded: false,
  },
  {
    id: "plugin-7",
    name: "Vital",
    vendor: "Matt Tytel",
    version: "1.5.5",
    format: "CLAP",
    category: "Instrument",
    path: "/usr/lib/clap/Vital.clap",
    description: "Spectral warping wavetable synthesizer",
    features: ["Stereo", "Poly", "MIDI", "MPE"],
    loaded: false,
  },
  {
    id: "plugin-8",
    name: "Chow Tape Model",
    vendor: "Chowdhury DSP",
    version: "2.11.0",
    format: "CLAP",
    category: "Distortion",
    path: "/usr/lib/clap/ChowTapeModel.clap",
    description: "Physical modeling analog tape machine",
    features: ["Stereo", "Effect"],
    loaded: false,
  },
];

// Mock Transport State
export const mockTransportState: TransportState = {
  playing: false,
  recording: false,
  looping: false,
  tempo: 120,
  timeSignature: {
    numerator: 4,
    denominator: 4,
  },
  position: 0,
  loopStart: 0,
  loopEnd: 16,
};

// Mock MIDI Notes for Piano Roll
export const mockMidiNotes: MidiNote[] = [
  { id: "note-1", pitch: 60, velocity: 100, startTime: 0, duration: 1, channel: 0 },
  { id: "note-2", pitch: 64, velocity: 95, startTime: 1, duration: 1, channel: 0 },
  { id: "note-3", pitch: 67, velocity: 90, startTime: 2, duration: 1, channel: 0 },
  { id: "note-4", pitch: 72, velocity: 105, startTime: 3, duration: 1, channel: 0 },
  { id: "note-5", pitch: 60, velocity: 85, startTime: 4, duration: 2, channel: 0 },
  { id: "note-6", pitch: 67, velocity: 92, startTime: 6, duration: 1, channel: 0 },
  { id: "note-7", pitch: 64, velocity: 88, startTime: 7, duration: 1, channel: 0 },
  { id: "note-8", pitch: 60, velocity: 100, startTime: 8, duration: 4, channel: 0 },
];

// Generate mock VU meter data
export function generateVUMeterData() {
  const peak = -60 + Math.random() * 50;
  return {
    peak,
    rms: peak - 3 - Math.random() * 6,
    peakHold: peak + Math.random() * 3,
  };
}
