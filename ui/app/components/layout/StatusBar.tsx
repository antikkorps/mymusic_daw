import { useEffect, useState } from "react";
import { Cpu, HardDrive, Activity } from "lucide-react";
import { cn } from "~/lib/utils";

export function StatusBar() {
  const [cpuUsage, setCpuUsage] = useState(0);
  const [ramUsage, setRamUsage] = useState(0);

  // Simulate CPU/RAM usage updates
  useEffect(() => {
    const interval = setInterval(() => {
      setCpuUsage(15 + Math.random() * 25); // 15-40% CPU
      setRamUsage(45 + Math.random() * 15); // 45-60% RAM
    }, 2000);

    return () => clearInterval(interval);
  }, []);

  const getCpuColor = () => {
    if (cpuUsage > 80) return "text-red-500";
    if (cpuUsage > 60) return "text-yellow-500";
    return "text-green-500";
  };

  return (
    <footer className="h-8 bg-zinc-950 border-t border-zinc-800 flex items-center px-4 text-xs">
      <div className="flex items-center gap-6">
        {/* CPU Usage */}
        <div className="flex items-center gap-2">
          <Cpu className={cn("w-3.5 h-3.5", getCpuColor())} />
          <span className="text-zinc-400">CPU:</span>
          <span className={cn("font-mono font-medium", getCpuColor())}>
            {cpuUsage.toFixed(1)}%
          </span>
        </div>

        {/* RAM Usage */}
        <div className="flex items-center gap-2">
          <HardDrive className="w-3.5 h-3.5 text-cyan-500" />
          <span className="text-zinc-400">RAM:</span>
          <span className="font-mono font-medium text-cyan-500">
            {ramUsage.toFixed(1)}%
          </span>
        </div>

        {/* Latency */}
        <div className="flex items-center gap-2">
          <Activity className="w-3.5 h-3.5 text-blue-500" />
          <span className="text-zinc-400">Latency:</span>
          <span className="font-mono font-medium text-blue-500">5.8ms</span>
        </div>

        {/* Divider */}
        <div className="h-4 w-px bg-zinc-800" />

        {/* Sample Rate */}
        <div className="text-zinc-400">
          <span className="text-zinc-500">Sample Rate:</span>{" "}
          <span className="font-mono">48000 Hz</span>
        </div>

        {/* Buffer Size */}
        <div className="text-zinc-400">
          <span className="text-zinc-500">Buffer:</span>{" "}
          <span className="font-mono">256 samples</span>
        </div>
      </div>

      {/* Spacer */}
      <div className="flex-1" />

      {/* Right side info */}
      <div className="flex items-center gap-4 text-zinc-500">
        <div>Engine: CPAL</div>
        <div className="h-4 w-px bg-zinc-800" />
        <div>Backend: Rust</div>
      </div>
    </footer>
  );
}
