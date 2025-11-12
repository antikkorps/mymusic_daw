export type PluginFormat = "CLAP" | "VST3" | "AU";

export type PluginCategory =
  | "Instrument"
  | "Effect"
  | "Analyzer"
  | "Utility"
  | "Dynamics"
  | "EQ"
  | "Filter"
  | "Modulation"
  | "Reverb"
  | "Delay"
  | "Distortion"
  | "Other";

export interface Plugin {
  id: string;
  name: string;
  vendor: string;
  version: string;
  format: PluginFormat;
  category: PluginCategory;
  path: string;
  description?: string;
  features?: string[];
  loaded: boolean;
  instanceId?: string;
}

export interface PluginParameter {
  id: string;
  name: string;
  value: number;
  min: number;
  max: number;
  default: number;
  unit?: string;
  displayValue?: string;
  automation?: boolean;
}

export interface PluginInstance {
  id: string;
  pluginId: string;
  trackId: string;
  slotIndex: number;
  enabled: boolean;
  parameters: PluginParameter[];
  presetName?: string;
}

export interface PluginPreset {
  name: string;
  parameters: Record<string, number>;
}
