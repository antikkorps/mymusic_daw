import { Play, Pause, Square, Circle, Repeat } from "lucide-react";
import { Button } from "~/components/ui/button";
import { cn } from "~/lib/utils";
import type { TransportState } from "~/types/audio";

interface ToolbarProps {
  transport: TransportState;
  onTransportChange: (transport: Partial<TransportState>) => void;
}

export function Toolbar({ transport, onTransportChange }: ToolbarProps) {
  const togglePlay = () => {
    onTransportChange({ playing: !transport.playing });
  };

  const toggleRecord = () => {
    onTransportChange({ recording: !transport.recording });
  };

  const stop = () => {
    onTransportChange({ playing: false, recording: false, position: 0 });
  };

  const toggleLoop = () => {
    onTransportChange({ looping: !transport.looping });
  };

  return (
    <header className="h-16 bg-zinc-950 border-b border-zinc-800 flex items-center px-6 gap-8">
      {/* Transport Controls */}
      <div className="flex items-center gap-2">
        <Button
          size="icon"
          variant={transport.recording ? "destructive" : "outline"}
          onClick={toggleRecord}
          className={cn(transport.recording && "animate-pulse")}
        >
          <Circle className="w-4 h-4" fill={transport.recording ? "currentColor" : "none"} />
        </Button>

        <Button
          size="icon"
          variant={transport.playing ? "secondary" : "outline"}
          onClick={togglePlay}
        >
          {transport.playing ? (
            <Pause className="w-4 h-4" />
          ) : (
            <Play className="w-4 h-4" />
          )}
        </Button>

        <Button size="icon" variant="outline" onClick={stop}>
          <Square className="w-4 h-4" />
        </Button>

        <Button
          size="icon"
          variant={transport.looping ? "secondary" : "outline"}
          onClick={toggleLoop}
        >
          <Repeat className="w-4 h-4" />
        </Button>
      </div>

      {/* Divider */}
      <div className="h-8 w-px bg-zinc-800" />

      {/* Position Display */}
      <div className="flex items-center gap-4">
        <div className="text-sm font-mono text-zinc-100 bg-zinc-900 px-3 py-1.5 rounded border border-zinc-800">
          {formatTime(transport.position)}
        </div>

        <div className="text-xs text-zinc-500">
          Bar {Math.floor(transport.position / (4 * 60 / transport.tempo)) + 1}
        </div>
      </div>

      {/* Divider */}
      <div className="h-8 w-px bg-zinc-800" />

      {/* Tempo */}
      <div className="flex items-center gap-2">
        <label className="text-xs text-zinc-500 font-medium">BPM</label>
        <input
          type="number"
          value={transport.tempo}
          onChange={(e) =>
            onTransportChange({ tempo: parseInt(e.target.value) || 120 })
          }
          className="w-16 bg-zinc-900 text-zinc-100 text-sm font-mono px-2 py-1 rounded border border-zinc-800 focus:outline-none focus:border-cyan-500"
          min={60}
          max={240}
        />
      </div>

      {/* Time Signature */}
      <div className="flex items-center gap-2">
        <label className="text-xs text-zinc-500 font-medium">Time</label>
        <div className="bg-zinc-900 text-zinc-100 text-sm font-mono px-2 py-1 rounded border border-zinc-800">
          {transport.timeSignature.numerator}/{transport.timeSignature.denominator}
        </div>
      </div>

      {/* Spacer */}
      <div className="flex-1" />

      {/* Right side info */}
      <div className="flex items-center gap-4 text-xs text-zinc-500">
        <div className="flex items-center gap-2">
          <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
          <span>48kHz / 24bit</span>
        </div>
      </div>
    </header>
  );
}

function formatTime(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  const ms = Math.floor((seconds % 1) * 100);
  return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}.${ms.toString().padStart(2, "0")}`;
}
