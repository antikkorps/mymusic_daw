import * as React from "react";
import { cn } from "~/lib/utils";

interface SliderProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "onChange"> {
  value?: number;
  min?: number;
  max?: number;
  step?: number;
  onChange?: (value: number) => void;
  orientation?: "horizontal" | "vertical";
}

const Slider = React.forwardRef<HTMLInputElement, SliderProps>(
  ({ className, value = 0, min = 0, max = 100, step = 1, onChange, orientation = "horizontal", ...props }, ref) => {
    const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange?.(parseFloat(e.target.value));
    };

    return (
      <input
        type="range"
        ref={ref}
        value={value}
        min={min}
        max={max}
        step={step}
        onChange={handleChange}
        className={cn(
          "slider-thumb appearance-none bg-transparent cursor-pointer",
          orientation === "vertical" && "slider-vertical",
          className
        )}
        style={{
          writingMode: orientation === "vertical" ? "bt-lr" : undefined,
          WebkitAppearance: orientation === "vertical" ? "slider-vertical" : undefined,
        } as React.CSSProperties}
        {...props}
      />
    );
  }
);
Slider.displayName = "Slider";

export { Slider };
