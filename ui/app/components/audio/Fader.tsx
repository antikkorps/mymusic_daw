import { useState, useRef, useEffect } from "react";
import { motion } from "framer-motion";
import { cn, clamp } from "~/lib/utils";

interface FaderProps {
  value: number; // dB value (-60 to +6)
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  className?: string;
  label?: string;
}

export function Fader({
  value,
  onChange,
  min = -60,
  max = 6,
  className,
  label,
}: FaderProps) {
  const [isDragging, setIsDragging] = useState(false);
  const [isHovered, setIsHovered] = useState(false);
  const faderRef = useRef<HTMLDivElement>(null);

  const percentage = ((value - min) / (max - min)) * 100;

  const updateValue = (clientY: number) => {
    if (!faderRef.current) return;

    const rect = faderRef.current.getBoundingClientRect();
    const y = clamp(clientY - rect.top, 0, rect.height);
    const percentage = 1 - y / rect.height;
    const newValue = min + percentage * (max - min);
    onChange(clamp(newValue, min, max));
  };

  const handleMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    setIsDragging(true);
    updateValue(e.clientY);
  };

  useEffect(() => {
    if (!isDragging) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (!faderRef.current) return;

      const rect = faderRef.current.getBoundingClientRect();
      const y = clamp(e.clientY - rect.top, 0, rect.height);
      const percentage = 1 - y / rect.height;
      const newValue = min + percentage * (max - min);
      onChange(clamp(newValue, min, max));
    };

    const handleMouseUp = () => {
      setIsDragging(false);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isDragging, min, max, onChange]);

  // Determine color based on dB value
  const getColor = () => {
    if (value > 0) return "bg-red-500";
    if (value > -6) return "bg-yellow-500";
    return "bg-cyan-500";
  };

  return (
    <div className={cn("flex flex-col items-center gap-2", className)}>
      <motion.div
        ref={faderRef}
        className="relative w-8 h-48 bg-zinc-900 rounded-sm border border-zinc-700 cursor-ns-resize select-none overflow-hidden"
        onMouseDown={handleMouseDown}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
        whileHover={{ scale: 1.05, borderColor: "#06b6d4" }}
        transition={{ duration: 0.15 }}
      >
        {/* 0dB mark */}
        <motion.div
          className="absolute left-0 right-0 h-px bg-zinc-600"
          style={{ bottom: `${((0 - min) / (max - min)) * 100}%` }}
          animate={{ opacity: isHovered ? 1 : 0.5 }}
        />

        {/* Fill */}
        <motion.div
          className={cn("absolute bottom-0 left-0 right-0", getColor())}
          initial={{ height: 0 }}
          animate={{ height: `${percentage}%` }}
          transition={{ type: "spring", stiffness: 300, damping: 30 }}
        />

        {/* Glow effect when hovering */}
        {isHovered && (
          <motion.div
            className={cn("absolute inset-0 pointer-events-none", getColor(), "opacity-20")}
            initial={{ opacity: 0 }}
            animate={{ opacity: 0.2 }}
            exit={{ opacity: 0 }}
          />
        )}

        {/* Thumb */}
        <motion.div
          className={cn(
            "absolute left-0 right-0 h-3 bg-zinc-200 border border-zinc-400 rounded-sm z-10",
            isDragging && "cursor-grabbing"
          )}
          style={{ bottom: `calc(${percentage}% - 6px)` }}
          animate={{
            scale: isDragging ? 1.2 : isHovered ? 1.1 : 1,
            boxShadow: isDragging
              ? "0 0 20px rgba(6, 182, 212, 0.6)"
              : isHovered
              ? "0 0 10px rgba(6, 182, 212, 0.3)"
              : "0 4px 6px rgba(0, 0, 0, 0.3)",
          }}
          transition={{ type: "spring", stiffness: 400, damping: 25 }}
        />
      </motion.div>

      {/* Value display */}
      <motion.div
        className="text-xs text-zinc-400 font-mono w-12 text-center"
        animate={{
          color: isDragging ? "#06b6d4" : "#a1a1aa",
          scale: isDragging ? 1.1 : 1,
        }}
        transition={{ duration: 0.15 }}
      >
        {value > -60 ? `${value.toFixed(1)}dB` : "-∞"}
      </motion.div>

      {label && (
        <div className="text-xs text-zinc-500 font-medium">{label}</div>
      )}
    </div>
  );
}
