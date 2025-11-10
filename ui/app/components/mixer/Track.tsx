import { memo } from "react";
import { Fader } from "~/components/audio/Fader";
import { Knob } from "~/components/audio/Knob";
import { VUMeter } from "~/components/audio/VUMeter";
import { Button } from "~/components/ui/button";
import { cn, panToString } from "~/lib/utils";
import { useToast } from "~/lib/toast";
import type { Track as TrackType } from "~/types/audio";
import { Circle } from "lucide-react";

// Available routing options
const INPUT_SOURCES = [
  "In 1-2",
  "In 3-4",
  "In 5-6",
  "In 7-8",
  "Bus A",
  "Bus B",
  "Bus C",
  "Bus D",
  "Sidechain",
];

const OUTPUT_DESTINATIONS = [
  "Master",
  "Bus A",
  "Bus B",
  "Bus C",
  "Bus D",
  "Out 1-2",
  "Out 3-4",
  "Out 5-6",
  "Out 7-8",
];

interface TrackProps {
  track: TrackType;
  onUpdate: (trackId: string, updates: Partial<TrackType>) => void;
  isMaster?: boolean;
}

export const Track = memo(function Track({ track, onUpdate, isMaster = false }: TrackProps) {
  const { showToast } = useToast();

  const handleVolumeChange = (volume: number) => {
    onUpdate(track.id, { volume });
  };

  const handlePanChange = (pan: number) => {
    onUpdate(track.id, { pan });
  };

  const handleInputChange = (input: string) => {
    onUpdate(track.id, { input });
    showToast(`${track.name} input: ${input}`, "success", 1500);
  };

  const handleOutputChange = (output: string) => {
    onUpdate(track.id, { output });
    showToast(`${track.name} output: ${output}`, "success", 1500);
  };

  const toggleSolo = () => {
    onUpdate(track.id, { solo: !track.solo });
  };

  const toggleMute = () => {
    onUpdate(track.id, { mute: !track.mute });
  };

  const toggleArmed = () => {
    if (!isMaster) {
      onUpdate(track.id, { armed: !track.armed });
    }
  };

  return (
    <div
      className={cn(
        "flex flex-col items-center gap-4 p-4 bg-zinc-900 rounded-lg border",
        isMaster ? "border-cyan-700 w-32" : "border-zinc-800 w-28"
      )}
      style={{
        borderTopColor: isMaster ? undefined : track.color,
        borderTopWidth: isMaster ? undefined : "3px",
      }}
    >
      {/* Track Name */}
      <div className={cn("text-sm font-medium text-center", isMaster ? "text-cyan-400" : "text-zinc-100")}>
        {track.name}
      </div>

      {/* Input/Output */}
      {!isMaster && (
        <div className="w-full space-y-1">
          <div className="text-[10px] text-zinc-600 uppercase font-semibold mb-0.5">Input</div>
          <select
            className="w-full text-xs bg-zinc-950 border border-zinc-700 rounded px-1 py-0.5 text-zinc-300 hover:border-zinc-600 focus:border-cyan-500 focus:outline-none transition-colors"
            value={track.input || "In 1-2"}
            onChange={(e) => handleInputChange(e.target.value)}
          >
            {INPUT_SOURCES.map((source) => (
              <option key={source} value={source}>
                {source}
              </option>
            ))}
          </select>

          <div className="text-[10px] text-zinc-600 uppercase font-semibold mb-0.5 mt-2">Output</div>
          <select
            className="w-full text-xs bg-zinc-950 border border-zinc-700 rounded px-1 py-0.5 text-zinc-300 hover:border-zinc-600 focus:border-cyan-500 focus:outline-none transition-colors"
            value={track.output || "Master"}
            onChange={(e) => handleOutputChange(e.target.value)}
          >
            {OUTPUT_DESTINATIONS.map((dest) => (
              <option key={dest} value={dest}>
                {dest}
              </option>
            ))}
          </select>
        </div>
      )}

      {/* Effects Slots */}
      <div className="w-full space-y-1">
        {track.effects.slice(0, 3).map((effect, idx) => (
          <div
            key={effect.id}
            className={cn(
              "text-xs px-2 py-1 rounded truncate",
              effect.enabled
                ? "bg-cyan-900/30 text-cyan-400 border border-cyan-700/50"
                : "bg-zinc-800 text-zinc-500 border border-zinc-700"
            )}
            title={effect.name}
          >
            {effect.name}
          </div>
        ))}
        {Array.from({ length: Math.max(0, 3 - track.effects.length) }).map((_, idx) => (
          <div
            key={`empty-${idx}`}
            className="text-xs px-2 py-1 rounded bg-zinc-950 border border-zinc-800 text-zinc-600"
          >
            Empty
          </div>
        ))}
      </div>

      {/* Pan Knob */}
      <Knob
        value={track.pan}
        onChange={handlePanChange}
        min={-1}
        max={1}
        size="sm"
        label="Pan"
        valueDisplay={panToString}
      />

      {/* VU Meter */}
      <VUMeter trackId={track.id} className="h-32" />

      {/* Fader */}
      <Fader
        value={track.volume}
        onChange={handleVolumeChange}
        min={-60}
        max={6}
      />

      {/* Solo/Mute/Arm Buttons */}
      <div className="flex flex-col gap-1 w-full">
        {!isMaster && (
          <Button
            size="sm"
            variant={track.armed ? "destructive" : "outline"}
            className="w-full h-7 text-xs"
            onClick={toggleArmed}
          >
            <Circle className="w-3 h-3 mr-1" fill={track.armed ? "currentColor" : "none"} />
            R
          </Button>
        )}

        <Button
          size="sm"
          variant={track.solo ? "secondary" : "outline"}
          className="w-full h-7 text-xs"
          onClick={toggleSolo}
        >
          S
        </Button>

        <Button
          size="sm"
          variant={track.mute ? "secondary" : "outline"}
          className={cn("w-full h-7 text-xs", track.mute && "bg-red-900 hover:bg-red-800")}
          onClick={toggleMute}
        >
          M
        </Button>
      </div>

      {/* Track Type */}
      <div className="text-xs text-zinc-600 uppercase">
        {isMaster ? "Master" : track.type}
      </div>
    </div>
  );
});
