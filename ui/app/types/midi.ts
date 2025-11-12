export interface MidiNote {
  id: string;
  pitch: number; // 0-127 (MIDI note number)
  velocity: number; // 0-127
  startTime: number; // in beats or seconds
  duration: number; // in beats or seconds
  channel?: number; // 0-15
}

export interface MidiClip {
  id: string;
  notes: MidiNote[];
  startTime: number;
  duration: number;
}

export type GridSnap = "1/1" | "1/2" | "1/4" | "1/8" | "1/16" | "1/32" | "1/64";

export interface PianoRollState {
  zoom: number;
  scroll: {
    horizontal: number;
    vertical: number;
  };
  gridSnap: GridSnap;
  selectedNotes: string[];
  clipId?: string;
}

export const NOTE_NAMES = [
  "C",
  "C#",
  "D",
  "D#",
  "E",
  "F",
  "F#",
  "G",
  "G#",
  "A",
  "A#",
  "B",
];

// MIDI value range constants
export const MIDI_MIN_VALUE = 0;
export const MIDI_MAX_VALUE = 127;

// Minimum note duration in beats (1/32 note)
export const MIN_NOTE_DURATION = 0.03125;

export function midiNoteToName(midiNote: number): string {
  const octave = Math.floor(midiNote / 12) - 1;
  const noteName = NOTE_NAMES[midiNote % 12];
  return `${noteName}${octave}`;
}

export function noteNameToMidi(noteName: string): number {
  const match = noteName.match(/^([A-G]#?)(-?\d+)$/);
  if (!match) return 60; // default to C4

  const [, note, octaveStr] = match;
  const noteIndex = NOTE_NAMES.indexOf(note);
  const octave = parseInt(octaveStr, 10);

  return (octave + 1) * 12 + noteIndex;
}
