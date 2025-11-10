import { motion, useDragControls, type PanInfo } from "framer-motion";
import { useState, useRef, memo, useEffect } from "react";
import { cn } from "~/lib/utils";
import { ContextMenu, type ContextMenuItem } from "~/components/ui/context-menu";
import type { Clip } from "~/types/audio";
import {
  Copy,
  Scissors,
  SplitSquareVertical,
  Trash2,
  Volume2,
  VolumeX,
} from "lucide-react"

interface DraggableClipProps {
  clip: Clip;
  zoom: number;
  isSelected: boolean;
  onSelect: (clipId: string, shiftKey: boolean) => void;
  onMove: (clipId: string, newStartTime: number, silent?: boolean) => void;
  onResize: (clipId: string, newDuration: number) => void;
  onDuplicate?: (clipId: string) => void;
  onDelete?: (clipId: string) => void;
  onSplit?: (clipId: string) => void;
}

export const DraggableClip = memo(function DraggableClip({
  clip,
  zoom,
  isSelected,
  onSelect,
  onMove,
  onResize,
  onDuplicate,
  onDelete,
  onSplit,
}: DraggableClipProps) {
  const [isDragging, setIsDragging] = useState(false);
  const [isResizing, setIsResizing] = useState<"left" | "right" | null>(null);
  const [dragStartTime, setDragStartTime] = useState(0);
  const dragControls = useDragControls();
  const resizeStartPosRef = useRef({ time: 0, duration: 0 });
  const finalSnappedPosRef = useRef({ time: 0, duration: 0 });
  const cleanupRef = useRef<(() => void) | null>(null);

  const handleDragStart = (event: MouseEvent | TouchEvent | PointerEvent) => {
    setIsDragging(true)
    setDragStartTime(clip.startTime)
    // Check if shift key is pressed during drag start
    const shiftKey = "shiftKey" in event && event.shiftKey
    onSelect(clip.id, shiftKey)
  }

  const handleDragEnd = (
    _event: MouseEvent | TouchEvent | PointerEvent,
    info: PanInfo
  ) => {
    setIsDragging(false)

    // Calculate final position on drag end
    const pixelOffset = info.offset.x
    const timeOffset = pixelOffset / zoom
    const newStartTime = Math.max(0, dragStartTime + timeOffset)

    // Snap to grid (0.25 second intervals)
    const snappedTime = Math.round(newStartTime * 4) / 4

    // Only call onMove once at the end if position changed
    if (snappedTime !== clip.startTime) {
      onMove(clip.id, snappedTime)
    }
  }

  // Cleanup event listeners on unmount
  useEffect(() => {
    return () => {
      if (cleanupRef.current) {
        cleanupRef.current();
      }
    };
  }, []);

  const handleResizeStart = (side: "left" | "right") => (e: React.MouseEvent) => {
    e.stopPropagation()
    setIsResizing(side)

    const startX = e.clientX
    const startTime = clip.startTime
    const startDuration = clip.duration

    // Store initial position for final notification
    resizeStartPosRef.current = { time: startTime, duration: startDuration };

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const deltaX = moveEvent.clientX - startX
      const deltaTime = deltaX / zoom

      if (side === "left") {
        // Resizing from the left: change both startTime and duration
        const newStartTime = Math.max(0, startTime + deltaTime)
        const snappedStart = Math.round(newStartTime * 4) / 4 // Snap to 0.25s grid
        const newDuration = Math.max(0.25, startDuration - (snappedStart - startTime))

        // Track the final snapped position for comparison in handleMouseUp
        finalSnappedPosRef.current = { time: snappedStart, duration: newDuration };

        // Silent move during resize (no notification)
        onMove(clip.id, snappedStart, true);
        onResize(clip.id, newDuration);
      } else {
        // Resizing from the right: change only duration
        const newDuration = Math.max(0.25, startDuration + deltaTime)
        const snappedDuration = Math.round(newDuration * 4) / 4 // Snap to 0.25s grid

        onResize(clip.id, snappedDuration)
      }
    }

    const handleMouseUp = () => {
      setIsResizing(null);
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);

      // Only show notification if position actually changed
      // Compare the final snapped position (tracked during mousemove) with initial position
      if (side === "left" && finalSnappedPosRef.current.time !== resizeStartPosRef.current.time) {
        // Trigger final move with notification using the tracked final position
        onMove(clip.id, finalSnappedPosRef.current.time, false);
      }
    };

    // Add event listeners
    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    // Store cleanup function in ref for unmount cleanup
    cleanupRef.current = () => {
      document.removeEventListener("mousemove", handleMouseMove)
      document.removeEventListener("mouseup", handleMouseUp)
    }
  }

  const contextMenuItems: ContextMenuItem[] = [
    {
      label: "Duplicate",
      icon: <Copy className="w-4 h-4" />,
      onClick: () => onDuplicate?.(clip.id),
      shortcut: "Ctrl+D",
    },
    {
      label: "Split at Playhead",
      icon: <Scissors className="w-4 h-4" />,
      onClick: () => onSplit?.(clip.id),
      shortcut: "S",
    },
    {
      separator: true,
    },
    {
      label: clip.gain && clip.gain < -40 ? "Unmute" : "Mute",
      icon:
        clip.gain && clip.gain < -40 ? (
          <Volume2 className="w-4 h-4" />
        ) : (
          <VolumeX className="w-4 h-4" />
        ),
      onClick: () => {
        // Toggle mute (mock)
      },
      shortcut: "M",
    },
    {
      label: "Normalize",
      icon: <SplitSquareVertical className="w-4 h-4" />,
      onClick: () => {
        // Normalize audio (mock)
      },
    },
    {
      separator: true,
    },
    {
      label: "Delete",
      icon: <Trash2 className="w-4 h-4" />,
      onClick: () => onDelete?.(clip.id),
      danger: true,
      shortcut: "Del",
    },
  ]

  return (
    <ContextMenu items={contextMenuItems}>
      <motion.div
        drag={isResizing ? false : "x"}
        dragControls={dragControls}
        dragMomentum={false}
        dragElastic={0}
        dragConstraints={{ left: -(clip.startTime * zoom), right: 10000 }}
        onDragStart={handleDragStart}
        onDragEnd={handleDragEnd}
        whileHover={{ scale: 1.02, zIndex: 10 }}
        whileTap={{ scale: 0.98 }}
        className={cn(
          "absolute top-2 bottom-2 rounded transition-all",
          isResizing ? "cursor-ew-resize" : "cursor-move",
          "hover:brightness-110",
          isSelected && "ring-2 ring-white shadow-lg",
          (isDragging || isResizing) && "opacity-75 z-50"
        )}
        style={{
          left: clip.startTime * zoom,
          width: clip.duration * zoom,
          backgroundColor: clip.color,
        }}
        onClick={(e) => {
          e.stopPropagation()
          onSelect(clip.id, e.shiftKey)
        }}
      >
        <div className="p-2 h-full flex flex-col justify-between pointer-events-none">
          <div className="text-xs font-medium text-white truncate">{clip.name}</div>
          <div className="flex justify-between items-end">
            <span className="text-xs text-white/70">
              {clip.type === "audio" ? "♪" : "⌨"}
            </span>
            <span className="text-xs text-white/70 font-mono">
              {clip.duration.toFixed(1)}s
            </span>
          </div>
        </div>

        {/* Fade indicators */}
        {clip.fadeIn && (
          <div
            className="absolute left-0 top-0 bottom-0 bg-black/20 pointer-events-none"
            style={{ width: `${(clip.fadeIn / clip.duration) * 100}%` }}
          />
        )}
        {clip.fadeOut && (
          <div
            className="absolute right-0 top-0 bottom-0 bg-black/20 pointer-events-none"
            style={{ width: `${(clip.fadeOut! / clip.duration) * 100}%` }}
          />
        )}

        {/* Resize handles */}
        <div
          className={cn(
            "absolute left-0 top-0 bottom-0 w-2 cursor-ew-resize hover:bg-white/30 transition-all pointer-events-auto z-10",
            isResizing === "left" && "bg-white/40"
          )}
          title="Resize left"
          onMouseDown={handleResizeStart("left")}
        />
        <div
          className={cn(
            "absolute right-0 top-0 bottom-0 w-2 cursor-ew-resize hover:bg-white/30 transition-all pointer-events-auto z-10",
            isResizing === "right" && "bg-white/40"
          )}
          title="Resize right"
          onMouseDown={handleResizeStart("right")}
        />
      </motion.div>
    </ContextMenu>
  )
})
