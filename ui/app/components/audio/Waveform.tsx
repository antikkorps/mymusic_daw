import { useEffect, useRef, memo } from "react";
import { cn } from "~/lib/utils";

interface WaveformProps {
  className?: string;
  color?: string;
  animate?: boolean;
}

function WaveformComponent({ className, color = "#06b6d4", animate: shouldAnimate = true }: WaveformProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animationRef = useRef<number | undefined>(undefined);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    // Set canvas size
    const resizeCanvas = () => {
      const dpr = window.devicePixelRatio || 1;
      const rect = canvas.getBoundingClientRect();
      canvas.width = rect.width * dpr;
      canvas.height = rect.height * dpr;
      ctx.setTransform(1, 0, 0, 1, 0, 0); // Reset transform before scaling
      ctx.scale(dpr, dpr);
      canvas.style.width = `${rect.width}px`;
      canvas.style.height = `${rect.height}px`;
    };

    resizeCanvas();
    window.addEventListener("resize", resizeCanvas);

    // Generate waveform data
    const generateWaveform = (time: number) => {
      const rect = canvas.getBoundingClientRect();
      const width = rect.width;
      const height = rect.height;
      const centerY = height / 2;

      ctx.clearRect(0, 0, width, height);

      // Draw waveform
      ctx.beginPath();
      ctx.strokeStyle = color;
      ctx.lineWidth = 2;

      const samples = 200;
      const step = width / samples;

      for (let i = 0; i < samples; i++) {
        const x = i * step;

        // Generate audio-like waveform with multiple harmonics
        const t = (x / width + time) * Math.PI * 2;
        let y = 0;
        y += Math.sin(t * 2) * 0.3;
        y += Math.sin(t * 4) * 0.15;
        y += Math.sin(t * 8) * 0.08;
        y += Math.sin(t * 16) * 0.04;

        // Add some randomness
        y += (Math.random() - 0.5) * 0.1;

        const amplitude = centerY * 0.6;
        const yPos = centerY + y * amplitude;

        if (i === 0) {
          ctx.moveTo(x, yPos);
        } else {
          ctx.lineTo(x, yPos);
        }
      }

      ctx.stroke();

      // Draw center line
      ctx.beginPath();
      ctx.strokeStyle = "rgba(255, 255, 255, 0.1)";
      ctx.lineWidth = 1;
      ctx.moveTo(0, centerY);
      ctx.lineTo(width, centerY);
      ctx.stroke();
    };

    // Animation loop
    let time = 0;
    const animateLoop = () => {
      time += 0.005;
      generateWaveform(time);
      animationRef.current = requestAnimationFrame(animateLoop);
    };

    if (shouldAnimate) {
      animateLoop();
    } else {
      generateWaveform(0);
    }

    return () => {
      window.removeEventListener("resize", resizeCanvas);
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
    };
  }, [color, shouldAnimate]);

  return (
    <canvas
      ref={canvasRef}
      className={cn("w-full h-full", className)}
    />
  );
}

// Custom comparison function to prevent unnecessary re-renders
// Only re-render if color or animate props actually change
function arePropsEqual(prevProps: WaveformProps, nextProps: WaveformProps): boolean {
  return (
    prevProps.className === nextProps.className &&
    prevProps.color === nextProps.color &&
    prevProps.animate === nextProps.animate
  );
}

export const Waveform = memo(WaveformComponent, arePropsEqual);
