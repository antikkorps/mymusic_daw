export type TrackType = "audio" | "midi";

export interface Track {
  id: string;
  name: string;
  color: string;
  volume: number; // -60 to +6 dB
  pan: number; // -1 (left) to 1 (right)
  solo: boolean;
  mute: boolean;
  type: TrackType;
  effects: Effect[];
  clips: Clip[];
  armed: boolean; // recording armed
  monitored: boolean;
  input?: string;
  output?: string;
}

export interface Effect {
  id: string;
  name: string;
  type: "filter" | "delay" | "reverb" | "compressor" | "eq";
  enabled: boolean;
  parameters: Record<string, number>;
  bypass?: boolean;
}

export interface Clip {
  id: string;
  trackId: string;
  startTime: number; // in seconds
  duration: number; // in seconds
  name: string;
  type: TrackType;
  color: string;
  offset?: number; // clip start offset in seconds
  gain?: number; // clip gain in dB
  fadeIn?: number; // fade in duration in seconds
  fadeOut?: number; // fade out duration in seconds
}

export interface VUMeterData {
  peak: number; // -60 to +6 dB
  rms: number; // -60 to +6 dB
  peakHold: number;
}

export interface TransportState {
  playing: boolean;
  recording: boolean;
  looping: boolean;
  tempo: number; // BPM
  timeSignature: {
    numerator: number;
    denominator: number;
  };
  position: number; // in seconds
  loopStart: number; // in seconds
  loopEnd: number; // in seconds
}

export interface AudioRoutingConnection {
  id: string;
  source: string;
  target: string;
  sourceHandle?: string;
  targetHandle?: string;
}
