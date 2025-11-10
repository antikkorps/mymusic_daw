import { useState, useRef, useEffect } from "react";
import { motion } from "framer-motion";
import { cn, clamp } from "~/lib/utils";

interface KnobProps {
  value: number; // normalized 0-1 or custom range
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  size?: "sm" | "md" | "lg";
  label?: string;
  valueDisplay?: (value: number) => string;
  className?: string;
}

export function Knob({
  value,
  onChange,
  min = 0,
  max = 1,
  size = "md",
  label,
  valueDisplay,
  className,
}: KnobProps) {
  const [isDragging, setIsDragging] = useState(false);
  const [isHovered, setIsHovered] = useState(false);
  const [startY, setStartY] = useState(0);
  const [startValue, setStartValue] = useState(value);

  const sizeClasses = {
    sm: "w-12 h-12",
    md: "w-16 h-16",
    lg: "w-20 h-20",
  };

  // Convert value to angle (-135deg to +135deg)
  const normalized = (value - min) / (max - min);
  const angle = -135 + normalized * 270;

  const handleMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    setIsDragging(true);
    setStartY(e.clientY);
    setStartValue(value);
  };

  useEffect(() => {
    if (!isDragging) return;

    const handleMouseMove = (e: MouseEvent) => {
      const deltaY = startY - e.clientY;
      const sensitivity = 0.005;
      const delta = deltaY * sensitivity * (max - min);
      const newValue = clamp(startValue + delta, min, max);
      onChange(newValue);
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
  }, [isDragging, startY, startValue, min, max, onChange]);

  const displayValue = valueDisplay ? valueDisplay(value) : value.toFixed(2);

  return (
    <div className={cn("flex flex-col items-center gap-2", className)}>
      <motion.div
        className={cn(
          "relative rounded-full bg-zinc-900 border-2 cursor-ns-resize select-none",
          sizeClasses[size]
        )}
        onMouseDown={handleMouseDown}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
        animate={{
          scale: isDragging ? 1.15 : isHovered ? 1.08 : 1,
          borderColor: isDragging ? "#06b6d4" : isHovered ? "#3f3f46" : "#3f3f46",
          boxShadow: isDragging
            ? "0 0 25px rgba(6, 182, 212, 0.5)"
            : isHovered
            ? "0 0 15px rgba(6, 182, 212, 0.2)"
            : "0 4px 6px rgba(0, 0, 0, 0.3)",
        }}
        transition={{ type: "spring", stiffness: 400, damping: 25 }}
      >
        {/* Tick marks */}
        <svg className="absolute inset-0 w-full h-full pointer-events-none" viewBox="0 0 100 100">
          {/* Background arc */}
          <motion.path
            d="M 15,85 A 40 40 0 1 1 85,85"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            className="text-zinc-800"
            animate={{ opacity: isHovered ? 1 : 0.5 }}
          />

          {/* Value arc */}
          <motion.path
            d="M 15,85 A 40 40 0 1 1 85,85"
            fill="none"
            stroke="currentColor"
            strokeWidth="3"
            strokeDasharray={`${normalized * 188.5} 188.5`}
            strokeLinecap="round"
            className="text-cyan-500"
            animate={{
              strokeWidth: isDragging ? 4 : 3,
              filter: isDragging
                ? "drop-shadow(0 0 6px rgba(6, 182, 212, 0.8))"
                : "none",
            }}
            transition={{ duration: 0.15 }}
          />

          {/* Indicator line */}
          <motion.line
            x1="50"
            y1="50"
            x2="50"
            y2="20"
            stroke="currentColor"
            strokeWidth="3"
            strokeLinecap="round"
            className="text-zinc-200"
            animate={{
              strokeWidth: isDragging ? 4 : 3,
            }}
            transition={{ type: "spring", stiffness: 300, damping: 30 }}
            transform={`rotate(${angle} 50 50)`}
          />

          {/* Center dot */}
          <motion.circle
            cx="50"
            cy="50"
            r="4"
            fill="currentColor"
            className="text-zinc-200"
            animate={{
              r: isDragging ? 6 : isHovered ? 5 : 4,
              filter: isDragging
                ? "drop-shadow(0 0 4px rgba(255, 255, 255, 0.8))"
                : "none",
            }}
            transition={{ type: "spring", stiffness: 400, damping: 20 }}
          />
        </svg>
      </motion.div>

      {/* Value display */}
      <motion.div
        className="text-xs font-mono"
        animate={{
          color: isDragging ? "#06b6d4" : "#a1a1aa",
          scale: isDragging ? 1.1 : 1,
          fontWeight: isDragging ? 600 : 400,
        }}
        transition={{ duration: 0.15 }}
      >
        {displayValue}
      </motion.div>

      {label && (
        <div className="text-xs text-zinc-500 font-medium">{label}</div>
      )}
    </div>
  );
}
